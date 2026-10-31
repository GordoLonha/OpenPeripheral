//! Key remapping: a remapped key is put in host-only mode on the keyboard (the
//! OS no longer sees it) and its action is produced here through the OS input
//! API: enigo (SendInput) on Windows, a uinput virtual device on Linux (works
//! on Wayland, where X11 injection only reaches XWayland windows).

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(not(target_os = "linux"))]
use enigo::{Enigo, Keyboard, Mouse, Settings};
use enigo::{Axis, Button, Direction, Key};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Action {
    /// Key does nothing.
    Disabled,
    /// Acts as another key (held while the physical key is held).
    Key { key: String },
    /// Key combination, e.g. ["Control", "Shift", "Escape"] (held while held).
    Shortcut { keys: Vec<String> },
    /// Types a text on press.
    Text { text: String },
    /// Starts a program / opens a file or URL on press.
    Launch { target: String },
    /// Media / volume key on press: "playpause", "next", "prev", "stop",
    /// "volup", "voldown", "mute".
    Media { action: String },
    /// Mouse button held while the key is held ("left", "right", "middle",
    /// "back", "forward"), or a scroll step on press ("scrollup", "scrolldown").
    Mouse { button: String },
    /// Recorded sequence of keys, mouse buttons and wheel steps, played on its
    /// own thread as `options` say (iCUE's macro settings).
    Macro {
        steps: Vec<MacroStep>,
        #[serde(default)]
        options: MacroOptions,
    },
    /// Switch profile on press: a profile id, or "next" to cycle.
    /// Handled by the engine (needs the profile store), not the injector.
    Profile { target: String },
}

/// When a macro starts (iCUE "Acionamento de ação").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// On press ("Ao pressionar").
    #[default]
    Press,
    /// On release ("Ao soltar").
    Release,
    /// While held; releasing stops it ("Quando pressionado").
    Hold,
    /// First press starts, next press stops ("Alternar").
    Toggle,
}

/// How often it runs (iCUE "Repetição de ação").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    #[default]
    Once,
    /// `times` runs in a row.
    Times,
    /// Until stopped: release (Hold), next press (Press / Toggle).
    Continuous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MacroOptions {
    pub trigger: Trigger,
    pub repeat: Repeat,
    pub times: u32,
    /// A new press does not restart a running macro ("Executar ininterruptamente").
    pub uninterruptible: bool,
    /// The key keeps its own output too ("Manter a saída de tecla original").
    pub passthrough: bool,
    /// Sound file played when the macro starts.
    pub sound: Option<String>,
}

impl Default for MacroOptions {
    fn default() -> Self {
        Self { trigger: Trigger::Press, repeat: Repeat::Once, times: 2, uninterruptible: false, passthrough: false, sound: None }
    }
}

impl Action {
    /// Whether the key/button keeps reaching the OS (not put in host-only mode).
    pub fn passthrough(&self) -> bool {
        matches!(self, Action::Macro { options, .. } if options.passthrough)
    }
}

/// A step is a key name (as `key_from_name`), `Mouse:<button>` or `Wheel:up|down`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroStep {
    pub key: String,
    pub down: bool,
    /// Pause before this step.
    pub delay_ms: u32,
}

fn mouse_button(name: &str) -> Option<Button> {
    Some(match name {
        "left" => Button::Left,
        "right" => Button::Right,
        "middle" => Button::Middle,
        "back" => Button::Back,
        "forward" => Button::Forward,
        _ => return None,
    })
}

#[cfg(not(target_os = "linux"))]
type Out = Enigo;
#[cfg(target_os = "linux")]
type Out = crate::uinput::Uinput;

fn new_out() -> Option<Out> {
    #[cfg(not(target_os = "linux"))]
    {
        Enigo::new(&Settings::default()).ok()
    }
    #[cfg(target_os = "linux")]
    {
        crate::uinput::Uinput::new()
    }
}

/// Sleep up to `ms`, waking early when the macro is stopped.
fn wait(ms: u32, run: &AtomicBool) -> bool {
    let mut left = ms.min(60_000) as u64;
    while left > 0 && run.load(Ordering::Relaxed) {
        let step = left.min(20);
        std::thread::sleep(Duration::from_millis(step));
        left -= step;
    }
    run.load(Ordering::Relaxed)
}

