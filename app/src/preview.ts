// On-screen preview of the lighting; mirrors app/src-tauri/src/effects.rs.
import type { Lighting, RGB } from './api';

const hue = (h: number): RGB => {
  h = (((h % 360) + 360) % 360) / 60;
  const x = 1 - Math.abs((h % 2) - 1);
  const [r, g, b] = [[1, x, 0], [x, 1, 0], [0, 1, x], [0, x, 1], [x, 0, 1], [1, 0, x]][Math.floor(h) % 6];
  return [r * 255, g * 255, b * 255];
};

const lerp = (a: RGB, b: RGB, t: number): RGB => a.map((v, i) => v + (b[i] - v) * Math.min(1, Math.max(0, t))) as RGB;

export function previewColor(l: Lighting, slot: number | null, x: number, y: number, t: number): RGB {
  if (slot != null && l.painted[slot]) return scale(l.painted[slot], l.brightness);
  let c: RGB;
  switch (l.effect) {
    case 'static': c = l.color; break;
    case 'rainbow': c = hue(t * l.speed * 90 - x * 360); break;
    case 'wave': c = lerp(l.color2, l.color, 0.5 + 0.5 * Math.sin(Math.PI * 2 * (x * 1.5 - t * l.speed * 0.6) + y)); break;
    case 'breathing': c = lerp(l.color2, l.color, 0.5 - 0.5 * Math.cos((Math.PI * 2 * t * l.speed) / 3)); break;
    case 'reactive': c = l.color2; break;
  }
  return scale(c, l.brightness);
}

const scale = (c: RGB, brightness: number): RGB => c.map((v) => Math.round((v * brightness) / 100)) as RGB;
