//! Lighting effects: pure functions from (settings, time, key state) to a frame.
//! `app/src/preview.ts` mirrors these for the on-screen preview; keep the two in step.

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
    /// Every key fades together from the main colour to the second one and back.
    ColorShift,
    /// A band of the main colour sweeps left to right over the second colour.
    ColorWave,
    /// A band of the main colour bounces from side to side over the second colour.
    Visor,
    /// Rainbow turning around the centre of the keyboard.
    Spiral,
    /// Drops of the main colour fall down the columns over the second colour.
    Rain,
    /// Still blend from the main colour (left) to the second one (right).
    Gradient,
    /// Each key press sends out a ring of the main colour over the second colour.
    Ripple,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Width of the keyboard over its height, so round shapes (rings, the spiral)
/// stay round on the normalised 0..1 x 0..1 key positions.
const ASPECT: f32 = 3.0;
/// Seconds a Ripple ring lives.
const RIPPLE_LIFE: f32 = 1.6;
/// Most rings alive at once; the oldest goes first.
const RIPPLE_MAX: usize = 24;
/// Rain columns across the keyboard.
const RAIN_COLUMNS: u32 = 22;

/// Key state for the reactive effects: per-key "heat" for Reactive (1.0 when
/// pressed, decays to 0) and the rings started by key presses for Ripple.
pub struct Heat {
    keys: [f32; LED_COUNT],
    /// (LED slot where the key was pressed, seconds since).
    ripples: Vec<(u16, f32)>,
}

impl Heat {
    pub fn new() -> Self {
        Self { keys: [0.0; LED_COUNT], ripples: Vec::new() }
    }

    /// The key is held (called on every key report while it is down).
    pub fn press(&mut self, slot: usize) {
        if let Some(h) = self.keys.get_mut(slot) {
            *h = 1.0;
        }
    }

    /// The key went down just now: start a ring there.
    pub fn ripple(&mut self, slot: usize) {
        if slot >= LED_COUNT {
            return;
        }
        if self.ripples.len() >= RIPPLE_MAX {
            self.ripples.remove(0);
        }
        self.ripples.push((slot as u16, 0.0));
    }

    pub fn decay(&mut self, dt: f32, speed: f32) {
        for h in &mut self.keys {
            *h = (*h - dt * speed * 1.5).max(0.0);
        }
        for r in &mut self.ripples {
            r.1 += dt;
        }
        self.ripples.retain(|r| r.1 < RIPPLE_LIFE);
    }

    pub fn active(&self) -> bool {
        self.keys.iter().any(|&h| h > 0.0) || !self.ripples.is_empty()
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

/// Triangle wave: 0 -> 1 -> 0 over each unit of `v`.
fn tri(v: f32) -> f32 {
    1.0 - (v.rem_euclid(1.0) * 2.0 - 1.0).abs()
}

/// Repeatable pseudo-random 0..1 from an integer (same as `hash` in preview.ts).
fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B1);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EB_CA77);
    x ^= x >> 13;
    (x & 0xFFFF) as f32 / 65535.0
}

/// A Ripple ring at a known place: (x, y) of the pressed key, seconds since.
type Ring = (f32, f32, f32);

/// How strongly the rings light (x, y): 1 on a fresh ring's edge, 0 away from all rings.
fn ring_strength(rings: &[Ring], x: f32, y: f32, speed: f32) -> f32 {
    rings.iter().fold(0.0, |best: f32, &(ox, oy, age)| {
        let d = (((x - ox) * ASPECT).powi(2) + (y - oy).powi(2)).sqrt();
        let radius = age * speed * 1.5;
        let edge = (1.0 - (d - radius).abs() * 4.0).max(0.0);
        best.max(edge * (1.0 - age / RIPPLE_LIFE).max(0.0))
    })
}

