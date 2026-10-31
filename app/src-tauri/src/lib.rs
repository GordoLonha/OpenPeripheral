mod battery_icon;
mod effects;
mod engine;
mod eq;
mod foreground;
mod profiles;
mod mouse;
mod remap;
#[cfg(target_os = "linux")]
mod uinput;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WindowEvent,
};

use effects::{Geometry, Lighting};
use engine::{Cmd, Status};
use profiles::{Overview, ProfileInfo, Store};

/// Key name -> key index (report 0x03 bit), measured with `opctl k70 map` on an
/// ABNT2 K70 MK.2 LP. LED slots differ: see `get_ledmap`.
const KEYMAP_BR: &str = include_str!("../../../layout/k70_mk2_lp_BR.json");

struct Active {
    /// Profile in effect (selected, or the one an app switched to).
    id: String,
    auto_app: Option<String>,
}

/// App-wide preferences (not per profile).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AppSettings {
    /// Show the headset battery meter in the notification area.
    #[serde(default = "yes")]
    battery_tray: bool,
    /// Win Lock key state, kept across restarts.
    #[serde(default)]
    win_lock: bool,
    /// Under Win Lock, also block these (firmware lock, K70).
    #[serde(default)]
    lock_alt_tab: bool,
    #[serde(default)]
    lock_alt_f4: bool,
    #[serde(default)]
    lock_shift_tab: bool,
    /// K70 report interval in ms (1 = 1000 Hz).
    #[serde(default = "one")]
    polling_ms: u8,
    /// HS80 auto power-off in minutes (0 = never); None = leave the headset's own.
    #[serde(default)]
    headset_sleep_min: Option<u16>,
    /// Flipping the boom up also mutes the system's default microphone.
    #[serde(default)]
    mic_sync: bool,
    /// M65 DPI stage (0-based), restored when the mouse connects.
    #[serde(default = "two")]
    mouse_stage: u8,
    /// M65 DPI stages, sniper, polling, angle snapping, lighting.
    #[serde(default)]
    mouse: mouse::MouseOptions,
}

fn two() -> u8 {
    2
}

fn yes() -> bool {
    true
}

fn one() -> u8 {
    1
}

impl Default for AppSettings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("defaults")
    }
}

impl AppSettings {
    fn keyboard(&self) -> engine::KeyboardOptions {
        engine::KeyboardOptions {
            alt_tab: self.lock_alt_tab,
            alt_f4: self.lock_alt_f4,
            shift_tab: self.lock_shift_tab,
            polling_ms: self.polling_ms,
        }
    }

    fn headset(&self) -> engine::HeadsetOptions {
        engine::HeadsetOptions { sleep_min: self.headset_sleep_min, mic_sync: self.mic_sync }
    }

    /// Push the device settings to the engine.
    fn apply(&self, engine: &mpsc::Sender<Cmd>) {
        let _ = engine.send(Cmd::SetWinLock(self.win_lock));
        let _ = engine.send(Cmd::SetKeyboard(self.keyboard()));
        let _ = engine.send(Cmd::SetHeadset(self.headset()));
        let _ = engine.send(Cmd::SetMouseStage(self.mouse_stage));
        let _ = engine.send(Cmd::SetMouse(self.mouse.clone()));
    }
}

struct AppState {
    engine: mpsc::Sender<Cmd>,
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    store: Mutex<Store>,
    active: Mutex<Active>,
    status: Mutex<Status>,
    store_path: PathBuf,
}

impl AppState {
    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&*self.store.lock().unwrap()).map_err(|e| e.to_string())?;
        fs::write(&self.store_path, json).map_err(|e| e.to_string())
    }

    fn active_id(&self) -> String {
        self.active.lock().unwrap().id.clone()
    }

    /// Edit the profile currently in effect, then persist.
    fn edit_active(&self, f: impl FnOnce(&mut profiles::Profile)) -> Result<(), String> {
        let id = self.active_id();
        {
            let mut store = self.store.lock().unwrap();
            let p = store.get_mut(&id).ok_or("perfil não encontrado")?;
            f(p);
        }
        self.save()
    }

    fn overview(&self) -> Overview {
        let store = self.store.lock().unwrap();
        let active = self.active.lock().unwrap();
        Overview {
            selected: store.selected.clone(),
            active: active.id.clone(),
            auto_app: active.auto_app.clone(),
            profiles: store
                .profiles
                .iter()
                .map(|p| ProfileInfo { id: p.id.clone(), name: p.name.clone(), apps: p.apps.clone() })
                .collect(),
        }
    }
}

