//! opctl: command-line front end to op-core, used to test drivers on real hardware.

use std::{thread, time::Duration};

use clap::{Parser, Subcommand};
use hidapi::HidApi;
use op_core::{descriptor, enumerate, registry, hs80, hs80::Hs80, k70::K70, m65::M65, DeviceKind, FoundDevice, Rgb, CORSAIR_VID};

#[derive(Parser)]
#[command(about = "OpenPeripheral device control")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List known devices that are plugged in, with their database status.
    List,
    /// Print the device database (embedded + user file).
    Db {
        /// Only entries with this status (verified, community, untested).
        #[arg(long)]
        status: Option<String>,
    },
    /// Print udev rules for every device that is not untested
    /// (`opctl udev-rules > linux/70-openperipheral.rules`).
    UdevRules,
    /// Dump every Corsair HID collection with its report layout (read-only).
    Probe,
    /// Print every input report from all vendor collections for N seconds (read-only).
    Listen {
        #[arg(default_value_t = 5)]
        seconds: u64,
    },
    /// Send one raw report and print replies for 1 s: `raw 1b55 ffc2 00 0e 01`.
    /// First data byte is the report ID (00 when the collection has none).
    Raw {
        #[arg(value_parser = parse_u16)]
        pid: u16,
        #[arg(value_parser = parse_u16)]
        usage_page: u16,
        #[arg(value_parser = parse_u8, num_args = 1..)]
        bytes: Vec<u8>,
        /// Pad to this many bytes (including report ID).
        #[arg(long, default_value_t = 65)]
        len: usize,
        /// Use SET_FEATURE / GET_FEATURE instead of output/input reports.
        #[arg(long)]
        feature: bool,
    },
    #[command(subcommand)]
    K70(K70Cmd),
    #[command(subcommand)]
    Hs80(Hs80Cmd),
    #[command(subcommand)]
    M65(M65Cmd),
}

#[derive(Subcommand)]
enum M65Cmd {
    /// Read firmware info (read-only).
    Info,
    /// Software mode, colour the logo / DPI LED / wheel for N seconds, then back to hardware mode.
    Color {
        logo: Rgb,
        dpi: Rgb,
        wheel: Rgb,
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
    /// Software mode with only logo + wheel coloured for N seconds (DPI LED left alone).
    Twozone {
        logo: Rgb,
        wheel: Rgb,
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
    /// Define DPI stage 1..5 (0 = sniper): DPI and indicator colour.
    Dpi { n: usize, dpi: u16, color: Rgb },
    /// USB polling interval in ms (1, 2, 4, 8); prints the current one without an argument.
    Polling { ms: Option<u8> },
    /// Angle snapping on (1) / off (0).
    Angle { on: u8 },
    /// Button response optimization on (1) / off (0).
    Btnopt { on: u8 },
    /// Software mode for N seconds with these buttons (hex bit mask, e.g. 18 = back+forward)
    /// host-only, then everything back to normal.
    Hostonly {
        #[arg(value_parser = parse_u8_hex)]
        bits: u8,
        #[arg(long, default_value_t = 20)]
        seconds: u64,
    },
    /// Make DPI stage 1..5 active (0 = sniper).
    Stage { n: usize },
    /// Return the mouse to its onboard lighting and DPI handling.
    Release,
}

#[derive(Subcommand)]
enum K70Cmd {
    /// Read firmware info (read-only).
    Info,
    /// Set every key to one colour, e.g. `ff0000`.
    Color { rgb: Rgb },
    /// Rainbow wave for N seconds.
    Rainbow {
        #[arg(default_value_t = 10)]
        seconds: u64,
    },
    /// Light one LED index (others off) to map keys to indices.
    Index { index: usize },
    /// Return the keyboard to its onboard lighting.
    Release,
    /// Firmware key lock (`07 48`): `lock win alt-tab alt-f4 shift-tab`, nothing = off.
    Lock {
        #[arg(num_args = 0..)]
        keys: Vec<String>,
    },
    /// Read a property (`0E prop`) and print the reply: `read 0x0a`.
    Read {
        #[arg(value_parser = parse_u8)]
        prop: u8,
    },
    /// Set one key's input mode: `keymode 0x60 0xc8`.
    Keymode {
        #[arg(value_parser = parse_u8)]
        key: u8,
        #[arg(value_parser = parse_u8)]
        mode: u8,
    },
    /// Polling interval in ms (1, 2, 4, 8).
    Polling { ms: u8 },
    /// Key mapper test: a pressed key lights its own LED (`k70::led_for_key`).
    /// The keyboard does NOT type in Windows while this runs; restores itself at the end.
    Echo {
        #[arg(default_value_t = 25)]
        seconds: u64,
    },
    /// Interactive key mapper: prompts each key name from a TSV (name<TAB>x<TAB>y),
    /// records the LED slot reported when it is pressed, writes JSON.
    /// A key not pressed within 10 s is skipped (e.g. logo LEDs).
    Map {
        #[arg(long)]
        keys: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
    },
}

/// Puts the keyboard back in hardware mode (normal typing) even on error/panic.
struct Restore<'a>(&'a K70);
impl Drop for Restore<'_> {
    fn drop(&mut self) {
        let _ = self.0.set_software_mode(false);
    }
}

