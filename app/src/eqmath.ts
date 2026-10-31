// Equalizer bands, presets and the response curve, shared by the headset page and Início.

export const FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const Q = 1.41;
export const MAX = 12;

export const PRESETS: { id: string; name: string; gains: number[] }[] = [
  { id: 'flat', name: 'Neutro', gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: 'bass', name: 'Graves+', gains: [6, 5, 3.5, 1.5, 0, 0, 0, 0, 0, 0] },
  { id: 'fps', name: 'FPS (passos)', gains: [-3, -2, -1, 0, 0, 1.5, 3.5, 4.5, 3, 0] },
  { id: 'movie', name: 'Filmes', gains: [4, 3, 1.5, 0, -1, 0, 1.5, 2.5, 2, 1] },
  { id: 'music', name: 'Música (V)', gains: [4.5, 3.5, 1.5, 0, -1.5, -1.5, 0, 1.5, 3, 3.5] },
  { id: 'voice', name: 'Voz', gains: [-4, -3, -1.5, 0, 1.5, 3, 3, 1.5, 0, -1.5] },
];

/** Peaking biquad magnitude in dB at f (RBJ cookbook), fs = 48 kHz. */
function peakDb(f: number, fc: number, gain: number): number {
  if (gain === 0) return 0;
  const fs = 48000, A = Math.pow(10, gain / 40), w0 = (2 * Math.PI * fc) / fs, alpha = Math.sin(w0) / (2 * Q);
  const [b0, b1, b2, a0, a1, a2] = [1 + alpha * A, -2 * Math.cos(w0), 1 - alpha * A, 1 + alpha / A, -2 * Math.cos(w0), 1 - alpha / A];
  const w = (2 * Math.PI * f) / fs;
  const re = (c0: number, c1: number, c2: number) => c0 + c1 * Math.cos(w) + c2 * Math.cos(2 * w);
  const im = (c1: number, c2: number) => -c1 * Math.sin(w) - c2 * Math.sin(2 * w);
  const num = Math.hypot(re(b0, b1, b2), im(b1, b2)), den = Math.hypot(re(a0, a1, a2), im(a1, a2));
  return 20 * Math.log10(num / den);
}

export const W = 640, H = 200;
export const xOf = (f: number) => (Math.log10(f / 20) / Math.log10(20000 / 20)) * W;
export const yOf = (db: number) => H / 2 - (db / (MAX + 3)) * (H / 2);

/** SVG path of the summed response of `gains`, in a W×H box (0 dB in the middle). */
export function eqPath(gains: number[]): string {
  const pts: string[] = [];
  for (let i = 0; i <= 160; i++) {
    const f = 20 * Math.pow(1000, i / 160);
    const db = gains.reduce((s, g, k) => s + peakDb(f, FREQS[k], g), 0);
    pts.push(`${xOf(f).toFixed(1)},${yOf(db).toFixed(1)}`);
  }
  return 'M' + pts.join(' L');
}
