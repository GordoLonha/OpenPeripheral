//! Device engine: one thread owns every HID handle (they are not Sync), renders
//! lighting at ~30 fps, polls the headset, reconnects devices and reports state.

use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use hidapi::HidApi;
use op_core::{
    enumerate,
    hs80::{self, BatteryStatus, Event, Hs80},
    k70::{self, K70},
    m65::{self, M65},
    DeviceKind,
};
use serde::Serialize;

use std::collections::HashMap;

use crate::battery_icon;
use crate::mouse::MouseOptions;
use crate::effects::{self, Geometry, Heat, Lighting};
use crate::remap::{Action, Injector};

const FRAME: Duration = Duration::from_millis(33);
const RESCAN: Duration = Duration::from_secs(3);
/// Charge state is cheap to read; poll often so plugging the cable shows up quickly.
const BATTERY_POLL: Duration = Duration::from_secs(5);
const ICUE_CHECK: Duration = Duration::from_secs(5);
/// Headset LED frames: at most this often, and at least every HS_KEEPALIVE.
const HS_FRAME: Duration = Duration::from_millis(100);
const HS_KEEPALIVE: Duration = Duration::from_millis(500);
/// A wheel press shorter than this is a click (Corsair: 0.1-0.5 s toggles EQ).
const WHEEL_CLICK: Duration = Duration::from_millis(800);
const WHEEL_FLASH: Duration = Duration::from_millis(350);

pub enum Cmd {
    SetLighting(Lighting),
    SetGeometry(Geometry),
    /// LED slot (= key index) -> action.
    SetBindings(HashMap<u8, Action>),
    /// Win Lock on/off (Windows keys blocked, lock LED lit).
    SetWinLock(bool),
    /// DPI stage (0-based): restored on connect, selected now if connected.
    SetMouseStage(u8),
    SetMouse(MouseOptions),
    /// Mouse button (bit, see m65::BUTTONS) -> action.
    SetMouseBindings(HashMap<u8, Action>),
    SetKeyboard(KeyboardOptions),
    SetHeadset(HeadsetOptions),
    Shutdown(Sender<()>),
}

/// K70 settings kept by the firmware (they survive the app closing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyboardOptions {
    /// Under Win Lock, also block these combinations.
    pub alt_tab: bool,
    pub alt_f4: bool,
    pub shift_tab: bool,
    /// Report interval: 1, 2, 4 or 8 ms.
    pub polling_ms: u8,
}

impl Default for KeyboardOptions {
    fn default() -> Self {
        Self { alt_tab: false, alt_f4: false, shift_tab: false, polling_ms: 1 }
    }
}