#[derive(Subcommand)]
enum Hs80Cmd {
    /// Headset battery level (read-only).
    Battery,
    /// Software lighting: send 9 raw LED bytes for N seconds, e.g. `leds ff0000 00ff00 0000ff`.
    Leds {
        /// 9 bytes as hex, in any grouping.
        #[arg(num_args = 1..)]
        hex: Vec<String>,
        #[arg(long, default_value_t = 10)]
        seconds: u64,
    },
    /// Return headset and receiver to onboard lighting.
    Release,
    /// Write a property: `set 0x0d 1 [--target 0x09] [--u32]`.
    Set {
        #[arg(value_parser = parse_u8)]
        prop: u8,
        value: u32,
        #[arg(long, value_parser = parse_u8, default_value = "0x09")]
        target: u8,
        #[arg(long)]
        u32: bool,
    },
    /// Read a property: `get 0x0f [--target 0x09]`.
    Get {
        #[arg(value_parser = parse_u8)]
        prop: u8,
        #[arg(long, value_parser = parse_u8, default_value = "0x09")]
        target: u8,
    },
}

fn parse_u8(s: &str) -> Result<u8, String> {
    let s = s.trim_start_matches("0x");
    u8::from_str_radix(s, 16).map_err(|e| e.to_string())
}

