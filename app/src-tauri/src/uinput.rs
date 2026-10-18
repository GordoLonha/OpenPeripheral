//! Linux input injection through a uinput virtual keyboard + mouse. Unlike X11
//! injection this reaches every app on Wayland too. Needs write access to
//! /dev/uinput (see linux/70-openperipheral.rules).
//!
//! Characters are mapped to keys for the ABNT2 (br) layout, like the K70 itself;
//! accented letters are typed as dead key + letter.

use std::sync::{Mutex, OnceLock};

use enigo::{Axis, Button, Direction, Key};
use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, EventType, InputEvent, Key as K, RelativeAxisType,
};

/// One virtual device shared by the engine thread and macro threads: creating
/// a device per macro is slow and its first events get lost.
fn device() -> &'static Mutex<Option<VirtualDevice>> {
    static DEV: OnceLock<Mutex<Option<VirtualDevice>>> = OnceLock::new();
    DEV.get_or_init(|| Mutex::new(build().ok()))
}

fn build() -> std::io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<K>::new();
    // Every keyboard key plus the mouse buttons we emit.
    for code in 1..=248u16 {
        keys.insert(K::new(code));
    }
    for b in [K::BTN_LEFT, K::BTN_RIGHT, K::BTN_MIDDLE, K::BTN_SIDE, K::BTN_EXTRA] {
        keys.insert(b);
    }
    let mut rel = AttributeSet::<RelativeAxisType>::new();
    // X/Y make libinput treat it as a pointer, so buttons and wheel work.
    for a in [RelativeAxisType::REL_X, RelativeAxisType::REL_Y, RelativeAxisType::REL_WHEEL] {
        rel.insert(a);
    }
    VirtualDeviceBuilder::new()?
        .name("OpenPeripheral virtual input")
        .with_keys(&keys)?
        .with_relative_axes(&rel)?
        .build()
}

/// Whether the virtual device could be created.
pub fn available() -> bool {
    device().lock().map(|d| d.is_some()).unwrap_or(false)
}

fn emit(events: &[InputEvent]) {
    if let Ok(mut d) = device().lock() {
        if let Some(d) = d.as_mut() {
            let _ = d.emit(events);
        }
    }
}

fn key_event(k: K, down: bool) -> InputEvent {
    InputEvent::new(EventType::KEY, k.code(), down as i32)
}

/// A key press: the key and the modifiers it needs.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stroke {
    key: K,
    shift: bool,
    altgr: bool,
}

const fn plain(key: K) -> Stroke {
    Stroke { key, shift: false, altgr: false }
}
const fn shifted(key: K) -> Stroke {
    Stroke { key, shift: true, altgr: false }
}

/// Keys to type `c` on ABNT2: one stroke, or dead key + letter for accents.
fn strokes(c: char) -> Option<Vec<Stroke>> {
    if let Some(s) = base(c) {
        return Some(vec![s]);
    }
    // Accented letter = dead key, then the bare letter (shifted for capitals).
    let lower = c.to_lowercase().next()?;
    let (dead, letter) = match lower {
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' => (plain(K::KEY_LEFTBRACE), lower),
        'à' | 'è' | 'ì' | 'ò' | 'ù' => (shifted(K::KEY_LEFTBRACE), lower),
        'ã' | 'õ' | 'ñ' => (plain(K::KEY_APOSTROPHE), lower),
        'â' | 'ê' | 'î' | 'ô' | 'û' => (shifted(K::KEY_APOSTROPHE), lower),
        'ä' | 'ë' | 'ï' | 'ö' | 'ü' | 'ÿ' => (shifted(K::KEY_6), lower),
        _ => return None,
    };
    let bare = match letter {
        'á' | 'à' | 'ã' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'õ' | 'ô' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'ñ' => 'n',
        _ => return None,
    };
    let mut l = base(bare)?;
    l.shift = c.is_uppercase();
    Some(vec![dead, l])
}