/// Make `id` the profile in effect: push its lighting, keys and EQ to the devices.
fn activate(app: &AppHandle, id: &str, auto_app: Option<String>) {
    let Some(st) = app.try_state::<AppState>() else { return };
    let Some(p) = st.store.lock().unwrap().get(id).cloned() else { return };
    *st.active.lock().unwrap() = Active { id: p.id.clone(), auto_app };
    let _ = st.engine.send(Cmd::SetLighting(p.lighting.clone()));
    let _ = st.engine.send(Cmd::SetBindings(to_slots(&p.bindings)));
    let _ = st.engine.send(Cmd::SetMouseBindings(to_mouse_bits(&p.mouse_bindings)));
    let _ = eq::apply(&p.eq);
    let _ = app.emit("profile", st.overview());
}

/// Key-name bindings -> LED-slot bindings for the engine.
fn to_slots(b: &remap::Bindings) -> HashMap<u8, remap::Action> {
    let map = get_keymap();
    b.iter()
        .filter_map(|(name, a)| map.get(name).copied().flatten().map(|s| (s as u8, a.clone())))
        .collect()
}

// ---- Per-profile settings (always the profile in effect) ----

#[tauri::command]
fn get_lighting(state: State<AppState>) -> Lighting {
    let id = state.active_id();
    state.store.lock().unwrap().get(&id).map(|p| p.lighting.clone()).unwrap_or_default()
}

#[tauri::command]
fn set_lighting(state: State<AppState>, lighting: Lighting) -> Result<(), String> {
    let _ = state.engine.send(Cmd::SetLighting(lighting.clone()));
    state.edit_active(|p| p.lighting = lighting)
}

#[tauri::command]
fn get_bindings(state: State<AppState>) -> remap::Bindings {
    let id = state.active_id();
    state.store.lock().unwrap().get(&id).map(|p| p.bindings.clone()).unwrap_or_default()
}

#[tauri::command]
fn set_bindings(state: State<AppState>, bindings: remap::Bindings) -> Result<(), String> {
    let _ = state.engine.send(Cmd::SetBindings(to_slots(&bindings)));
    state.edit_active(|p| p.bindings = bindings)
}

/// Button-name bindings -> report bit bindings for the engine.
fn to_mouse_bits(b: &remap::Bindings) -> HashMap<u8, remap::Action> {
    op_core::m65::BUTTONS
        .iter()
        .filter_map(|(bit, _, name)| b.get(*name).map(|a| (*bit, a.clone())))
        .collect()
}

#[tauri::command]
fn get_mouse_bindings(state: State<AppState>) -> remap::Bindings {
    let id = state.active_id();
    state.store.lock().unwrap().get(&id).map(|p| p.mouse_bindings.clone()).unwrap_or_default()
}

#[tauri::command]
fn set_mouse_bindings(state: State<AppState>, bindings: remap::Bindings) -> Result<(), String> {
    let _ = state.engine.send(Cmd::SetMouseBindings(to_mouse_bits(&bindings)));
    state.edit_active(|p| p.mouse_bindings = bindings)
}

#[tauri::command]
fn get_eq(state: State<AppState>) -> eq::Eq {
    let id = state.active_id();
    state.store.lock().unwrap().get(&id).map(|p| p.eq.clone()).unwrap_or_default()
}

#[tauri::command]
fn set_eq(state: State<AppState>, value: eq::Eq) -> Result<(), String> {
    eq::apply(&value)?;
    state.edit_active(|p| p.eq = value)
}

#[tauri::command]
fn eq_backend() -> eq::Backend {
    eq::backend()
}

// ---- Profiles ----

#[tauri::command]
fn get_profiles(state: State<AppState>) -> Overview {
    state.overview()
}

#[tauri::command]
fn select_profile(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        if store.get(&id).is_none() {
            return Err("perfil não encontrado".into());
        }
        store.selected = id.clone();
    }
    state.save()?;
    activate(&app, &id, None);
    Ok(())
}