fn parse_u8_hex(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

fn parse_u16(s: &str) -> Result<u16, String> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

fn find(api: &HidApi, kind: DeviceKind) -> FoundDevice {
    enumerate(api).into_iter().find(|d| d.kind == kind).unwrap_or_else(|| {
        eprintln!("device not found ({kind:?})");
        std::process::exit(1)
    })
}

fn main() {
    let cli = Cli::parse();
    let api = HidApi::new().expect("hidapi init");
    if let Err(e) = run(&api, cli.cmd) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(api: &HidApi, cmd: Cmd) -> op_core::Result<()> {
    match cmd {
        Cmd::List => {
            for d in enumerate(api) {
                println!(
                    "{:04x}  {:<30} {:<9} family={:<5} serial={} control_collections={}",
                    d.pid,
                    d.name,
                    d.status.as_str(),
                    d.family.map_or("?", |f| f.as_str()),
                    d.serial,
                    d.control.len()
                );
            }
        }
        Cmd::Db { status } => {
            let db = registry::db();
            if let Some(p) = registry::user_db_path() {
                println!("# embedded devices/corsair.toml + {} (if present)", p.display());
            }
            for d in &db.devices {
                let st = d.status.as_str();
                if status.as_deref().is_some_and(|s| s != st) {
                    continue;
                }
                let control: Vec<String> = d.control.iter().map(|(k, u)| format!("{k}={u}")).collect();
                println!(
                    "{:04x}  {:<40} {:<14} {:<5} {:<9} {} [{}]",
                    d.pid,
                    d.name,
                    d.kind.as_str(),
                    d.family.map_or("?", |f| f.as_str()),
                    st,
                    control.join(" "),
                    d.sources.join(",")
                );
            }
        }
        Cmd::UdevRules => {
            print!("{}", registry::udev_rules(&op_core::registry::Db::embedded()));
        }
        Cmd::Probe => {
            for info in api.device_list().filter(|d| d.vendor_id() == CORSAIR_VID) {
                println!(
                    "pid={:04x} if={} usage={:04x}:{:04x} {}",
                    info.product_id(),
                    info.interface_number(),
                    info.usage_page(),
                    info.usage(),
                    info.product_string().unwrap_or("?")
                );
                if info.usage_page() < 0xFF00 {
                    continue;
                }
                let Ok(dev) = info.open_device(api) else {
                    println!("    (cannot open)");
                    continue;
                };
                let mut raw = [0u8; 4096];
                if let Ok(n) = dev.get_report_descriptor(&mut raw) {
                    for (id, s) in descriptor::parse(&raw[..n]).reports {
                        println!("    report id {id:#04x}: in={} out={} feature={}", s.input, s.output, s.feature);
                    }
                }
            }
        }
        Cmd::Listen { seconds } => {
            let start = std::time::Instant::now();
            let handles: Vec<_> = api
                .device_list()
                .filter(|d| d.vendor_id() == CORSAIR_VID && d.usage_page() >= 0xFF00)
                .filter_map(|d| {
                    let dev = d.open_device(api).ok()?;
                    dev.set_blocking_mode(false).ok()?;
                    Some((format!("{:04x} {:04x}:{:04x}", d.product_id(), d.usage_page(), d.usage()), dev))
                })
                .collect();
            println!("listening on {} collections", handles.len());
            let mut buf = [0u8; 128];
            while start.elapsed() < Duration::from_secs(seconds) {
                for (name, dev) in &handles {
                    while let Ok(n) = dev.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        println!("{:7.3}s {name}  {}", start.elapsed().as_secs_f32(), hs80::hex(&buf[..n.min(24)]));
                    }
                }
                thread::sleep(Duration::from_millis(5));
            }
        }
        Cmd::Raw { pid, usage_page, bytes, len, feature } => {
            let info = api
                .device_list()
                .find(|d| d.vendor_id() == CORSAIR_VID && d.product_id() == pid && d.usage_page() == usage_page)
                .ok_or(op_core::Error::NotFound("collection"))?;
            let dev = info.open_device(api)?;
            let mut buf = bytes.clone();
            buf.resize(len.max(bytes.len()), 0);
            if feature {
                dev.send_feature_report(&buf)?;
                let mut r = vec![0u8; len];
                r[0] = bytes[0];
                let n = dev.get_feature_report(&mut r)?;
                println!("feature reply: {}", hs80::hex(&r[..n.min(32)]));
            } else {
                let n = dev.write(&buf)?;
                println!("wrote {n} bytes");
                let mut r = [0u8; 128];
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_secs(1) {
                    let n = dev.read_timeout(&mut r, 100)?;
                    if n > 0 {
                        println!("{:6.3}s  {}", start.elapsed().as_secs_f32(), hs80::hex(&r[..n.min(32)]));
                    }
                }
            }
        }
        Cmd::K70(c) => {
            let kb = K70::open(api, &find(api, DeviceKind::LegacyKeyboard))?;
            match c {
                K70Cmd::Info => {
                    let fw = kb.firmware_info()?;
                    println!("firmware {:#06x}  bootloader {:#06x}  vid:pid {:04x}:{:04x}", fw.version, fw.bootloader, fw.vid, fw.pid);
                    println!("raw: {}", hs80::hex(&fw.raw[..24]));
                    println!("polling: {:?} ms", kb.polling_ms());
                }
                K70Cmd::Color { rgb } => {
                    kb.set_software_mode(true)?;
                    kb.set_all(rgb)?;
                }
                K70Cmd::Rainbow { seconds } => {
                    kb.set_software_mode(true)?;
                    let frames = seconds * 30;
                    for f in 0..frames {
                        let mut leds = [Rgb::default(); op_core::k70::LED_COUNT];
                        for (i, led) in leds.iter_mut().enumerate() {
                            *led = Rgb::from_hue(f as f32 * 6.0 + i as f32 * 4.0);
                        }
                        kb.set_leds(&leds)?;
                        thread::sleep(Duration::from_millis(33));
                    }
                }
                K70Cmd::Index { index } => {
                    kb.set_software_mode(true)?;
                    let mut leds = [Rgb::default(); op_core::k70::LED_COUNT];
                    if let Some(l) = leds.get_mut(index) {
                        *l = Rgb::new(255, 255, 255);
                    }
                    kb.set_leds(&leds)?;
                }
                K70Cmd::Release => kb.set_software_mode(false)?,
                K70Cmd::Lock { keys } => {
                    let has = |k: &str| keys.iter().any(|x| x == k);
                    let lock = op_core::k70::Lock {
                        windows: has("win"),
                        alt_tab: has("alt-tab"),
                        alt_f4: has("alt-f4"),
                        shift_tab: has("shift-tab"),
                    };
                    kb.set_lock(lock)?;
                    println!("lock byte {:#04x}", lock.byte());
                }
                K70Cmd::Read { prop } => println!("{}", hs80::hex(&kb.read_property(prop)?[..24])),
                K70Cmd::Keymode { key, mode } => kb.set_key_mode(key, mode)?,
                K70Cmd::Polling { ms } => kb.set_polling_ms(ms)?,
                K70Cmd::Map { keys, out } => {
                    let list = std::fs::read_to_string(&keys).expect("read key list");
                    // Columns: name, x, y, optional human label.
                    let entries: Vec<(&str, &str)> = list
                        .lines()
                        .filter_map(|l| {
                            let f: Vec<&str> = l.split('\t').collect();
                            let name = *f.first().filter(|n| !n.is_empty())?;
                            Some((name, f.get(3).copied().unwrap_or(name)))
                        })
                        .collect();
                    let _restore = Restore(&kb);
                    kb.set_software_mode(true)?;
                    kb.capture_keys()?;
                    let dim = Rgb::new(0, 0, 40);
                    let mut leds = [dim; op_core::k70::LED_COUNT];
                    kb.set_leds(&leds)?;
                    let mut map: Vec<(String, Option<usize>)> = Vec::new();
                    for (n, &(name, label)) in entries.iter().enumerate() {
                        if name.starts_with("Led_") {
                            // LED without a key (logo): nothing to press.
                            map.push((name.to_string(), None));
                            continue;
                        }
                        println!("[{:3}/{}] Pressione: {label}", n + 1, entries.len());
                        let deadline = std::time::Instant::now() + Duration::from_secs(10);
                        let mut slot = None;
                        while std::time::Instant::now() < deadline {
                            let Some(bits) = kb.read_keys(Duration::from_millis(100))? else { continue };
                            let held: Vec<usize> =
                                (0..bits.len() * 8).filter(|i| bits[i / 8] & (1 << (i % 8)) != 0).collect();
                            if let [one] = held[..] {
                                // Already assigned: the user pressed the wrong key, keep waiting.
                                if let Some((other, _)) = map.iter().find(|(_, s)| *s == Some(one)) {
                                    let other_label =
                                        entries.iter().find(|(n, _)| n == other).map_or(other.as_str(), |e| e.1);
                                    println!("         (essa já foi registrada como: {other_label} — aperte a pedida, ou espere para pular)");
                                    while kb.read_keys(Duration::from_millis(300))?.is_some_and(|b| b.iter().any(|&x| x != 0)) {}
                                    continue;
                                }
                                slot = Some(one);
                                break;
                            }
                        }
                        match slot {
                            Some(s) => {
                                println!("         -> slot {s}");
                                if let Some(l) = leds.get_mut(s) {
                                    *l = Rgb::new(0, 255, 0);
                                }
                                kb.set_leds(&leds)?;
                                // Wait for release so key repeat is not read as the next key.
                                while kb.read_keys(Duration::from_millis(300))?.is_some_and(|b| b.iter().any(|&x| x != 0)) {}
                            }
                            None => println!("         -> pulado"),
                        }
                        map.push((name.to_string(), slot));
                    }
                    let body: Vec<String> = map
                        .iter()
                        .map(|(k, v)| format!("  \"{k}\": {}", v.map_or("null".into(), |s| s.to_string())))
                        .collect();
                    std::fs::write(&out, format!("{{\n{}\n}}\n", body.join(",\n"))).expect("write map");
                    println!("mapa salvo em {}", out.display());
                }
                K70Cmd::Echo { seconds } => {
                    let _restore = Restore(&kb);
                    kb.set_software_mode(true)?;
                    kb.capture_keys()?;
                    let dim = Rgb::new(0, 0, 40);
                    let mut leds = [dim; op_core::k70::LED_COUNT];
                    kb.set_leds(&leds)?;
                    let start = std::time::Instant::now();
                    while start.elapsed() < Duration::from_secs(seconds) {
                        let Some(bits) = kb.read_keys(Duration::from_millis(200))? else { continue };
                        let held: Vec<usize> = (0..bits.len() * 8).filter(|i| bits[i / 8] & (1 << (i % 8)) != 0).collect();
                        leds = [dim; op_core::k70::LED_COUNT];
                        for &i in &held {
                            if let Some(l) = op_core::k70::led_for_key(i as u8).and_then(|s| leds.get_mut(s)) {
                                *l = Rgb::new(255, 255, 255);
                            }
                        }
                        kb.set_leds(&leds)?;
                        println!("{:6.2}s held bits: {held:?}", start.elapsed().as_secs_f32());
                    }
                }
            }
        }
        Cmd::M65(c) => {
            let m = M65::open(api, &find(api, DeviceKind::LegacyMouse))?;
            match c {
                M65Cmd::Info => {
                    let fw = m.firmware_info()?;
                    println!("firmware {:#06x}  bootloader {:#06x}  pid {:04x}", fw.version, fw.bootloader, fw.pid);
                }
                M65Cmd::Color { logo, dpi, wheel, seconds } => {
                    m.set_software_mode(true)?;
                    let start = std::time::Instant::now();
                    while start.elapsed() < Duration::from_secs(seconds) {
                        m.set_leds(logo, dpi, wheel)?;
                        thread::sleep(Duration::from_millis(500));
                    }
                    m.set_software_mode(false)?;
                }
                M65Cmd::Twozone { logo, wheel, seconds } => {
                    m.set_software_mode(true)?;
                    let start = std::time::Instant::now();
                    while start.elapsed() < Duration::from_secs(seconds) {
                        m.set_zones(&[(op_core::m65::LED_LOGO, logo), (op_core::m65::LED_WHEEL, wheel)])?;
                        thread::sleep(Duration::from_millis(500));
                    }
                    m.set_software_mode(false)?;
                }
                M65Cmd::Dpi { n, dpi, color } => m.set_dpi_stage(n.checked_sub(1), dpi, color)?,
                M65Cmd::Polling { ms: None } => println!("polling: {:?} ms", m.polling_ms()),
                M65Cmd::Polling { ms: Some(ms) } => m.set_polling_ms(ms)?,
                M65Cmd::Angle { on } => m.set_angle_snapping(on != 0)?,
                M65Cmd::Btnopt { on } => m.set_button_optimization(on != 0)?,
                M65Cmd::Hostonly { bits, seconds } => {
                    m.set_software_mode(true)?;
                    m.set_button_modes(bits)?;
                    thread::sleep(Duration::from_secs(seconds));
                    m.set_button_modes(0)?;
                    m.set_software_mode(false)?;
                }
                M65Cmd::Stage { n } => m.select_dpi_stage(n.checked_sub(1))?,
                M65Cmd::Release => m.set_software_mode(false)?,
            }
        }
        Cmd::Hs80(c) => {
            let hs = Hs80::open(api, &find(api, DeviceKind::BragiDongle))?;
            match c {
                Hs80Cmd::Battery => println!("battery: {:.1}%", hs.battery_percent()?),
                Hs80Cmd::Leds { hex, seconds } => {
                    let joined: String = hex.concat();
                    let mut data = [0u8; hs80::LED_BYTES];
                    if joined.len() != data.len() * 2 {
                        return Err(op_core::Error::BadReply(format!("need {} hex bytes", data.len())));
                    }
                    for (i, b) in data.iter_mut().enumerate() {
                        *b = u8::from_str_radix(&joined[i * 2..i * 2 + 2], 16)
                            .map_err(|e| op_core::Error::BadReply(e.to_string()))?;
                    }
                    hs.set_software_mode(true)?;
                    hs.open_lighting()?;
                    let start = std::time::Instant::now();
                    while start.elapsed() < Duration::from_secs(seconds) {
                        hs.write_leds(&data)?;
                        thread::sleep(Duration::from_millis(500));
                    }
                    hs.set_software_mode(false)?;
                }
                Hs80Cmd::Release => hs.set_software_mode(false)?,
                Hs80Cmd::Get { prop, target } => {
                    let r = hs.get_property(target, prop)?;
                    println!("report id {:#04x} reply: {}", hs.report_id(), hs80::hex(&r[..r.len().min(32)]));
                }
                Hs80Cmd::Set { prop, value, target, u32 } => {
                    if u32 {
                        hs.set_property_u32(target, prop, value)?;
                    } else {
                        hs.set_property(target, prop, value as u16)?;
                    }
                    println!("ok");
                }
            }
        }
    }
    Ok(())
}