#[derive(PartialEq)]
enum Held {
    Key(Key),
    Button(Button),
}

/// One pass over the steps; false if stopped midway.
fn play_steps(e: &mut Out, steps: &[MacroStep], run: &AtomicBool, held: &mut Vec<Held>) -> bool {
    for s in steps {
        if s.delay_ms > 0 && !wait(s.delay_ms, run) {
            return false;
        }
        if !run.load(Ordering::Relaxed) {
            return false;
        }
        let dir = if s.down { Direction::Press } else { Direction::Release };
        let item = if let Some(b) = s.key.strip_prefix("Mouse:").and_then(mouse_button) {
            let _ = e.button(b, dir);
            Held::Button(b)
        } else if let Some(w) = s.key.strip_prefix("Wheel:") {
            if s.down {
                let _ = e.scroll(if w == "up" { -1 } else { 1 }, Axis::Vertical);
            }
            continue;
        } else if let Some(k) = key_from_name(&s.key) {
            let _ = e.key(k, dir);
            Held::Key(k)
        } else {
            continue;
        };
        if s.down { held.push(item) } else { held.retain(|h| *h != item) }
    }
    true
}

fn run_macro(steps: Vec<MacroStep>, o: MacroOptions, run: Arc<AtomicBool>) {
    let Some(mut e) = new_out() else {
        run.store(false, Ordering::Relaxed);
        return;
    };
    let mut held = Vec::new();
    let mut done = 0;
    while play_steps(&mut e, &steps, &run, &mut held) {
        done += 1;
        let more = match o.repeat {
            Repeat::Once => false,
            Repeat::Times => done < o.times.max(1),
            Repeat::Continuous => true,
        };
        // A macro of zero-length steps must not spin a core when repeated.
        if !more || !wait(10, &run) {
            break;
        }
    }
    // Never leave a key or button stuck down if the macro was cut mid-press.
    for h in held {
        let _ = match h {
            Held::Key(k) => e.key(k, Direction::Release),
            Held::Button(b) => e.button(b, Direction::Release),
        };
    }
    run.store(false, Ordering::Relaxed);
}

fn play_sound(path: &str) {
    let path = path.trim();
    if path.is_empty() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let script = format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path.replace('\'', "''"));
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        if std::process::Command::new("pw-play").arg(path).spawn().is_err() {
            let _ = std::process::Command::new("paplay").arg(path).spawn();
        }
    }
}

/// Key id (as in the keymap, e.g. "F12") -> action.
pub type Bindings = HashMap<String, Action>;

/// Names offered by the UI for "acts as key" / shortcuts.
pub fn key_from_name(name: &str) -> Option<Key> {
    let k = match name {
        "Control" => Key::Control,
        "Shift" => Key::Shift,
        "Alt" => Key::Alt,
        "Meta" => Key::Meta,
        "Escape" => Key::Escape,
        "Enter" => Key::Return,
        "Tab" => Key::Tab,
        "Space" => Key::Space,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Insert" => Key::Insert,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Up" => Key::UpArrow,
        "Down" => Key::DownArrow,
        "Left" => Key::LeftArrow,
        "Right" => Key::RightArrow,
        "CapsLock" => Key::CapsLock,
        "PrintScreen" => Key::PrintScr,
        "Pause" => Key::Pause,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "F13" => Key::F13,
        "F14" => Key::F14,
        "F15" => Key::F15,
        "F16" => Key::F16,
        "F17" => Key::F17,
        "F18" => Key::F18,
        "F19" => Key::F19,
        "F20" => Key::F20,
        s => {
            // Single printable character: letters, digits, punctuation, ç…
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Unicode(c.to_lowercase().next().unwrap_or(c)),
                _ => return None,
            }
        }
    };
    Some(k)
}

fn media_key(action: &str) -> Option<Key> {
    Some(match action {
        "playpause" => Key::MediaPlayPause,
        "next" => Key::MediaNextTrack,
        "prev" => Key::MediaPrevTrack,
        "stop" => Key::MediaStop,
        "volup" => Key::VolumeUp,
        "voldown" => Key::VolumeDown,
        "mute" => Key::VolumeMute,
        _ => return None,
    })
}