#[tauri::command]
fn create_profile(app: AppHandle, state: State<AppState>, name: String) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("nome vazio".into());
    }
    let from = state.active_id();
    let id = state.store.lock().unwrap().create(&name, &from);
    state.save()?;
    let _ = app.emit("profile", state.overview());
    Ok(id)
}

#[tauri::command]
fn update_profile(app: AppHandle, state: State<AppState>, id: String, name: String, apps: Vec<String>) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        let p = store.get_mut(&id).ok_or("perfil não encontrado")?;
        if !name.trim().is_empty() {
            p.name = name.trim().to_string();
        }
        p.apps = apps.into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    }
    state.save()?;
    let _ = app.emit("profile", state.overview());
    Ok(())
}

#[tauri::command]
fn delete_profile(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let was_active = state.active_id() == id;
    let selected = {
        let mut store = state.store.lock().unwrap();
        if !store.delete(&id) {
            return Err("o último perfil não pode ser apagado".into());
        }
        store.selected.clone()
    };
    state.save()?;
    if was_active {
        activate(&app, &selected, None);
    } else {
        let _ = app.emit("profile", state.overview());
    }
    Ok(())
}

/// Program of the window in front, to help fill a profile's app list.
#[tauri::command]
fn foreground_app() -> Option<String> {
    foreground::names().into_iter().next()
}

// ---- App settings / battery tray ----

#[tauri::command]
fn get_app_settings(state: State<AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

/// How this copy was installed, for the update screen: the updater replaces the
/// Windows install and an AppImage itself; .rpm/.deb go through the package manager.
#[tauri::command]
fn install_kind() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if std::env::var_os("APPIMAGE").is_some() {
        "appimage"
    } else {
        "package"
    }
}

/// Open a web page (release notes, downloads) in the default browser.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/GordoLonha/OpenPeripheral") {
        return Err("endereço não permitido".into());
    }
    remap::launch(&url);
    Ok(())
}

/// Make a DPI stage active (0-based); saved through the status like the DPI buttons.
#[tauri::command]
fn set_mouse_stage(state: State<AppState>, stage: u8) {
    let _ = state.engine.send(Cmd::SetMouseStage(stage));
}

#[tauri::command]
fn set_app_settings(app: AppHandle, state: State<AppState>, value: AppSettings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    value.apply(&state.engine);
    *state.settings.lock().unwrap() = value;
    fs::write(&state.settings_path, json).map_err(|e| e.to_string())?;
    let status = state.status.lock().unwrap().clone();
    update_battery_tray(&app, &status);
    Ok(())
}

const BATTERY_TRAY: &str = "battery";
/// Status line of the battery tray menu, updated in place.
static BATTERY_ITEM: Mutex<Option<MenuItem<tauri::Wry>>> = Mutex::new(None);

/// Show, update or remove the coloured headset battery meter in the tray.
fn update_battery_tray(app: &AppHandle, status: &Status) {
    let enabled = app.try_state::<AppState>().is_some_and(|s| s.settings.lock().unwrap().battery_tray);
    let app2 = app.clone();
    let headset = status.headset.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(hs) = headset.filter(|_| enabled) else {
            let _ = app2.remove_tray_by_id(BATTERY_TRAY);
            *BATTERY_ITEM.lock().unwrap() = None;
            return;
        };
        let state = match (hs.battery, hs.charging) {
            (None, _) => battery_icon::State::Off,
            (Some(p), true) => battery_icon::State::Charging(p),
            (Some(p), false) => battery_icon::State::Discharging(p),
        };
        let icon = tauri::image::Image::new_owned(battery_icon::render(state), battery_icon::SIZE, battery_icon::SIZE);
        let tip = match state {
            battery_icon::State::Off => "HS80 · desligado".to_string(),
            battery_icon::State::Charging(p) => format!("HS80 · {p:.0}% · carregando"),
            battery_icon::State::Discharging(p) => format!("HS80 · {p:.0}%"),
        };
        if let Some(tray) = app2.tray_by_id(BATTERY_TRAY) {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(&tip));
            if let Some(item) = BATTERY_ITEM.lock().unwrap().as_ref() {
                let _ = item.set_text(&tip);
            }
        } else {
            // A menu is required on Linux: AppIndicator may hide an icon without
            // one, and has no tooltip or click events, so the status lives there.
            let Ok(item) = MenuItem::with_id(&app2, "battery-status", &tip, false, None::<&str>) else { return };
            let Ok(open) = MenuItem::with_id(&app2, "battery-open", "Abrir fone", true, None::<&str>) else { return };
            let Ok(menu) = Menu::with_items(&app2, &[&item, &open]) else { return };
            *BATTERY_ITEM.lock().unwrap() = Some(item);
            let _ = TrayIconBuilder::with_id(BATTERY_TRAY)
                .icon(icon)
                .tooltip(&tip)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| {
                    if e.id.as_ref() == "battery-open" {
                        show_main(app);
                        let _ = app.emit("navigate", "headset");
                    }
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = e {
                        show_main(tray.app_handle());
                        let _ = tray.app_handle().emit("navigate", "headset");
                    }
                })
                .build(&app2);
        }
    });
}

