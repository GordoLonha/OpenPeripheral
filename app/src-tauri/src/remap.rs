//! Key remapping: a remapped key is put in host-only mode on the keyboard (the
//! OS no longer sees it) and its action is produced here through the OS input
//! API: enigo (SendInput) on Windows, a uinput virtual device on Linux (works
//! on Wayland, where X11 injection only reaches XWayland windows).

use std::collections::HashMap;

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
    /// Recorded key sequence, played on press (on its own thread).
    Macro { steps: Vec<MacroStep> },
    /// Switch profile on press: a profile id, or "next" to cycle.
    /// Handled by the engine (needs the profile store), not the injector.
    Profile { target: String },
}

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

fn play_macro(steps: Vec<MacroStep>) {
    std::thread::spawn(move || {
        let Some(mut e) = new_out() else { return };
        let mut held: Vec<Key> = Vec::new();
        for s in steps {
            if s.delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(s.delay_ms.min(10_000) as u64));
            }
            if let Some(k) = key_from_name(&s.key) {
                let _ = e.key(k, if s.down { Direction::Press } else { Direction::Release });
                if s.down { held.push(k) } else { held.retain(|h| *h != k) }
            }
        }
        // Never leave a key stuck down if the recording was cut mid-press.
        for k in held {
            let _ = e.key(k, Direction::Release);
        }
    });
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
}

impl Injector {
    pub fn new() -> Self {
        Self { enigo: new_out() }
    }

    fn keys(&mut self, keys: &[Key], dir: Direction) {
        let Some(e) = self.enigo.as_mut() else { return };
        // Press modifiers first, release them last.
        let ordered: Vec<&Key> = if dir == Direction::Release { keys.iter().rev().collect() } else { keys.iter().collect() };
        for k in ordered {
            let _ = e.key(*k, dir);
        }
    }

    pub fn press(&mut self, action: &Action) {
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
            Action::Macro { steps } => play_macro(steps.clone()),
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

    pub fn release(&mut self, action: &Action) {
        match action {
            Action::Key { key } => {
                if let Some(k) = key_from_name(key) {
                    self.keys(&[k], Direction::Release);
                }
            }
            Action::Shortcut { keys } => {
                let ks: Vec<Key> = keys.iter().filter_map(|k| key_from_name(k)).collect();
                self.keys(&ks, Direction::Release);
            }
            Action::Mouse { button } => self.mouse(button, Direction::Release),
            _ => {}
        }
    }
}

fn launch(target: &str) {
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
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
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
