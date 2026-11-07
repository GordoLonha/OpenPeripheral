// On-screen preview of the lighting; mirrors app/src-tauri/src/effects.rs.
import type { Lighting, RGB } from './api';

const TAU = Math.PI * 2;
/** Keyboard width over height (ASPECT in effects.rs). */
const ASPECT = 3;
const RAIN_COLUMNS = 22;

const hue = (h: number): RGB => {
  h = (((h % 360) + 360) % 360) / 60;
  const x = 1 - Math.abs((h % 2) - 1);
  const [r, g, b] = [[1, x, 0], [x, 1, 0], [0, 1, x], [0, x, 1], [x, 0, 1], [1, 0, x]][Math.floor(h) % 6];
  return [r * 255, g * 255, b * 255];
};

const lerp = (a: RGB, b: RGB, t: number): RGB => a.map((v, i) => v + (b[i] - v) * Math.min(1, Math.max(0, t))) as RGB;

const fract = (v: number) => v - Math.floor(v);
const tri = (v: number) => 1 - Math.abs(fract(v) * 2 - 1);

/** Same numbers as `hash` in effects.rs. */
const hash = (n: number): number => {
  let x = Math.imul(n, 0x9e3779b1) >>> 0;
  x = (x ^ (x >>> 15)) >>> 0;
  x = Math.imul(x, 0x85ebca77) >>> 0;
  x = (x ^ (x >>> 13)) >>> 0;
  return (x & 0xffff) / 65535;
};

export function previewColor(l: Lighting, slot: number | null, x: number, y: number, t: number): RGB {
  if (slot != null && l.painted[slot]) return scale(l.painted[slot], l.brightness);
  const s = l.speed;
  let c: RGB;
  switch (l.effect) {
    case 'static': c = l.color; break;
    case 'rainbow': c = hue(t * s * 90 - x * 360); break;
    case 'wave': c = lerp(l.color2, l.color, 0.5 + 0.5 * Math.sin(TAU * (x * 1.5 - t * s * 0.6) + y)); break;
    case 'breathing': c = lerp(l.color2, l.color, 0.5 - 0.5 * Math.cos((TAU * t * s) / 3)); break;
    // Key presses only reach the engine, so the preview shows the resting colour.
    case 'reactive':
    case 'ripple':
      c = l.color2;
      break;
    case 'colorshift': c = lerp(l.color, l.color2, 0.5 - 0.5 * Math.cos((TAU * t * s) / 8)); break;
    case 'colorwave': c = lerp(l.color2, l.color, 1 - Math.abs(fract(x - t * s * 0.35) - 0.5) * 4); break;
    case 'visor': c = lerp(l.color2, l.color, 1 - Math.abs(x - tri(t * s * 0.25)) * 5); break;
    case 'spiral': c = hue((Math.atan2(y - 0.5, (x - 0.5) * ASPECT) / TAU) * 360 + t * s * 90); break;
    case 'rain': {
      const h = hash(Math.floor(Math.min(Math.max(x, 0), 0.999) * RAIN_COLUMNS));
      const d = fract(t * s * (0.3 + 0.4 * h) + h) * 1.4 - 0.2 - y;
      c = lerp(l.color2, l.color, d >= 0 && d < 0.35 ? 1 - d / 0.35 : 0);
      break;
    }
    case 'gradient': c = lerp(l.color, l.color2, x); break;
  }
  return scale(c, l.brightness);
}

const scale = (c: RGB, brightness: number): RGB => c.map((v) => Math.round((v * brightness) / 100)) as RGB;
