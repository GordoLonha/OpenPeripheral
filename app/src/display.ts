// What the UI shows about lighting and batteries, shared by the pages.
import type { Effect, Lighting } from './api';

export const EFFECTS: { id: Effect; name: string; hint: string; two?: boolean }[] = [
  { id: 'static', name: 'Cor fixa', hint: 'Uma cor em todo o teclado' },
  { id: 'rainbow', name: 'Arco-íris', hint: 'Espectro correndo da esquerda para a direita' },
  { id: 'wave', name: 'Onda', hint: 'Onda entre duas cores', two: true },
  { id: 'breathing', name: 'Respiração', hint: 'Pulsa suavemente entre duas cores', two: true },
  { id: 'reactive', name: 'Reativo', hint: 'A tecla acende quando você aperta', two: true },
];

/** One-click lighting scenes (applied to the active profile). */
export const SCENES: { id: string; name: string; css: string; patch: Partial<Lighting> }[] = [
  { id: 'rainbow', name: 'Arco-íris', css: 'linear-gradient(90deg,#ff3b3b,#ffb02e,#f6ff3d,#3dff7a,#3dc8ff,#7a5cff,#ff3df2)', patch: { effect: 'rainbow', speed: 1 } },
  { id: 'ocean', name: 'Oceano', css: 'linear-gradient(120deg,#003a8c,#00c2ff,#003a8c)', patch: { effect: 'wave', color: [0, 190, 255], color2: [0, 20, 90], speed: 0.8 } },
  { id: 'sunset', name: 'Pôr do sol', css: 'linear-gradient(120deg,#5a005e,#ff4f6d,#ffb347)', patch: { effect: 'wave', color: [255, 120, 40], color2: [120, 0, 90], speed: 0.6 } },
  { id: 'aurora', name: 'Aurora', css: 'linear-gradient(120deg,#002b1f,#3dff9a,#7a5cff)', patch: { effect: 'wave', color: [60, 255, 150], color2: [80, 40, 200], speed: 0.5 } },
  { id: 'calm', name: 'Respirar', css: 'radial-gradient(circle,#b37bff,#2a0b4a)', patch: { effect: 'breathing', color: [170, 110, 255], color2: [10, 0, 30], speed: 0.8 } },
  { id: 'reactive', name: 'Reativo', css: 'radial-gradient(circle at 35% 50%,#ffffff 0 12%,#00d0ff 13% 30%,#06121c 31%)', patch: { effect: 'reactive', color: [255, 255, 255], color2: [0, 40, 70], speed: 1.2 } },
  { id: 'white', name: 'Branco', css: '#f2f4f8', patch: { effect: 'static', color: [255, 255, 255] } },
  { id: 'red', name: 'Vermelho', css: '#ff2a3d', patch: { effect: 'static', color: [255, 20, 30] } },
];

/** The scene the lighting is set to, if it is exactly one of them (and nothing painted). */
export function sceneOf(l: Lighting): (typeof SCENES)[number] | undefined {
  if (Object.keys(l.painted).length > 0) return undefined;
  return SCENES.find((s) =>
    Object.entries(s.patch).every(([k, v]) => JSON.stringify(l[k as keyof Lighting]) === JSON.stringify(v)),
  );
}

export function batteryColor(pct: number, charging: boolean): string {
  if (charging) return 'var(--accent)';
  if (pct < 15) return 'var(--danger)';
  if (pct < 40) return 'var(--warn)';
  return 'var(--ok)';
}