// ---- Misc ----

#[tauri::command]
fn get_status(state: State<AppState>) -> Status {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
fn get_keymap() -> HashMap<String, Option<u16>> {
    serde_json::from_str(KEYMAP_BR).unwrap_or_default()
}

/// Key name -> LED slot, for painting and spatial effects.
#[tauri::command]
fn get_ledmap() -> HashMap<String, Option<u16>> {
    get_keymap()
        .into_iter()
        .map(|(name, key)| (name, key.and_then(|k| op_core::k70::led_for_key(k as u8)).map(|s| s as u16)))
        .collect()
}

/// Normalised key centres from the UI's layout, used by spatial effects (wave, rainbow).
#[tauri::command]
fn set_geometry(state: State<AppState>, geometry: HashMap<u16, (f32, f32)>) {
    let _ = state.engine.send(Cmd::SetGeometry(geometry as Geometry));
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

/// Launched by autostart: stay in the tray instead of opening the window.
const MINIMIZED_ARG: &str = "--minimized";

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Load profiles.json, or build the first profile from the pre-profile settings files.
fn load_store(dir: &std::path::Path) -> Store {
    if let Some(s) = fs::read_to_string(dir.join("profiles.json")).ok().and_then(|s| serde_json::from_str(&s).ok()) {
        return s;
    }
    let read = |f: &str| fs::read_to_string(dir.join(f)).ok();
    Store::new_default(
        read("lighting.json").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
        read("keys.json").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
        read("eq.json").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
    )
}

/// Switch to a game's profile while it is in front, back to the selected one after.
fn spawn_auto_switch(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1000));
        let Some(st) = app.try_state::<AppState>() else { continue };
        // Stay on an auto profile while our own window is in front (user is editing it).
        // On Wayland our window is invisible to the X11 lookup, so ask the window too.
        if app.get_webview_window("main").and_then(|w| w.is_focused().ok()).unwrap_or(false) {
            continue;
        }
        let names = foreground::names();
        if names.iter().any(|n| n.to_lowercase().starts_with("openperipheral")) {
            continue;
        }
        let (target, auto) = {
            let store = st.store.lock().unwrap();
            match names.iter().find_map(|n| store.for_app(n).map(|p| (p.id.clone(), n.clone()))) {
                Some((id, n)) => (id, Some(n)),
                None => (store.selected.clone(), None),
            }
        };
        if st.active_id() != target {
            activate(&app, &target, auto);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Logout, shutdown and `kill` send SIGTERM/SIGHUP: quit through the normal
/// exit path so the K70 gets its hardware-mode command. A keyboard left in
/// software mode with no host stops typing until it is replugged.
#[cfg(unix)]
fn exit_on_signals(app: AppHandle) -> std::io::Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    let mut signals = signal_hook::iterator::Signals::new([SIGTERM, SIGINT, SIGHUP])?;
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            app.exit(0);
        }
    });
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        // A second launch (e.g. from the Start menu) just brings the window back.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .setup(|app| {
            if !std::env::args().any(|a| a == MINIMIZED_ARG) {
                show_main(app.handle());
            }
            let config_dir = app.path().app_config_dir()?;
            fs::create_dir_all(&config_dir)?;
            let store = load_store(&config_dir);
            let start = store.get(&store.selected).cloned().unwrap_or_else(|| store.profiles[0].clone());

            let status_handle = app.handle().clone();
            let key_handle = app.handle().clone();
            let profile_handle = app.handle().clone();
            let wheel_handle = app.handle().clone();
            let bright_handle = app.handle().clone();
            let engine = engine::spawn(
                start.lighting.clone(),
                move |status| {
                    if let Some(st) = status_handle.try_state::<AppState>() {
                        *st.status.lock().unwrap() = status.clone();
                        // Win Lock and the mouse DPI stage change on the devices: remember them.
                        let mut s = st.settings.lock().unwrap();
                        let stage = status.mouse.as_ref().map_or(s.mouse_stage, |m| m.dpi_stage);
                        if s.win_lock != status.win_lock || s.mouse_stage != stage {
                            s.win_lock = status.win_lock;
                            s.mouse_stage = stage;
                            let _ = fs::write(&st.settings_path, serde_json::to_string_pretty(&*s).unwrap_or_default());
                        }
                    }
                    let _ = status_handle.emit("status", status);
                    update_battery_tray(&status_handle, status);
                },
                move |slot| {
                    let _ = key_handle.emit("key-press", slot);
                },
                // "Switch profile" key: runs off the engine thread.
                move |target| {
                    let app = profile_handle.clone();
                    thread::spawn(move || {
                        let Some(st) = app.try_state::<AppState>() else { return };
                        let id = if target == "next" {
                            st.store.lock().unwrap().next_after(&st.active_id())
                        } else {
                            target
                        };
                        st.store.lock().unwrap().selected = id.clone();
                        let _ = st.save();
                        activate(&app, &id, None);
                    });
                },
                // Headset volume-wheel click: next EQ preset (as in iCUE).
                move || {
                    let app = wheel_handle.clone();
                    thread::spawn(move || {
                        let Some(st) = app.try_state::<AppState>() else { return };
                        let id = st.active_id();
                        let Some(next) = st.store.lock().unwrap().get(&id).map(|p| p.eq.next_preset()) else { return };
                        let _ = eq::apply(&next);
                        let _ = st.edit_active(|p| p.eq = next.clone());
                        let _ = app.emit("eq", next);
                    });
                },
                // Brightness key: next level for the profile in effect.
                move || {
                    let app = bright_handle.clone();
                    thread::spawn(move || {
                        let Some(st) = app.try_state::<AppState>() else { return };
                        let id = st.active_id();
                        let Some(mut l) = st.store.lock().unwrap().get(&id).map(|p| p.lighting.clone()) else { return };
                        l.brightness = effects::next_brightness(l.brightness);
                        let _ = st.engine.send(Cmd::SetLighting(l.clone()));
                        let _ = st.edit_active(|p| p.lighting = l.clone());
                        let _ = app.emit("lighting", l);
                    });
                },
            );
            let _ = engine.send(Cmd::SetBindings(to_slots(&start.bindings)));
            let _ = engine.send(Cmd::SetMouseBindings(to_mouse_bits(&start.mouse_bindings)));
            let _ = eq::apply(&start.eq);
            let settings_path = config_dir.join("settings.json");
            let settings: AppSettings =
                fs::read_to_string(&settings_path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            settings.apply(&engine);
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                engine,
                active: Mutex::new(Active { id: start.id.clone(), auto_app: None }),
                store: Mutex::new(store),
                status: Mutex::new(Status::default()),
                store_path: config_dir.join("profiles.json"),
            });
            let _ = app.state::<AppState>().save();
            spawn_auto_switch(app.handle().clone());
            #[cfg(unix)]
            exit_on_signals(app.handle().clone())?;

            let open = MenuItem::with_id(app, "open", "Abrir OpenPeripheral", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().expect("icon"))
                .tooltip("OpenPeripheral")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = e {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps lighting running from the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_lighting,
            set_lighting,
            get_keymap,
            get_ledmap,
            set_geometry,
            get_autostart,
            set_autostart,
            get_eq,
            set_eq,
            eq_backend,
            get_bindings,
            set_bindings,
            get_profiles,
            select_profile,
            create_profile,
            update_profile,
            delete_profile,
            foreground_app,
            get_app_settings,
            set_app_settings,
            set_mouse_stage,
            install_kind,
            open_url,
            get_mouse_bindings,
            set_mouse_bindings
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Hand the keyboard back to its onboard lighting before quitting.
            if let Some(st) = app.try_state::<AppState>() {
                let (tx, rx) = mpsc::channel();
                if st.engine.send(Cmd::Shutdown(tx)).is_ok() {
                    let _ = rx.recv_timeout(Duration::from_secs(2));
                }
            }
            eq::shutdown();
        }
    });
}