/// Characters that are a single key on ABNT2.
fn base(c: char) -> Option<Stroke> {
    if c.is_ascii_alphabetic() {
        let k = letter_key(c.to_ascii_lowercase())?;
        return Some(Stroke { key: k, shift: c.is_ascii_uppercase(), altgr: false });
    }
    Some(match c {
        '1' => plain(K::KEY_1),
        '2' => plain(K::KEY_2),
        '3' => plain(K::KEY_3),
        '4' => plain(K::KEY_4),
        '5' => plain(K::KEY_5),
        '6' => plain(K::KEY_6),
        '7' => plain(K::KEY_7),
        '8' => plain(K::KEY_8),
        '9' => plain(K::KEY_9),
        '0' => plain(K::KEY_0),
        '!' => shifted(K::KEY_1),
        '@' => shifted(K::KEY_2),
        '#' => shifted(K::KEY_3),
        '$' => shifted(K::KEY_4),
        '%' => shifted(K::KEY_5),
        '&' => shifted(K::KEY_7),
        '*' => shifted(K::KEY_8),
        '(' => shifted(K::KEY_9),
        ')' => shifted(K::KEY_0),
        '-' => plain(K::KEY_MINUS),
        '_' => shifted(K::KEY_MINUS),
        '=' => plain(K::KEY_EQUAL),
        '+' => shifted(K::KEY_EQUAL),
        '\'' => plain(K::KEY_GRAVE),
        '"' => shifted(K::KEY_GRAVE),
        '[' => plain(K::KEY_RIGHTBRACE),
        '{' => shifted(K::KEY_RIGHTBRACE),
        ']' => plain(K::KEY_BACKSLASH),
        '}' => shifted(K::KEY_BACKSLASH),
        'ç' => plain(K::KEY_SEMICOLON),
        'Ç' => shifted(K::KEY_SEMICOLON),
        '\\' => plain(K::KEY_102ND),
        '|' => shifted(K::KEY_102ND),
        ',' => plain(K::KEY_COMMA),
        '<' => shifted(K::KEY_COMMA),
        '.' => plain(K::KEY_DOT),
        '>' => shifted(K::KEY_DOT),
        ';' => plain(K::KEY_SLASH),
        ':' => shifted(K::KEY_SLASH),
        '/' => plain(K::KEY_RO),
        '?' => shifted(K::KEY_RO),
        ' ' => plain(K::KEY_SPACE),
        '\n' => plain(K::KEY_ENTER),
        '\t' => plain(K::KEY_TAB),
        _ => return None,
    })
}

fn letter_key(c: char) -> Option<K> {
    Some(match c {
        'a' => K::KEY_A,
        'b' => K::KEY_B,
        'c' => K::KEY_C,
        'd' => K::KEY_D,
        'e' => K::KEY_E,
        'f' => K::KEY_F,
        'g' => K::KEY_G,
        'h' => K::KEY_H,
        'i' => K::KEY_I,
        'j' => K::KEY_J,
        'k' => K::KEY_K,
        'l' => K::KEY_L,
        'm' => K::KEY_M,
        'n' => K::KEY_N,
        'o' => K::KEY_O,
        'p' => K::KEY_P,
        'q' => K::KEY_Q,
        'r' => K::KEY_R,
        's' => K::KEY_S,
        't' => K::KEY_T,
        'u' => K::KEY_U,
        'v' => K::KEY_V,
        'w' => K::KEY_W,
        'x' => K::KEY_X,
        'y' => K::KEY_Y,
        'z' => K::KEY_Z,
        _ => return None,
    })
}

/// Named (non-character) keys.
fn named(k: Key) -> Option<K> {
    Some(match k {
        Key::Control => K::KEY_LEFTCTRL,
        Key::Shift => K::KEY_LEFTSHIFT,
        Key::Alt => K::KEY_LEFTALT,
        Key::Meta => K::KEY_LEFTMETA,
        Key::Escape => K::KEY_ESC,
        Key::Return => K::KEY_ENTER,
        Key::Tab => K::KEY_TAB,
        Key::Space => K::KEY_SPACE,
        Key::Backspace => K::KEY_BACKSPACE,
        Key::Delete => K::KEY_DELETE,
        Key::Insert => K::KEY_INSERT,
        Key::Home => K::KEY_HOME,
        Key::End => K::KEY_END,
        Key::PageUp => K::KEY_PAGEUP,
        Key::PageDown => K::KEY_PAGEDOWN,
        Key::UpArrow => K::KEY_UP,
        Key::DownArrow => K::KEY_DOWN,
        Key::LeftArrow => K::KEY_LEFT,
        Key::RightArrow => K::KEY_RIGHT,
        Key::CapsLock => K::KEY_CAPSLOCK,
        Key::PrintScr => K::KEY_SYSRQ,
        Key::Pause => K::KEY_PAUSE,
        Key::F1 => K::KEY_F1,
        Key::F2 => K::KEY_F2,
        Key::F3 => K::KEY_F3,
        Key::F4 => K::KEY_F4,
        Key::F5 => K::KEY_F5,
        Key::F6 => K::KEY_F6,
        Key::F7 => K::KEY_F7,
        Key::F8 => K::KEY_F8,
        Key::F9 => K::KEY_F9,
        Key::F10 => K::KEY_F10,
        Key::F11 => K::KEY_F11,
        Key::F12 => K::KEY_F12,
        Key::F13 => K::KEY_F13,
        Key::F14 => K::KEY_F14,
        Key::F15 => K::KEY_F15,
        Key::F16 => K::KEY_F16,
        Key::F17 => K::KEY_F17,
        Key::F18 => K::KEY_F18,
        Key::F19 => K::KEY_F19,
        Key::F20 => K::KEY_F20,
        Key::MediaPlayPause => K::KEY_PLAYPAUSE,
        Key::MediaNextTrack => K::KEY_NEXTSONG,
        Key::MediaPrevTrack => K::KEY_PREVIOUSSONG,
        Key::MediaStop => K::KEY_STOPCD,
        Key::VolumeUp => K::KEY_VOLUMEUP,
        Key::VolumeDown => K::KEY_VOLUMEDOWN,
        Key::VolumeMute => K::KEY_MUTE,
        _ => return None,
    })
}

