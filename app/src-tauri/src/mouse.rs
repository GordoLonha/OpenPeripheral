//! M65 settings kept in settings.json: DPI stages, sniper, polling, angle
//! snapping, button response optimization and per-zone lighting.

use op_core::{m65, Rgb};
use serde::{Deserialize, Serialize};

use crate::effects::{self, Effect, Lighting};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DpiStage {
    pub x: u16,
    pub y: u16,
    pub color: [u8; 3],
}

impl DpiStage {
    const fn new(dpi: u16, color: [u8; 3]) -> Self {
        Self { x: dpi, y: dpi, color }
    }

    pub fn rgb(&self) -> Rgb {
        Rgb::new(self.color[0], self.color[1], self.color[2])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseLighting {
    /// Logo and wheel follow the keyboard's effect.
    pub follow_keyboard: bool,
    pub logo: Lighting,
    pub wheel: Lighting,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseOptions {
    pub stages: [DpiStage; m65::DPI_STAGES],
    pub sniper: DpiStage,
    /// Report interval: 1, 2, 4 or 8 ms.
    pub polling_ms: u8,
    pub angle_snapping: bool,
    pub button_optimization: bool,
    pub lighting: MouseLighting,
}

impl Default for MouseOptions {
    /// iCUE's defaults for the M65 RGB Elite.
    fn default() -> Self {
        let zone = |color: [u8; 3]| Lighting { effect: Effect::Static, color, ..Lighting::default() };
        Self {
            stages: [
                DpiStage::new(800, [255, 0, 0]),
                DpiStage::new(1500, [255, 255, 255]),
                DpiStage::new(3000, [0, 255, 0]),
                DpiStage::new(6000, [128, 0, 128]),
                DpiStage::new(9000, [0, 191, 255]),
            ],
            sniper: DpiStage::new(400, [255, 255, 0]),
            polling_ms: 1,
            angle_snapping: false,
            button_optimization: true,
            lighting: MouseLighting { follow_keyboard: true, logo: zone([0, 190, 255]), wheel: zone([0, 190, 255]) },
        }
    }
}

impl MouseOptions {
    /// Logo and wheel colours at time `t`.
    pub fn zones(&self, keyboard: &Lighting, t: f32) -> (Rgb, Rgb) {
        if self.lighting.follow_keyboard {
            let c = effects::headset(keyboard, t);
            (c, c)
        } else {
            (effects::headset(&self.lighting.logo, t), effects::headset(&self.lighting.wheel, t))
        }
    }

    /// Indicator colour: the sniper's while it is held, else the active stage's.
    pub fn dpi_color(&self, stage: u8, sniper: bool) -> Rgb {
        if sniper {
            self.sniper.rgb()
        } else {
            self.stages[(stage as usize).min(m65::DPI_STAGES - 1)].rgb()
        }
    }

    /// Everything but polling (which re-enumerates the mouse) and lighting.
    pub fn apply(&self, m: &m65::M65, stage: u8) {
        for (i, s) in self.stages.iter().enumerate() {
            let _ = m.set_dpi_stage_xy(Some(i), s.x, s.y, s.rgb());
        }
        let _ = m.set_dpi_stage_xy(None, self.sniper.x, self.sniper.y, self.sniper.rgb());
        let _ = m.set_angle_snapping(self.angle_snapping);
        let _ = m.set_button_optimization(self.button_optimization);
        let _ = m.select_dpi_stage(Some(stage as usize));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_icue_and_old_files_load() {
        let o = MouseOptions::default();
        assert_eq!(o.stages.map(|s| s.x), [800, 1500, 3000, 6000, 9000]);
        assert_eq!(o.sniper.x, 400);
        // settings.json from 0.3.0 has no "mouse" block; a partial one keeps defaults.
        let partial: MouseOptions = serde_json::from_str(r#"{"polling_ms": 4}"#).unwrap();
        assert_eq!(partial.polling_ms, 4);
        assert_eq!(partial.stages, o.stages);
        assert_eq!(o.dpi_color(2, false), Rgb::new(0, 255, 0));
        assert_eq!(o.dpi_color(2, true), Rgb::new(255, 255, 0));
    }
}