impl KeyboardOptions {
    fn lock(&self, on: bool) -> k70::Lock {
        k70::Lock {
            windows: on,
            alt_tab: on && self.alt_tab,
            alt_f4: on && self.alt_f4,
            shift_tab: on && self.shift_tab,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeadsetOptions {
    /// Auto power-off after this many idle minutes (0 = never); None = leave as is.
    pub sleep_min: Option<u16>,
    /// Mirror the boom mute onto the system's default microphone.
    pub mic_sync: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Status {
    pub keyboard: Option<KeyboardInfo>,
    pub headset: Option<HeadsetInfo>,
    /// iCUE fights over the devices; the UI warns while it runs.
    pub icue_running: bool,
    /// Windows keys blocked by the Win Lock key.
    pub win_lock: bool,
    pub mouse: Option<MouseInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MouseInfo {
    pub name: String,
    pub firmware: String,
    /// Active DPI stage, 0-based (followed from the DPI buttons).
    pub dpi_stage: u8,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KeyboardInfo {
    pub name: String,
    pub firmware: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HeadsetInfo {
    pub name: String,
    /// None while the headset is off / out of range (receiver still plugged).
    pub battery: Option<f32>,
    /// USB cable plugged in (property 0x10 = 1).
    pub charging: bool,
    pub firmware: Option<String>,
    /// Boom flipped up. None until the headset answers.
    pub mic_muted: Option<bool>,
}

/// Software mode: the headset then reports wheel clicks, and we draw its LEDs
/// (it keeps the last frame, so the status LED is ours to keep up to date).
/// Needed again whenever the headset powers back on. Settings writes are only
/// accepted in software mode (status 09 otherwise), so they go here too.
fn take_headset(hs: &Hs80, opts: &HeadsetOptions) {
    if hs.set_software_mode(true).is_ok() {
        let _ = hs.open_lighting();
        if let Some(m) = opts.sleep_min {
            let _ = hs.set_sleep(m);
        }
    }
}

/// Status LED like the headset's own: battery colour, blinking while charging.
fn headset_frame(lighting: &Lighting, t: f32, info: Option<&HeadsetInfo>, flash: bool) -> [u8; hs80::LED_BYTES] {
    let white = op_core::Rgb::new(255, 255, 255);
    let mut leds = [op_core::Rgb::default(); hs80::LED_COUNT];
    leds[0] = if flash { white } else { effects::headset(lighting, t) };
    if let Some(p) = info.and_then(|i| i.battery) {
        let blink_off = info.is_some_and(|i| i.charging) && (t * 2.0) as u32 % 2 == 1;
        let [r, g, b] = battery_icon::color(battery_icon::State::Discharging(p));
        leds[hs80::LED_STATUS] = if flash {
            white
        } else if blink_off {
            op_core::Rgb::default()
        } else {
            op_core::Rgb::new(r, g, b)
        };
    }
    if info.and_then(|i| i.mic_muted) == Some(true) {
        leds[hs80::LED_MIC] = op_core::Rgb::new(255, 0, 0);
    }
    hs80::led_frame(&leds)
}

fn read_headset(hs: &Hs80) -> (Option<f32>, bool) {
    let charging = matches!(hs.battery_status(), Ok(BatteryStatus::Charging));
    (hs.battery_percent().ok(), charging)
}

/// `on_key` gets the LED slot (= key index) of every key press, for "record key" in the UI.
pub fn spawn(
    lighting: Lighting,
    on_status: impl Fn(&Status) + Send + 'static,
    on_key: impl Fn(u8) + Send + 'static,
    on_profile: impl Fn(String) + Send + 'static,
    on_wheel_click: impl Fn() + Send + 'static,
    on_brightness_key: impl Fn() + Send + 'static,
) -> Sender<Cmd> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("device-engine".into())
        .spawn(move || run(rx, lighting, on_status, on_key, on_profile, on_wheel_click, on_brightness_key))
        .expect("spawn engine");
    tx
}

struct Devices {
    api: HidApi,
    k70: Option<K70>,
    hs80: Option<Hs80>,
    m65: Option<M65>,
    /// DPI stage to select on connect (kept in settings by the app).
    mouse_stage: u8,
    mouse_opts: MouseOptions,
    /// Buttons put in host-only mode on connect (remapped ones).
    mouse_host_only: u8,
    /// Polling rate last sent to the mouse (it re-enumerates, like the K70).
    mouse_polling_sent: Option<u8>,
    status: Status,
    kb_opts: KeyboardOptions,
    hs_opts: HeadsetOptions,
    /// Polling rate last sent on connect, so a rate that does not take is not
    /// re-sent (and the keyboard re-enumerated) on every scan.
    polling_sent: Option<u8>,
}

/// Mirror the boom mute onto the system's default microphone (PipeWire).
fn sync_system_mute(muted: bool) {
    #[cfg(target_os = "linux")]
    thread::spawn(move || {
        let _ = std::process::Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", if muted { "1" } else { "0" }])
            .status();
    });
    #[cfg(not(target_os = "linux"))]
    let _ = muted;
}

impl Devices {
    /// Firmware Win Lock: Windows key (+ chosen combinations) dropped by the
    /// keyboard itself, so it keeps working after the app quits.
    fn apply_lock(&self) {
        if let Some(kb) = &self.k70 {
            let _ = kb.set_lock(self.kb_opts.lock(self.status.win_lock));
        }
    }

    /// New mute state from the headset: LED, UI and (optionally) the system mic.
    fn set_mic_muted(&mut self, muted: bool) {
        let Some(info) = self.status.headset.as_mut() else {
            return;
        };
        if info.mic_muted != Some(muted) {
            info.mic_muted = Some(muted);
            if self.hs_opts.mic_sync {
                sync_system_mute(muted);
            }
        }
    }

    fn rescan(&mut self, swallowed: &[u8]) {
        // Keep scanning while on the receiver: a cabled headset takes precedence.
        let headset_settled = self.hs80.as_ref().is_some_and(|h| h.wired);
        if self.k70.is_some() && headset_settled && self.m65.is_some() {
            return;
        }
        let _ = self.api.refresh_devices();
        for found in enumerate(&self.api) {
            if !found.writable() {
                continue; // untested: read-only until verified
            }
            match found.kind {
                DeviceKind::LegacyKeyboard if self.k70.is_none() && !self.status.icue_running => {
                    if let Ok(kb) = K70::open(&self.api, &found) {
                        // Polling first, still in hardware mode: the keyboard
                        // re-enumerates, and is picked up again on the next scan.
                        let want = self.kb_opts.polling_ms;
                        if self.polling_sent != Some(want) && kb.polling_ms().is_some_and(|ms| ms != want) {
                            self.polling_sent = Some(want);
                            let _ = kb.set_polling_ms(want);
                            continue;
                        }
                        let fw = kb.firmware_info().map(|f| format!("{}.{:02x}", f.version >> 8, f.version & 0xff));
                        if kb.set_software_mode(true).is_ok() && kb.set_key_modes(swallowed).is_ok() {
                            self.status.keyboard =
                                Some(KeyboardInfo { name: found.name.into(), firmware: fw.unwrap_or_else(|_| "?".into()) });
                            self.k70 = Some(kb);
                            self.apply_lock();
                        }
                    }
                }
                DeviceKind::LegacyMouse if self.m65.is_none() && !self.status.icue_running => {
                    if let Ok(m) = M65::open(&self.api, &found) {
                        // Polling first, in hardware mode: the mouse re-enumerates.
                        let want = self.mouse_opts.polling_ms;
                        if self.mouse_polling_sent != Some(want) && m.polling_ms().is_some_and(|ms| ms != want) {
                            self.mouse_polling_sent = Some(want);
                            let _ = m.set_polling_ms(want);
                            continue;
                        }
                        let fw = m.firmware_info().map(|f| format!("{}.{:02x}", f.version >> 8, f.version & 0xff));
                        let stage = self.mouse_stage.min(m65::DPI_STAGES as u8 - 1);
                        if m.set_software_mode(true).is_ok() {
                            self.mouse_opts.apply(&m, stage);
                            let _ = m.set_button_modes(self.mouse_host_only);
                            self.status.mouse = Some(MouseInfo {
                                name: found.name.into(),
                                firmware: fw.unwrap_or_else(|_| "?".into()),
                                dpi_stage: stage,
                            });
                            self.m65 = Some(m);
                        }
                    }
                }
                DeviceKind::BragiDongle
                    if self.hs80.is_none() || (found.pid == op_core::hs80::PID_WIRED && !headset_settled) =>
                {
                    if let Ok(hs) = Hs80::open(&self.api, &found) {
                        take_headset(&hs, &self.hs_opts);
                        let (battery, charging) = read_headset(&hs);
                        let name = if hs.wired { "HS80 RGB (cabo USB)" } else { "HS80 RGB Wireless" };
                        let firmware = hs.firmware().ok();
                        self.status.headset =
                            Some(HeadsetInfo { name: name.into(), battery, charging, firmware, mic_muted: None });
                        if let Ok(m) = hs.mic_muted() {
                            self.set_mic_muted(m);
                        }
                        self.hs80 = Some(hs);
                    }
                }
                _ => {}
            }
        }
    }

    fn drop_mouse(&mut self) {
        self.m65 = None;
        self.status.mouse = None;
    }

    fn drop_keyboard(&mut self) {
        self.k70 = None;
        self.status.keyboard = None;
    }
}

/// Keys the OS must not see: the remapped ones (Win Lock is done by the firmware).
fn swallowed(bindings: &HashMap<u8, Action>) -> Vec<u8> {
    bindings.iter().filter(|(_, a)| !a.passthrough()).map(|(&k, _)| k).collect()
}

/// Mouse buttons the OS must not see (bits): the remapped ones.
fn mouse_host_only(bindings: &HashMap<u8, Action>) -> u8 {
    bindings.iter().filter(|(_, a)| !a.passthrough()).fold(0, |m, (&bit, _)| m | bit)
}

fn run(
    rx: Receiver<Cmd>,
    mut lighting: Lighting,
    on_status: impl Fn(&Status),
    on_key: impl Fn(u8),
    on_profile: impl Fn(String),
    on_wheel_click: impl Fn(),
    on_brightness_key: impl Fn(),
) {
    let api = match HidApi::new() {
        Ok(a) => a,
        Err(_) => return,
    };
    let mut dev = Devices {
        api,
        k70: None,
        hs80: None,
        m65: None,
        mouse_stage: 2,
        mouse_opts: MouseOptions::default(),
        mouse_host_only: 0,
        mouse_polling_sent: None,
        status: Status::default(),
        kb_opts: KeyboardOptions::default(),
        hs_opts: HeadsetOptions::default(),
        polling_sent: None,
    };
    let mut geometry = Geometry::new();
    let mut heat = Heat::new();
    let start = Instant::now();
    let (mut last_scan, mut last_battery, mut last_icue) = (None::<Instant>, Instant::now(), None::<Instant>);
    let mut last_frame = Instant::now();
    let mut dirty = true;
    let mut reported = None::<Status>;
    let mut bindings: HashMap<u8, Action> = HashMap::new();
    let mut mouse_bindings: HashMap<u8, Action> = HashMap::new();
    let mut injector = Injector::new();
    let mut prev_keys: Vec<u8> = Vec::new();
    let mut wheel_down = None::<Instant>;
    let mut mouse_buttons = 0u8;
    let (mut mouse_tick, mut mouse_sent_at) = (Instant::now(), Instant::now());
    let mut mouse_last = None::<(op_core::Rgb, op_core::Rgb, op_core::Rgb)>;
    let mut flash_until = None::<Instant>;
    // Last LED tick, last frame actually sent and when.
    let (mut hs_tick, mut hs_sent_at, mut hs_last_frame) = (Instant::now(), Instant::now(), None::<[u8; hs80::LED_BYTES]>);

    loop {
        // Commands.
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::SetLighting(l) => {
                    lighting = l;
                    dirty = true;
                }
                Cmd::SetGeometry(g) => {
                    geometry = g;
                    dirty = true;
                }
                Cmd::SetBindings(b) => {
                    injector.stop_all();
                    bindings = b;
                    if let Some(kb) = &dev.k70 {
                        let _ = kb.set_key_modes(&swallowed(&bindings));
                        dev.apply_lock();
                    }
                }
                Cmd::SetWinLock(on) => {
                    dev.status.win_lock = on;
                    dev.apply_lock();
                    dirty = true;
                }
                Cmd::SetKeyboard(o) => {
                    let polling_changed = o.polling_ms != dev.kb_opts.polling_ms;
                    dev.kb_opts = o;
                    dev.apply_lock();
                    if let Some(kb) = dev.k70.as_ref().filter(|_| polling_changed) {
                        // Hand the keyboard back before it drops off the bus;
                        // the rescan applies the new rate and takes it again.
                        let _ = kb.set_software_mode(false);
                        let _ = kb.set_polling_ms(o.polling_ms);
                        dev.drop_keyboard();
                        last_scan = Some(Instant::now());
                    }
                }
                Cmd::SetHeadset(o) => {
                    let sleep_changed = o.sleep_min != dev.hs_opts.sleep_min;
                    dev.hs_opts = o;
                    if let (Some(hs), Some(m)) = (&dev.hs80, o.sleep_min.filter(|_| sleep_changed)) {
                        let _ = hs.set_sleep(m);
                    }
                }
                Cmd::SetMouseStage(stage) => {
                    let stage = stage.min(m65::DPI_STAGES as u8 - 1);
                    dev.mouse_stage = stage;
                    if let (Some(m), Some(info)) = (&dev.m65, dev.status.mouse.as_mut()) {
                        info.dpi_stage = stage;
                        let _ = m.select_dpi_stage(Some(stage as usize));
                    }
                }
                Cmd::SetMouseBindings(b) => {
                    injector.stop_all();
                    mouse_bindings = b;
                    dev.mouse_host_only = mouse_host_only(&mouse_bindings);
                    if let Some(m) = &dev.m65 {
                        let _ = m.set_button_modes(dev.mouse_host_only);
                    }
                }
                Cmd::SetMouse(o) => {
                    let polling_changed = o.polling_ms != dev.mouse_opts.polling_ms;
                    dev.mouse_opts = o;
                    if let Some(m) = &dev.m65 {
                        if polling_changed {
                            // Hand it back before it drops off the bus; the rescan
                            // sets the new rate and takes it again.
                            let _ = m.set_button_modes(0);
                            let _ = m.set_software_mode(false);
                            let _ = m.set_polling_ms(dev.mouse_opts.polling_ms);
                            dev.mouse_polling_sent = Some(dev.mouse_opts.polling_ms);
                            dev.drop_mouse();
                            last_scan = Some(Instant::now());
                        } else {
                            dev.mouse_opts.apply(m, dev.mouse_stage);
                        }
                    }
                    mouse_last = None;
                }
                Cmd::Shutdown(done) => {
                    injector.stop_all();
                    if let Some(m) = &dev.m65 {
                        // Every button back to normal first: a host-only left
                        // button would leave the mouse unable to click.
                        let _ = m.set_button_modes(0);
                        let _ = m.set_software_mode(false); // onboard lighting
                    }
                    if let Some(kb) = &dev.k70 {
                        let _ = kb.set_software_mode(false); // onboard lighting + normal keys
                        // Leaving software mode clears the lock; set again, it holds.
                        dev.apply_lock();
                    }
                    if let Some(hs) = &dev.hs80 {
                        let _ = hs.set_software_mode(false); // onboard status LED + lighting
                    }
                    let _ = done.send(());
                    return;
                }
            }
        }

        let now = Instant::now();
        if last_icue.is_none_or(|t| now - t > ICUE_CHECK) {
            dev.status.icue_running = icue_running();
            if dev.status.icue_running && dev.k70.is_some() {
                dev.drop_keyboard(); // let iCUE have it; we take it back when it quits
            }
            last_icue = Some(now);
        }
        if last_scan.is_none_or(|t| now - t > RESCAN) {
            dev.rescan(&swallowed(&bindings));
            if dev.k70.is_some() {
                dirty = true;
            }
            last_scan = Some(now);
        }
        if now - last_battery > BATTERY_POLL {
            if let (Some(hs), Some(info)) = (&dev.hs80, dev.status.headset.as_mut()) {
                let was_off = info.battery.is_none();
                (info.battery, info.charging) = read_headset(hs);
                if was_off && info.battery.is_some() {
                    take_headset(hs, &dev.hs_opts); // powered back on in hardware mode
                    info.firmware = hs.firmware().ok();
                    hs_last_frame = None;
                }
                // Cable pulled: forget the wired handle so the receiver is picked up again.
                if hs.wired && info.battery.is_none() {
                    dev.hs80 = None;
                    dev.status.headset = None;
                    last_scan = None;
                }
            }
            last_battery = now;
        }

        // Mouse: follow the DPI buttons (the firmware changes the DPI itself; we
        // re-assert the stage so ours stays the truth), then its LEDs.
        let mut mouse_lost = false;
        if let (Some(m), Some(info)) = (&dev.m65, dev.status.mouse.as_mut()) {
            loop {
                match m.read_buttons(0) {
                    Ok(Some(bits)) => {
                        let pressed = bits & !mouse_buttons;
                        let released = mouse_buttons & !bits;
                        mouse_buttons = bits;
                        // Remapped buttons: their action replaces the default one.
                        for (bit, _, name) in m65::BUTTONS {
                            let Some(action) = mouse_bindings.get(&bit) else { continue };
                            let id = format!("m{name}");
                            if pressed & bit != 0 {
                                if let Action::Profile { target } = action {
                                    on_profile(target.clone());
                                }
                                injector.press(&id, action);
                            } else if released & bit != 0 {
                                injector.release(&id, action);
                            }
                        }
                        let bound = mouse_bindings.keys().fold(0u8, |m, b| m | b);
                        let (pressed, released) = (pressed & !bound, released & !bound);
                        // Sniper: once the app defines the stages the firmware no
                        // longer slows down by itself, so select the sniper stage
                        // while held (as OpenLinkHub does).
                        if pressed & m65::BUTTON_SNIPER != 0 {
                            let _ = m.select_dpi_stage(None);
                        } else if released & m65::BUTTON_SNIPER != 0 {
                            let _ = m.select_dpi_stage(Some(info.dpi_stage as usize));
                        }
                        let max = m65::DPI_STAGES as u8 - 1;
                        let next = if pressed & m65::BUTTON_DPI_UP != 0 {
                            Some((info.dpi_stage + 1).min(max))
                        } else if pressed & m65::BUTTON_DPI_DOWN != 0 {
                            Some(info.dpi_stage.saturating_sub(1))
                        } else {
                            None
                        };
                        if let Some(n) = next {
                            info.dpi_stage = n;
                            dev.mouse_stage = n;
                            let _ = m.select_dpi_stage(Some(n as usize));
                        }
                    }
                    Ok(None) => break,
                    Err(_) => {
                        mouse_lost = true;
                        break;
                    }
                }
            }
            if now - mouse_tick >= HS_FRAME {
                let (logo, wheel) = dev.mouse_opts.zones(&lighting, (now - start).as_secs_f32());
                let sniper = mouse_buttons & m65::BUTTON_SNIPER != 0;
                let frame = (logo, dev.mouse_opts.dpi_color(info.dpi_stage, sniper), wheel);
                if mouse_last != Some(frame) || now - mouse_sent_at >= HS_KEEPALIVE {
                    mouse_lost |= m.set_leds(frame.0, frame.1, frame.2).is_err();
                    mouse_last = Some(frame);
                    mouse_sent_at = now;
                }
                mouse_tick = now;
            }
        }
        if mouse_lost {
            dev.drop_mouse();
            mouse_last = None;
        }

        // Headset: wheel clicks, then its LEDs.
        let mut events = Vec::new();
        if let Some(hs) = &dev.hs80 {
            while let Ok(Some(ev)) = hs.read_event(0) {
                events.push(ev);
            }
        }
        for ev in events {
            match ev {
                Event::WheelButton { pressed: true } => wheel_down = Some(now),
                Event::WheelButton { pressed: false } => {
                    if wheel_down.take().is_some_and(|t| now - t < WHEEL_CLICK) {
                        on_wheel_click();
                        flash_until = Some(now + WHEEL_FLASH);
                    }
                }
                // Battery (tenths of %): show it now, the poll stays as fallback.
                Event::Property { prop: hs80::PROP_BATTERY_LEVEL, value } => {
                    if let Some(info) = dev.status.headset.as_mut().filter(|i| i.battery.is_some()) {
                        info.battery = Some(value as f32 / 10.0);
                    }
                }
                Event::Property { prop: hs80::PROP_MIC_MUTE | hs80::PROP_MIC_MUTE_EVENT, value } => {
                    dev.set_mic_muted(value & 0xff == 1);
                }
                Event::Property { .. } => {}
            }
        }
        if let Some(hs) = &dev.hs80 {
            if now - hs_tick >= HS_FRAME {
                let flash = flash_until.is_some_and(|t| now < t);
                let frame =
                    headset_frame(&lighting, (now - start).as_secs_f32(), dev.status.headset.as_ref(), flash);
                if hs_last_frame != Some(frame) || now - hs_sent_at >= HS_KEEPALIVE {
                    let _ = hs.write_leds(&frame);
                    hs_last_frame = Some(frame);
                    hs_sent_at = now;
                }
                hs_tick = now;
            }
        }

        // Key presses (for Reactive). Short timeout doubles as the frame pacing.
        let mut lost = false;
        if let Some(kb) = &dev.k70 {
            match kb.read_keys(Duration::from_millis(5)) {
                Ok(Some(bits)) => {
                    let held = |b: &[u8], i: usize| b.get(i / 8).is_some_and(|x| x & (1 << (i % 8)) != 0);
                    for i in (0..bits.len() * 8).filter(|&i| held(&bits, i)) {
                        if let Some(led) = k70::led_for_key(i as u8) {
                            heat.press(led);
                        }
                        if !held(&prev_keys, i) {
                            on_key(i as u8);
                            // Top-left keys are reported to us only; act on them here
                            // (unless remapped).
                            match i as u8 {
                                k if bindings.contains_key(&k) => {}
                                k70::KEY_PROFILE => on_profile("next".into()),
                                k70::KEY_BRIGHTNESS => on_brightness_key(),
                                k70::KEY_WINLOCK => {
                                    dev.status.win_lock = !dev.status.win_lock;
                                    let _ = kb.set_lock(dev.kb_opts.lock(dev.status.win_lock));
                                    dirty = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    // Remapped keys: fire on press / release edges.
                    for (&slot, action) in &bindings {
                        let (was, is) = (held(&prev_keys, slot as usize), held(&bits, slot as usize));
                        if is && !was {
                            if let Action::Profile { target } = action {
                                on_profile(target.clone());
                            }
                            injector.press(&format!("k{slot}"), action);
                        } else if was && !is {
                            injector.release(&format!("k{slot}"), action);
                        }
                    }
                    prev_keys = bits;
                }
                Ok(None) => {}
                Err(_) => lost = true,
            }
        }

        let dt = (now - last_frame).as_secs_f32();
        if now - last_frame >= FRAME {
            heat.decay(dt, lighting.speed);
            if dirty || effects::animated(&lighting, &heat) {
                if let Some(kb) = &dev.k70 {
                    let mut frame = effects::render(&lighting, &geometry, (now - start).as_secs_f32(), &heat);
                    // Lock symbol shows the Win Lock state, whatever the effect.
                    frame[k70::LED_WINLOCK] =
                        if dev.status.win_lock { op_core::Rgb::new(255, 255, 255) } else { op_core::Rgb::default() };
                    lost |= kb.set_leds(&frame).is_err();
                }
                dirty = false;
            }
            last_frame = now;
        }
        if lost {
            dev.drop_keyboard();
        }

        if reported.as_ref() != Some(&dev.status) {
            on_status(&dev.status);
            reported = Some(dev.status.clone());
        }
        thread::sleep(Duration::from_millis(4));
    }
}

#[cfg(windows)]
fn icue_running() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("tasklist")
        .args(["/NH", "/FI", "IMAGENAME eq iCUE.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("iCUE.exe"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn icue_running() -> bool {
    false // iCUE does not exist on Linux
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_only_while_win_lock_is_on() {
        let o = KeyboardOptions { alt_tab: true, ..Default::default() };
        assert_eq!(o.lock(false).byte(), 0);
        assert_eq!(o.lock(true).byte(), 0xA3);
    }

    #[test]
    fn mic_led_red_when_muted() {
        let mut info = HeadsetInfo {
            name: "HS80".into(),
            battery: Some(50.0),
            charging: false,
            firmware: None,
            mic_muted: Some(true),
        };
        let mic = |f: [u8; hs80::LED_BYTES]| [f[2], f[5], f[8]];
        assert_eq!(mic(headset_frame(&Lighting::default(), 0.0, Some(&info), false)), [255, 0, 0]);
        info.mic_muted = Some(false);
        assert_eq!(mic(headset_frame(&Lighting::default(), 0.0, Some(&info), false)), [0, 0, 0]);
    }
}