fn sequence(k: Key) -> Option<Vec<Stroke>> {
    match k {
        Key::Unicode(c) => strokes(c),
        k => named(k).map(|n| vec![plain(n)]),
    }
}

fn stroke_events(s: Stroke, down: bool, out: &mut Vec<InputEvent>) {
    let mods = [(s.shift, K::KEY_LEFTSHIFT), (s.altgr, K::KEY_RIGHTALT)];
    if down {
        for (on, m) in mods {
            if on {
                out.push(key_event(m, true));
            }
        }
        out.push(key_event(s.key, true));
    } else {
        out.push(key_event(s.key, false));
        for (on, m) in mods.into_iter().rev() {
            if on {
                out.push(key_event(m, false));
            }
        }
    }
}

/// Same calls as enigo's Keyboard/Mouse traits, so `remap` uses either.
pub struct Uinput;

impl Uinput {
    pub fn new() -> Option<Self> {
        available().then_some(Self)
    }

    pub fn key(&mut self, k: Key, dir: Direction) -> Result<(), ()> {
        let seq = sequence(k).ok_or(())?;
        // Dead-key sequences cannot be held: type them whole on press.
        let dir = if seq.len() > 1 && dir != Direction::Release { Direction::Click } else { dir };
        if seq.len() > 1 && dir == Direction::Release {
            return Ok(());
        }
        // Each stroke goes in its own report: some apps miss a key pressed
        // and released within the same SYN frame.
        for s in &seq {
            if matches!(dir, Direction::Press | Direction::Click) {
                let mut ev = Vec::new();
                stroke_events(*s, true, &mut ev);
                emit(&ev);
            }
            if matches!(dir, Direction::Release | Direction::Click) {
                let mut ev = Vec::new();
                stroke_events(*s, false, &mut ev);
                emit(&ev);
            }
        }
        Ok(())
    }

    pub fn text(&mut self, text: &str) -> Result<(), ()> {
        for c in text.chars() {
            let _ = self.key(Key::Unicode(c), Direction::Click);
        }
        Ok(())
    }

    pub fn button(&mut self, b: Button, dir: Direction) -> Result<(), ()> {
        let k = match b {
            Button::Left => K::BTN_LEFT,
            Button::Right => K::BTN_RIGHT,
            Button::Middle => K::BTN_MIDDLE,
            Button::Back => K::BTN_SIDE,
            Button::Forward => K::BTN_EXTRA,
            _ => return Err(()),
        };
        if matches!(dir, Direction::Press | Direction::Click) {
            emit(&[key_event(k, true)]);
        }
        if matches!(dir, Direction::Release | Direction::Click) {
            emit(&[key_event(k, false)]);
        }
        Ok(())
    }

    /// enigo convention: negative = up. REL_WHEEL is positive up.
    pub fn scroll(&mut self, length: i32, axis: Axis) -> Result<(), ()> {
        if axis != Axis::Vertical {
            return Err(());
        }
        emit(&[InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, -length)]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abnt2_chars() {
        assert_eq!(strokes('a'), Some(vec![plain(K::KEY_A)]));
        assert_eq!(strokes('A'), Some(vec![shifted(K::KEY_A)]));
        assert_eq!(strokes('ç'), Some(vec![plain(K::KEY_SEMICOLON)]));
        assert_eq!(strokes('/'), Some(vec![plain(K::KEY_RO)]));
        assert_eq!(strokes('ã'), Some(vec![plain(K::KEY_APOSTROPHE), plain(K::KEY_A)]));
        assert_eq!(strokes('É'), Some(vec![plain(K::KEY_LEFTBRACE), shifted(K::KEY_E)]));
        assert_eq!(strokes('ô'), Some(vec![shifted(K::KEY_APOSTROPHE), plain(K::KEY_O)]));
        assert_eq!(strokes('€'), None);
    }

    #[test]
    fn named_keys() {
        assert_eq!(sequence(Key::Control), Some(vec![plain(K::KEY_LEFTCTRL)]));
        assert_eq!(sequence(Key::VolumeUp), Some(vec![plain(K::KEY_VOLUMEUP)]));
    }
}