/// Runs actions; lives on the engine thread.
pub struct Injector {
    enigo: Option<Out>,
    /// Running macro per key/button id; the flag is cleared to stop it.
    running: HashMap<String, Arc<AtomicBool>>,
    /// Key, shortcut or mouse-button action held down per key/button id. Released from
    /// here, not from the current bindings: those may have changed (profile switch)
    /// while the key was down, and the OS would keep the old key pressed.
    held: HashMap<String, Action>,
}

impl Injector {
    pub fn new() -> Self {
        Self { enigo: new_out(), running: HashMap::new(), held: HashMap::new() }
    }

    fn macro_running(&self, id: &str) -> bool {
        self.running.get(id).is_some_and(|f| f.load(Ordering::Relaxed))
    }

    fn stop_macro(&mut self, id: &str) {
        if let Some(f) = self.running.remove(id) {
            f.store(false, Ordering::Relaxed);
        }
    }

    fn start_macro(&mut self, id: &str, steps: &[MacroStep], o: &MacroOptions) {
        if self.macro_running(id) {
            if o.uninterruptible {
                return;
            }
            self.stop_macro(id);
        }
        let run = Arc::new(AtomicBool::new(true));
        self.running.insert(id.to_string(), run.clone());
        if let Some(sound) = &o.sound {
            play_sound(sound);
        }
        let (steps, o) = (steps.to_vec(), o.clone());
        std::thread::spawn(move || run_macro(steps, o, run));
    }

    /// Press/Release triggers: a running endless macro is stopped by the next one.
    fn fire_macro(&mut self, id: &str, steps: &[MacroStep], o: &MacroOptions) {
        if o.repeat == Repeat::Continuous && self.macro_running(id) {
            self.stop_macro(id);
        } else {
            self.start_macro(id, steps, o);
        }
    }

    /// Stop every macro and let go of every held key (profile switch, bindings changed).
    pub fn stop_all(&mut self) {
        for (_, f) in self.running.drain() {
            f.store(false, Ordering::Relaxed);
        }
        self.release_where(|_| true);
    }

    /// Let go of the keys held by ids matching `which` (a device went away).
    pub fn release_where(&mut self, which: impl Fn(&str) -> bool) {
        let ids: Vec<String> = self.held.keys().filter(|id| which(id)).cloned().collect();
        for id in ids {
            self.release_held(&id);
        }
    }

    fn release_held(&mut self, id: &str) {
        match self.held.remove(id) {
            Some(Action::Key { key }) => {
                if let Some(k) = key_from_name(&key) {
                    self.keys(&[k], Direction::Release);
                }
            }
            Some(Action::Shortcut { keys }) => {
                let ks: Vec<Key> = keys.iter().filter_map(|k| key_from_name(k)).collect();
                self.keys(&ks, Direction::Release);
            }
            Some(Action::Mouse { button }) => self.mouse(&button, Direction::Release),
            _ => {}
        }
    }

    fn keys(&mut self, keys: &[Key], dir: Direction) {
        let Some(e) = self.enigo.as_mut() else { return };
        // Press modifiers first, release them last.
        let ordered: Vec<&Key> = if dir == Direction::Release { keys.iter().rev().collect() } else { keys.iter().collect() };
        for k in ordered {
            let _ = e.key(*k, dir);
        }
    }

    /// `id` names the physical key/button, so its macro can be stopped later.
    pub fn press(&mut self, id: &str, action: &Action) {
        if matches!(action, Action::Key { .. } | Action::Shortcut { .. } | Action::Mouse { .. }) {
            // A press without its release (key bound twice in a row): let go first.
            self.release_held(id);
            self.held.insert(id.to_string(), action.clone());
        }
        match action {
            Action::Disabled => {}
            Action::Key { key } => {
                if let Some(k) = key_from_name(key) {
                    self.keys(&[k], Direction::Press);
                }
            }
            Action::Shortcut { keys } => {
                let ks: Vec<Key> = keys.iter().filter_map(|k| key_from_name(k)).collect();
                self.keys(&ks, Direction::Press);
            }
            Action::Text { text } => {
                if let Some(e) = self.enigo.as_mut() {
                    let _ = e.text(text);
                }
            }
            Action::Launch { target } => launch(target),
            Action::Media { action } => {
                if let Some(k) = media_key(action) {
                    self.keys(&[k], Direction::Click);
                }
            }
            Action::Mouse { button } => self.mouse(button, Direction::Press),
            Action::Macro { steps, options: o } => match o.trigger {
                Trigger::Press => self.fire_macro(id, steps, o),
                Trigger::Release => {}
                Trigger::Hold => self.start_macro(id, steps, o),
                Trigger::Toggle => {
                    if self.macro_running(id) {
                        self.stop_macro(id);
                    } else {
                        self.start_macro(id, steps, o);
                    }
                }
            },
            Action::Profile { .. } => {}
        }
    }