/// Colour of the effect at (x, y) at time t; `heat` only matters for Reactive
/// and `rings` only for Ripple.
fn pixel(l: &Lighting, x: f32, y: f32, t: f32, heat: f32, rings: &[Ring]) -> Rgb {
    let (c1, c2) = (rgb(l.color), rgb(l.color2));
    let tau = std::f32::consts::TAU;
    let s = l.speed;
    match l.effect {
        Effect::Static => c1,
        Effect::Rainbow => Rgb::from_hue(t * s * 90.0 - x * 360.0),
        Effect::Wave => {
            let k = 0.5 + 0.5 * (tau * (x * 1.5 - t * s * 0.6) + y).sin();
            lerp(c2, c1, k)
        }
        Effect::Breathing => {
            let k = 0.5 - 0.5 * (tau * t * s / 3.0).cos();
            lerp(c2, c1, k)
        }
        Effect::Reactive => lerp(c2, c1, heat),
        Effect::ColorShift => {
            let k = 0.5 - 0.5 * (tau * t * s / 8.0).cos();
            lerp(c1, c2, k)
        }
        Effect::ColorWave => {
            let d = (x - t * s * 0.35).rem_euclid(1.0);
            lerp(c2, c1, 1.0 - (d - 0.5).abs() * 4.0)
        }
        Effect::Visor => {
            let p = tri(t * s * 0.25);
            lerp(c2, c1, 1.0 - (x - p).abs() * 5.0)
        }
        Effect::Spiral => {
            let turn = (y - 0.5).atan2((x - 0.5) * ASPECT) / tau;
            Rgb::from_hue(turn * 360.0 + t * s * 90.0)
        }
        Effect::Rain => {
            let col = (x.clamp(0.0, 0.999) * RAIN_COLUMNS as f32) as u32;
            let h = hash(col);
            let drop = (t * s * (0.3 + 0.4 * h) + h).rem_euclid(1.0) * 1.4 - 0.2;
            let d = drop - y;
            let k = if (0.0..0.35).contains(&d) { 1.0 - d / 0.35 } else { 0.0 };
            lerp(c2, c1, k)
        }
        Effect::Gradient => lerp(c1, c2, x),
        Effect::Ripple => lerp(c2, c1, ring_strength(rings, x, y, s)),
    }
}

fn dim(l: &Lighting, c: Rgb) -> Rgb {
    let k = l.brightness.min(100) as f32 / 100.0;
    if k < 1.0 { scale(c, k) } else { c }
}

fn place(geo: &Geometry, slot: u16) -> (f32, f32) {
    geo.get(&slot).copied().unwrap_or((slot as f32 / LED_COUNT as f32, 0.5))
}

