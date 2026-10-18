use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Hue in degrees (0..360), full saturation and value.
    pub fn from_hue(h: f32) -> Self {
        let h = h.rem_euclid(360.0) / 60.0;
        let x = 1.0 - (h % 2.0 - 1.0).abs();
        let (r, g, b) = match h as u32 {
            0 => (1.0, x, 0.0),
            1 => (x, 1.0, 0.0),
            2 => (0.0, 1.0, x),
            3 => (0.0, x, 1.0),
            4 => (x, 0.0, 1.0),
            _ => (1.0, 0.0, x),
        };
        Self::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

impl FromStr for Rgb {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 {
            return Err(format!("expected RRGGBB, got {s:?}"));
        }
        let v = u32::from_str_radix(s, 16).map_err(|e| e.to_string())?;
        Ok(Self::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }
}