    fn mouse(&mut self, button: &str, dir: Direction) {
        let Some(e) = self.enigo.as_mut() else { return };
        match (button, dir) {
            ("scrollup", Direction::Press) => drop(e.scroll(-3, Axis::Vertical)),
            ("scrolldown", Direction::Press) => drop(e.scroll(3, Axis::Vertical)),
            (b, d) => {
                if let Some(btn) = mouse_button(b) {
                    let _ = e.button(btn, d);
                }
            }
        }
    }

    pub fn release(&mut self, id: &str, action: &Action) {
        self.release_held(id);
        if let Action::Macro { steps, options: o } = action {
            match o.trigger {
                Trigger::Release => self.fire_macro(id, steps, o),
                Trigger::Hold => self.stop_macro(id),
                _ => {}
            }
        }
    }
}

pub(crate) fn launch(target: &str) {
    let target = target.trim();
    if target.is_empty() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // `start` resolves programs, documents and URLs like the Run dialog does.
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
        let Some((program, args)) = unix_command(target, &home, in_path) else { return };
        let child = std::process::Command::new(&program)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        // Reap it, or every launch stays a zombie until the app quits.
        if let Ok(mut child) = child {
            std::thread::spawn(move || child.wait());
        }
    }
}

/// What to run on Linux for a launch target.
///
/// The presets in the UI are named after Windows programs (`calc`, `notepad`,
/// `explorer`, `ms-settings:`) so profiles move between systems; here they become the
/// desktop's own calculator, text editor, file manager and settings. Otherwise a
/// program on the PATH runs with its arguments, and anything else (URL, file,
/// folder) goes to `xdg-open`, which cannot start programs by itself.
#[cfg(not(windows))]
fn unix_command(target: &str, home: &str, exists: impl Fn(&str) -> bool) -> Option<(String, Vec<String>)> {
    let first = |names: &[&str]| names.iter().find(|n| exists(n)).map(|n| (n.to_string(), vec![]));
    match target.to_ascii_lowercase().as_str() {
        "calc" | "calc.exe" => {
            return first(&["gnome-calculator", "kcalc", "qalculate-gtk", "galculator", "mate-calc", "xcalc"]);
        }
        "notepad" | "notepad.exe" => {
            return first(&["gnome-text-editor", "kwrite", "kate", "gedit", "mousepad", "xed", "pluma", "featherpad"]);
        }
        "ms-settings:" => {
            return first(&["systemsettings", "gnome-control-center", "xfce4-settings-manager", "cinnamon-settings", "mate-control-center"]);
        }
        "explorer" | "explorer.exe" => return Some(("xdg-open".into(), vec![home.to_string()])),
        _ => {}
    }
    // The first word decides: a program (on the PATH, or a path to an executable) runs
    // with the rest as arguments. A URL has a '/' and is not an executable file.
    let mut words = target.split_whitespace();
    if let Some(program) = words.next() {
        let runnable = if program.contains('/') { is_executable(std::path::Path::new(program)) } else { exists(program) };
        if runnable {
            return Some((program.to_string(), words.map(str::to_string).collect()));
        }
    }
    Some(("xdg-open".into(), vec![target.to_string()]))
}

#[cfg(not(windows))]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(windows))]
fn in_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|dir| is_executable(&dir.join(program))))
}

#[cfg(test)]
mod held_tests {
    use super::*;

    fn injector() -> Injector {
        Injector { enigo: None, running: HashMap::new(), held: HashMap::new() }
    }

    #[test]
    fn release_uses_the_action_that_was_pressed() {
        let mut i = injector();
        i.press("k5", &Action::Key { key: "Control".into() });
        assert!(i.held.contains_key("k5"));
        // Bindings changed while the key was down: the release still clears what was pressed.
        i.release("k5", &Action::Disabled);
        assert!(i.held.is_empty());
    }

