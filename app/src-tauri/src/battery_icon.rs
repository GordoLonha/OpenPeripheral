//! Tray battery meter, drawn at runtime: a battery outline filled to the
//! current level, coloured by state (green / yellow / red, blue + bolt while
//! charging, grey when the headset is off).

pub const SIZE: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Off,
    Discharging(f32),
    Charging(f32),
}

pub fn color(state: State) -> [u8; 3] {
    match state {
        State::Off => [120, 128, 140],
        State::Charging(_) => [57, 208, 240],
        State::Discharging(p) if p < 15.0 => [255, 77, 90],
        State::Discharging(p) if p < 40.0 => [255, 196, 64],
        State::Discharging(_) => [61, 220, 132],
    }
}

/// Signed distance to a rounded rectangle (negative inside).
fn rrect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hw, hh) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let dx = (x - cx).abs() - (hw - r);
    let dy = (y - cy).abs() - (hh - r);
    (dx.max(0.0).hypot(dy.max(0.0))) + dx.max(dy).min(0.0) - r
}

fn cover(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

/// Lightning bolt polygon (point-in-polygon, for the charging glyph).
fn in_bolt(x: f32, y: f32) -> bool {
    const P: [(f32, f32); 6] = [(17.5, 9.0), (10.5, 17.0), (15.0, 17.0), (13.5, 24.0), (21.0, 15.0), (16.5, 15.0)];
    let mut inside = false;
    let mut j = P.len() - 1;
    for i in 0..P.len() {
        let (xi, yi) = P[i];
        let (xj, yj) = P[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// RGBA pixels, SIZE x SIZE.
pub fn render(state: State) -> Vec<u8> {
    let [r, g, b] = color(state);
    let level = match state {
        State::Off => 0.0,
        State::Discharging(p) | State::Charging(p) => (p / 100.0).clamp(0.0, 1.0),
    };
    // Horizontal battery: body 2..27 x 8..24, terminal 27..30 x 13..19.
    let (bx0, by0, bx1, by1) = (2.0, 8.0, 27.0, 24.0);
    let fill_x1 = 5.0 + (bx1 - 3.0 - 5.0) * level;
    let mut px = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            // 2x2 supersampling
            let mut acc = [0f32; 4];
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let (fx, fy) = (x as f32 + sx, y as f32 + sy);
                let outer = rrect(fx, fy, bx0, by0, bx1, by1, 3.5);
                let inner = rrect(fx, fy, bx0 + 2.2, by0 + 2.2, bx1 - 2.2, by1 - 2.2, 1.8);
                let shell = cover(outer) * (1.0 - cover(inner));
                let tip = cover(rrect(fx, fy, bx1 - 0.5, 13.0, 30.0, 19.0, 1.5));
                let fill = if level > 0.0 { cover(rrect(fx, fy, 5.0, 11.0, fill_x1.max(9.0), 21.0, 1.2)) } else { 0.0 };
                // White outline (readable on dark and light taskbars thanks to the dark halo below).
                let halo = cover(outer - 1.3) * (1.0 - cover(outer));
                let mut c = [0.0f32; 4];
                let mut put = |col: [f32; 3], a: f32| {
                    for k in 0..3 {
                        c[k] = c[k] * (1.0 - a) + col[k] * a;
                    }
                    c[3] = c[3] + a * (1.0 - c[3]);
                };
                put([0.0, 0.0, 0.0], halo * 0.6);
                put([235.0, 238.0, 245.0], shell.max(tip));
                put([r as f32, g as f32, b as f32], fill);
                if matches!(state, State::Charging(_)) && in_bolt(fx, fy) {
                    put([255.0, 255.0, 255.0], 1.0);
                }
                for k in 0..4 {
                    acc[k] += c[k] / 4.0;
                }
            }
            // Colours were composited premultiplied; store straight alpha.
            let i = ((y * SIZE + x) * 4) as usize;
            let a = acc[3];
            for k in 0..3 {
                px[i + k] = if a > 0.0 { (acc[k] / a).round().clamp(0.0, 255.0) as u8 } else { 0 };
            }
            px[i + 3] = (a * 255.0).round() as u8;
        }
    }
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_by_level() {
        assert_eq!(color(State::Discharging(80.0)), [61, 220, 132]);
        assert_eq!(color(State::Discharging(25.0)), [255, 196, 64]);
        assert_eq!(color(State::Discharging(5.0)), [255, 77, 90]);
        assert_eq!(color(State::Charging(5.0)), [57, 208, 240]);
    }

    /// `cargo test -p openperipheral-app dump_icons -- --ignored` writes raw RGBA
    /// previews to the temp dir (convert with tools/rgba_to_png.py).
    #[test]
    #[ignore]
    fn dump_icons() {
        let dir = std::env::temp_dir();
        for (name, s) in [
            ("full", State::Discharging(90.0)),
            ("mid", State::Discharging(30.0)),
            ("low", State::Discharging(8.0)),
            ("charging", State::Charging(55.0)),
            ("off", State::Off),
        ] {
            std::fs::write(dir.join(format!("op_battery_{name}.rgba")), render(s)).unwrap();
        }
    }

    #[test]
    fn renders_rgba() {
        let px = render(State::Discharging(50.0));
        assert_eq!(px.len(), (SIZE * SIZE * 4) as usize);
        assert!(px.chunks(4).any(|p| p[3] > 200), "has opaque pixels");
    }
}