pub fn render(l: &Lighting, geo: &Geometry, t: f32, heat: &Heat) -> [Rgb; LED_COUNT] {
    let rings: Vec<Ring> = if l.effect == Effect::Ripple {
        heat.ripples
            .iter()
            .map(|&(slot, age)| {
                let (x, y) = place(geo, slot);
                (x, y, age)
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut frame = [Rgb::default(); LED_COUNT];
    for (slot, px) in frame.iter_mut().enumerate() {
        let (x, y) = place(geo, slot as u16);
        *px = pixel(l, x, y, t, heat.keys[slot], &rings);
        if let Some(&p) = l.painted.get(&(slot as u16)) {
            *px = rgb(p);
        }
        *px = dim(l, *px);
    }
    frame
}

/// Headset logo colour: the keyboard effect sampled at the keyboard's centre,
/// so it moves in step with it (Reactive and Ripple show their resting colour).
pub fn headset(l: &Lighting, t: f32) -> Rgb {
    dim(l, pixel(l, 0.5, 0.5, t, 0.0, &[]))
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
        Effect::Static | Effect::Gradient => false,
        Effect::Reactive | Effect::Ripple => heat.active(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lighting(effect: Effect) -> Lighting {
        Lighting { effect, color: [255, 255, 255], color2: [0, 0, 0], ..Default::default() }
    }

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
        let l = lighting(Effect::Reactive);
        let mut h = Heat::new();
        h.press(5);
        assert_eq!(render(&l, &Geometry::new(), 0.0, &h)[5], Rgb::new(255, 255, 255));
        h.decay(1.0, 1.0);
        assert!(!h.active());
    }

    #[test]
    fn effects_save_with_lowercase_names() {
        assert_eq!(serde_json::to_string(&Effect::ColorShift).unwrap(), "\"colorshift\"");
        assert_eq!(serde_json::from_str::<Effect>("\"ripple\"").unwrap(), Effect::Ripple);
    }

    #[test]
    fn color_shift_is_uniform_and_cycles() {
        let l = lighting(Effect::ColorShift);
        assert_eq!(pixel(&l, 0.1, 0.2, 0.0, 0.0, &[]), Rgb::new(255, 255, 255));
        // Half way through the 8 s cycle every key shows the second colour.
        assert_eq!(pixel(&l, 0.1, 0.2, 4.0, 0.0, &[]), Rgb::new(0, 0, 0));
        assert_eq!(pixel(&l, 0.9, 0.8, 4.0, 0.0, &[]), Rgb::new(0, 0, 0));
    }

    #[test]
    fn gradient_runs_left_to_right_and_is_still() {
        let l = lighting(Effect::Gradient);
        assert_eq!(pixel(&l, 0.0, 0.5, 3.0, 0.0, &[]), Rgb::new(255, 255, 255));
        assert_eq!(pixel(&l, 1.0, 0.5, 3.0, 0.0, &[]), Rgb::new(0, 0, 0));
        assert!(!animated(&l, &Heat::new()));
    }

    #[test]
    fn visor_band_moves() {
        let l = lighting(Effect::Visor);
        // At t = 0 the band is at the left edge.
        assert_eq!(pixel(&l, 0.0, 0.5, 0.0, 0.0, &[]), Rgb::new(255, 255, 255));
        assert_eq!(pixel(&l, 0.9, 0.5, 0.0, 0.0, &[]), Rgb::new(0, 0, 0));
        // One second later (speed 1) it is in the middle, after two at the right edge.
        assert_eq!(pixel(&l, 0.5, 0.5, 1.0, 0.0, &[]), Rgb::new(255, 255, 255));
        assert_eq!(pixel(&l, 1.0, 0.5, 2.0, 0.0, &[]), Rgb::new(255, 255, 255));
    }

    #[test]
    fn rain_stays_in_range() {
        let l = lighting(Effect::Rain);
        let (mut lit, mut total) = (0, 0);
        for step in 0..40 {
            for i in 0..100 {
                let c = pixel(&l, i as f32 / 100.0, 0.5, step as f32 * 0.37, 0.0, &[]);
                assert!(c.r == c.g && c.g == c.b);
                lit += u32::from(c.r > 0);
                total += 1;
            }
        }
        assert!(lit > 0 && lit < total, "some keys lit, not all ({lit} of {total})");
        assert_eq!(hash(7), hash(7));
    }

    #[test]
    fn ripple_rings_spread_and_fade() {
        let l = lighting(Effect::Ripple);
        let mut geo = Geometry::new();
        geo.insert(10, (0.5, 0.5));
        let mut h = Heat::new();
        assert!(!animated(&l, &h));
        h.ripple(10);
        assert!(animated(&l, &h));
        // Fresh ring: lit where the key is.
        assert_eq!(render(&l, &geo, 0.0, &h)[10], Rgb::new(255, 255, 255));
        // 0.4 s later the ring has moved out: dark at the key, lit 0.6 high above it.
        h.decay(0.4, 1.0);
        let rings = [(0.5, 0.5, 0.4)];
        assert!(ring_strength(&rings, 0.5, 0.5, 1.0) == 0.0);
        assert!(ring_strength(&rings, 0.5, 1.1, 1.0) > 0.5);
        // Gone after its life.
        h.decay(RIPPLE_LIFE, 1.0);
        assert!(!h.active());
    }

    #[test]
    fn ripple_keeps_newest() {
        let mut h = Heat::new();
        for i in 0..RIPPLE_MAX + 5 {
            h.ripple(i);
        }
        assert_eq!(h.ripples.len(), RIPPLE_MAX);
        assert_eq!(h.ripples.last().map(|r| r.0), Some((RIPPLE_MAX + 4) as u16));
        h.ripple(LED_COUNT + 1); // not a slot: ignored
        assert_eq!(h.ripples.len(), RIPPLE_MAX);
    }

    #[test]
    fn spiral_turns() {
        let l = lighting(Effect::Spiral);
        assert_ne!(pixel(&l, 0.9, 0.5, 0.0, 0.0, &[]), pixel(&l, 0.1, 0.5, 0.0, 0.0, &[]));
        assert_ne!(pixel(&l, 0.9, 0.5, 0.0, 0.0, &[]), pixel(&l, 0.9, 0.5, 1.0, 0.0, &[]));
    }
}