    #[test]
    fn stop_all_and_device_loss_let_go() {
        let mut i = injector();
        i.press("k5", &Action::Shortcut { keys: vec!["Control".into(), "C".into()] });
        i.press("mback", &Action::Mouse { button: "left".into() });
        i.press("k6", &Action::Text { text: "x".into() });
        assert_eq!(i.held.len(), 2, "only keys and buttons stay down");
        i.release_where(|id| id.starts_with('m'));
        assert_eq!(i.held.keys().collect::<Vec<_>>(), ["k5"]);
        i.stop_all();
        assert!(i.held.is_empty());
    }
}

#[cfg(all(test, not(windows)))]
mod launch_tests {
    use super::unix_command;

    fn cmd(target: &str, installed: &[&str]) -> Option<(String, Vec<String>)> {
        unix_command(target, "/home/u", |p| installed.contains(&p))
    }

    #[test]
    fn presets_map_to_desktop_programs() {
        assert_eq!(cmd("calc", &["kcalc"]), Some(("kcalc".into(), vec![])));
        assert_eq!(cmd("notepad", &["kate", "gedit"]), Some(("kate".into(), vec![])));
        assert_eq!(cmd("ms-settings:", &["gnome-control-center"]), Some(("gnome-control-center".into(), vec![])));
        assert_eq!(cmd("explorer", &[]), Some(("xdg-open".into(), vec!["/home/u".into()])));
        assert_eq!(cmd("calc", &[]), None, "no calculator installed: do nothing");
    }

    #[test]
    fn programs_urls_and_files() {
        assert_eq!(cmd("steam steam://rungameid/730", &["steam"]), Some(("steam".into(), vec!["steam://rungameid/730".into()])));
        assert_eq!(cmd("https://example.com", &[]), Some(("xdg-open".into(), vec!["https://example.com".into()])));
        assert_eq!(cmd("mailto:a@b.c", &[]), Some(("xdg-open".into(), vec!["mailto:a@b.c".into()])));
        assert_eq!(cmd("/home/u/notes.txt", &[]), Some(("xdg-open".into(), vec!["/home/u/notes.txt".into()])));
        assert_eq!(cmd("/bin/sh -c true", &[]), Some(("/bin/sh".into(), vec!["-c".into(), "true".into()])));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve() {
        assert_eq!(key_from_name("B"), Some(Key::Unicode('b')));
        assert_eq!(key_from_name("Ç"), Some(Key::Unicode('ç')));
        assert_eq!(key_from_name("Control"), Some(Key::Control));
        assert_eq!(key_from_name("NotAKey"), None);
    }

    #[test]
    fn bindings_roundtrip_json() {
        let mut b = Bindings::new();
        b.insert("F12".into(), Action::Key { key: "B".into() });
        b.insert("CapsLock".into(), Action::Disabled);
        b.insert("F9".into(), Action::Launch { target: "calc".into() });
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(serde_json::from_str::<Bindings>(&json).unwrap(), b);
    }
}

#[cfg(test)]
mod macro_tests {
    use super::*;

    #[test]
    fn old_macros_load_with_default_options() {
        let a: Action = serde_json::from_str(r#"{"kind":"macro","steps":[{"key":"a","down":true,"delay_ms":0}]}"#).unwrap();
        let Action::Macro { options, .. } = &a else { panic!() };
        assert_eq!(options, &MacroOptions::default());
        assert!(!a.passthrough());
        let b: Action =
            serde_json::from_str(r#"{"kind":"macro","steps":[],"options":{"trigger":"toggle","repeat":"continuous","passthrough":true}}"#)
                .unwrap();
        let Action::Macro { options, .. } = &b else { panic!() };
        assert_eq!((options.trigger, options.repeat, options.times), (Trigger::Toggle, Repeat::Continuous, 2));
        assert!(b.passthrough());
    }

    #[test]
    fn wait_stops_early() {
        let run = AtomicBool::new(false);
        let t = std::time::Instant::now();
        assert!(!wait(5_000, &run));
        assert!(t.elapsed() < Duration::from_millis(100));
    }
}
