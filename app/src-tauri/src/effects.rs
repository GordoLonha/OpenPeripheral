//! Lighting effects: pure functions from (settings, time, key state) to a frame.

use std::collections::HashMap;

use op_core::{k70::LED_COUNT, Rgb};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Static,
    Rainbow,
    Wave,
    Breathing,
    Reactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lighting {
    pub effect: Effect,
    /// Main colour (effect colour / pressed colour for Reactive).
    pub color: [u8; 3],
    /// Secondary colour (wave trough / Reactive background).
    pub color2: [u8; 3],
    /// 0.1 (slow) .. 3.0 (fast).
    pub speed: f32,
    /// 0..=100.
    pub brightness: u8,
    /// Per-key colours painted by the user, drawn over the effect. Key = LED slot.
    #[serde(default)]
    pub painted: HashMap<u16, [u8; 3]>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            effect: Effect::Rainbow,
            color: [0, 170, 255],
            color2: [10, 0, 40],
            speed: 1.0,
            brightness: 100,
            painted: HashMap::new(),
        }
    }
}

/// Normalised key centre (0..1, 0..1) per LED slot; slots without a key get a
/// position along the x axis so effects still cover them.
pub type Geometry = HashMap<u16, (f32, f32)>;

/// Per-key "heat" for the Reactive effect: 1.0 when pressed, decays to 0.
pub struct Heat(pub [f32; LED_COUNT]);

impl Heat {
    pub fn new() -> Self {
        Self([0.0; LED_COUNT])
    }

    pub fn press(&mut self, slot: usize) {
        if let Some(h) = self.0.get_mut(slot) {
            *h = 1.0;
        }
    }

    pub fn decay(&mut self, dt: f32, speed: f32) {
        for h in &mut self.0 {
            *h = (*h - dt * speed * 1.5).max(0.0);
        }
    }

    pub fn active(&self) -> bool {
        self.0.iter().any(|&h| h > 0.0)
    }
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb::new(c[0], c[1], c[2])
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgb::new(m(a.r, b.r), m(a.g, b.g), m(a.b, b.b))
}

fn scale(c: Rgb, k: f32) -> Rgb {
    let m = |x: u8| (x as f32 * k).round().clamp(0.0, 255.0) as u8;
    Rgb::new(m(c.r), m(c.g), m(c.b))
}

/// Colour of the effect at (x, y) at time t; `heat` only matters for Reactive.
fn pixel(l: &Lighting, x: f32, y: f32, t: f32, heat: f32) -> Rgb {
    let (c1, c2) = (rgb(l.color), rgb(l.color2));
    let tau = std::f32::consts::TAU;
    match l.effect {
        Effect::Static => c1,
        Effect::Rainbow => Rgb::from_hue(t * l.speed * 90.0 - x * 360.0),
        Effect::Wave => {
            let k = 0.5 + 0.5 * (tau * (x * 1.5 - t * l.speed * 0.6) + y).sin();
            lerp(c2, c1, k)
        }
        Effect::Breathing => {
            let k = 0.5 - 0.5 * (tau * t * l.speed / 3.0).cos();
            lerp(c2, c1, k)
        }
        Effect::Reactive => lerp(c2, c1, heat),
    }
}

fn dim(l: &Lighting, c: Rgb) -> Rgb {
    let k = l.brightness.min(100) as f32 / 100.0;
    if k < 1.0 { scale(c, k) } else { c }
}

pub fn render(l: &Lighting, geo: &Geometry, t: f32, heat: &Heat) -> [Rgb; LED_COUNT] {
    let mut frame = [Rgb::default(); LED_COUNT];
    for (slot, px) in frame.iter_mut().enumerate() {
        let (x, y) = geo.get(&(slot as u16)).copied().unwrap_or((slot as f32 / LED_COUNT as f32, 0.5));
        *px = pixel(l, x, y, t, heat.0[slot]);
        if let Some(&p) = l.painted.get(&(slot as u16)) {
            *px = rgb(p);
        }
        *px = dim(l, *px);
    }
    frame
}

/// Headset logo colour: the keyboard effect sampled at the keyboard's centre,
/// so it moves in step with it (Reactive shows its resting colour).
pub fn headset(l: &Lighting, t: f32) -> Rgb {
    dim(l, pixel(l, 0.5, 0.5, t, 0.0))
}

/// Brightness key: 100 -> 66 -> 33 -> off -> 100 (like the keyboard's own levels).
pub fn next_brightness(b: u8) -> u8 {
    match b {
        67.. => 66,
        34..=66 => 33,
        1..=33 => 0,
        0 => 100,
    }
}

/// Whether the effect changes over time (so the engine must keep sending frames).
pub fn animated(l: &Lighting, heat: &Heat) -> bool {
    match l.effect {
        Effect::Static => false,
        Effect::Reactive => heat.active(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_key_cycles() {
        let mut b = 100;
        let mut seen = vec![];
        for _ in 0..4 {
            b = next_brightness(b);
            seen.push(b);
        }
        assert_eq!(seen, [66, 33, 0, 100]);
        assert_eq!(next_brightness(80), 66);
    }

    #[test]
    fn static_with_paint_and_brightness() {
        let mut l = Lighting { effect: Effect::Static, color: [200, 100, 0], brightness: 50, ..Default::default() };
        l.painted.insert(3, [0, 0, 255]);
        let f = render(&l, &Geometry::new(), 0.0, &Heat::new());
        assert_eq!(f[0], Rgb::new(100, 50, 0));
        assert_eq!(f[3], Rgb::new(0, 0, 128));
    }

    #[test]
    fn reactive_heat_decays() {
        let l = Lighting { effect: Effect::Reactive, color: [255, 255, 255], color2: [0, 0, 0], ..Default::default() };
        let mut h = Heat::new();
        h.press(5);
        assert_eq!(render(&l, &Geometry::new(), 0.0, &h)[5], Rgb::new(255, 255, 255));
        h.decay(1.0, 1.0);
        assert!(!h.active());
    }
}
