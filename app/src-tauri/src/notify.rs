//! Desktop notifications: headset battery and automatic profile switches.
//! What to say is decided here (pure, tested); `show` hands it to the system.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Battery percent that gets the "low" notice, and the "critical" one.
pub const LOW: f32 = 20.0;
pub const CRITICAL: f32 = 10.0;
/// Above LOW + REARM the low notices can be sent again (the reading wobbles).
const REARM: f32 = 5.0;
/// At or above this, on the cable, the headset counts as charged.
const FULL: f32 = 99.5;

#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    BatteryLow(u8),
    BatteryCritical(u8),
    Charged,
    /// Profile name, program that turned it on.
    ProfileAuto(String, String),
}

impl Note {
    pub fn text(&self) -> (String, String) {
        match self {
            Note::BatteryLow(p) => ("Bateria do headset fraca".into(), format!("{p}% restantes. Conecte o cabo USB para carregar.")),
            Note::BatteryCritical(p) => ("Bateria do headset acabando".into(), format!("Só {p}%. O headset vai desligar em breve.")),
            Note::Charged => ("Headset carregado".into(), "A bateria está em 100%. Já pode tirar o cabo.".into()),
            Note::ProfileAuto(name, app) => (format!("Perfil {name} ativado"), format!("Porque {app} está na frente.")),
        }
    }
}

/// Remembers which battery notices were sent, so each one comes once.
#[derive(Debug, Default)]
pub struct BatteryWatch {
    low_sent: bool,
    critical_sent: bool,
    full_sent: bool,
}

impl BatteryWatch {
    /// `battery` is None while the headset is off; `plugged` is the USB cable.
    pub fn update(&mut self, battery: Option<f32>, plugged: bool) -> Option<Note> {
        let pct = battery?;
        if plugged {
            self.low_sent = false;
            self.critical_sent = false;
            if pct >= FULL && !self.full_sent {
                self.full_sent = true;
                return Some(Note::Charged);
            }
            return None;
        }
        self.full_sent = false;
        if pct > LOW + REARM {
            self.low_sent = false;
            self.critical_sent = false;
            return None;
        }
        let shown = pct.round().clamp(0.0, 100.0) as u8;
        if pct <= CRITICAL && !self.critical_sent {
            self.critical_sent = true;
            self.low_sent = true;
            return Some(Note::BatteryCritical(shown));
        }
        if pct <= LOW && !self.low_sent {
            self.low_sent = true;
            return Some(Note::BatteryLow(shown));
        }
        None
    }
}

pub fn show(app: &AppHandle, note: &Note) {
    let (title, body) = note.text();
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_then_critical_once_each() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.update(Some(50.0), false), None);
        assert_eq!(w.update(Some(20.0), false), Some(Note::BatteryLow(20)));
        assert_eq!(w.update(Some(18.0), false), None);
        assert_eq!(w.update(Some(9.6), false), Some(Note::BatteryCritical(10)));
        assert_eq!(w.update(Some(5.0), false), None);
    }

    #[test]
    fn starting_very_low_gives_only_the_critical_notice() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.update(Some(7.0), false), Some(Note::BatteryCritical(7)));
        assert_eq!(w.update(Some(6.0), false), None);
    }

    #[test]
    fn wobble_does_not_repeat_but_a_real_recharge_rearms() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.update(Some(19.0), false), Some(Note::BatteryLow(19)));
        assert_eq!(w.update(Some(21.0), false), None);
        assert_eq!(w.update(Some(19.0), false), None);
        assert_eq!(w.update(Some(40.0), false), None);
        assert_eq!(w.update(Some(19.0), false), Some(Note::BatteryLow(19)));
    }

    #[test]
    fn plugging_in_rearms_and_full_is_told_once() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.update(Some(15.0), false), Some(Note::BatteryLow(15)));
        assert_eq!(w.update(Some(15.0), true), None);
        assert_eq!(w.update(Some(100.0), true), Some(Note::Charged));
        assert_eq!(w.update(Some(100.0), true), None);
        assert_eq!(w.update(Some(100.0), false), None);
        assert_eq!(w.update(Some(100.0), true), Some(Note::Charged));
    }

    #[test]
    fn headset_off_changes_nothing() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.update(Some(18.0), false), Some(Note::BatteryLow(18)));
        assert_eq!(w.update(None, false), None);
        assert_eq!(w.update(Some(18.0), false), None);
    }

    #[test]
    fn texts_are_portuguese() {
        assert_eq!(Note::ProfileAuto("FPS".into(), "cs2".into()).text().0, "Perfil FPS ativado");
        assert!(Note::BatteryLow(20).text().1.starts_with("20%"));
    }
}
