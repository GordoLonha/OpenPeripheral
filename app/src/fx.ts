import type { Effect, Lighting, Status } from './api';
import type { DeviceId } from './DeviceArt';

/** The connected devices, in the order the app shows them. */
export function connected(status: Status | null): DeviceId[] {
  const out: DeviceId[] = [];
  if (status?.keyboard) out.push('keyboard');
  if (status?.headset) out.push('headset');
  if (status?.mouse) out.push('mouse');
  return out;
}

export function deviceName(id: DeviceId, status: Status | null): string {
  if (id === 'keyboard') return status?.keyboard?.name ?? 'K70 RGB MK.2 Low Profile';
  if (id === 'headset') return status?.headset?.name ?? 'HS80 RGB Wireless';
  return status?.mouse?.name ?? 'M65 RGB Elite';
}

/** `colors`: names of the colours the effect uses (none for the rainbow ones). `still`: no speed. */
export const EFFECTS: { id: Effect; name: string; hint: string; colors: string[]; still?: boolean }[] = [
  { id: 'static', name: 'Cor fixa', hint: 'Uma cor em todo o teclado', colors: ['Cor'], still: true },
  { id: 'rainbow', name: 'Arco-íris', hint: 'Espectro correndo da esquerda para a direita', colors: [] },
  { id: 'wave', name: 'Onda', hint: 'Onda entre duas cores', colors: ['Cor 1', 'Cor 2'] },
  { id: 'breathing', name: 'Respiração', hint: 'Pulsa suavemente entre duas cores', colors: ['Cor 1', 'Cor 2'] },
  { id: 'reactive', name: 'Reativo', hint: 'A tecla acende quando você aperta', colors: ['Ao apertar', 'Fundo'] },
  { id: 'ripple', name: 'Ondulação', hint: 'Cada tecla apertada solta um anel de luz', colors: ['Anel', 'Fundo'] },
  { id: 'colorshift', name: 'Troca de cor', hint: 'O teclado inteiro vai de uma cor à outra e volta', colors: ['Cor 1', 'Cor 2'] },
  { id: 'colorwave', name: 'Faixa de cor', hint: 'Uma faixa de cor atravessa o teclado', colors: ['Faixa', 'Fundo'] },
  { id: 'visor', name: 'Visor', hint: 'Uma faixa vai e volta de um lado ao outro', colors: ['Faixa', 'Fundo'] },
  { id: 'rain', name: 'Chuva', hint: 'Gotas de cor caem pelas colunas', colors: ['Gotas', 'Fundo'] },
  { id: 'spiral', name: 'Espiral', hint: 'Arco-íris girando em volta do centro', colors: [] },
  { id: 'gradient', name: 'Degradê', hint: 'Uma cor à esquerda passando para outra à direita', colors: ['Esquerda', 'Direita'], still: true },
];

export const effectName = (id: Effect) => EFFECTS.find((e) => e.id === id)?.name ?? id;

/** One-click lighting scenes (applied to the active profile). */
export const SCENES: { id: string; name: string; patch: Partial<Lighting> }[] = [
  { id: 'rainbow', name: 'Arco-íris', patch: { effect: 'rainbow', speed: 1 } },
  { id: 'ocean', name: 'Oceano', patch: { effect: 'wave', color: [0, 190, 255], color2: [0, 20, 90], speed: 0.8 } },
  { id: 'sunset', name: 'Pôr do sol', patch: { effect: 'wave', color: [255, 120, 40], color2: [120, 0, 90], speed: 0.6 } },
  { id: 'aurora', name: 'Aurora', patch: { effect: 'wave', color: [60, 255, 150], color2: [80, 40, 200], speed: 0.5 } },
  { id: 'rain', name: 'Chuva', patch: { effect: 'rain', color: [90, 180, 255], color2: [0, 6, 24], speed: 1 } },
  { id: 'spiral', name: 'Espiral', patch: { effect: 'spiral', speed: 0.8 } },
  { id: 'visor', name: 'Visor', patch: { effect: 'visor', color: [255, 30, 30], color2: [24, 0, 0], speed: 1 } },
  { id: 'candy', name: 'Algodão-doce', patch: { effect: 'gradient', color: [255, 60, 170], color2: [60, 140, 255] } },
  { id: 'calm', name: 'Respirar', patch: { effect: 'breathing', color: [170, 110, 255], color2: [10, 0, 30], speed: 0.8 } },
  { id: 'shift', name: 'Troca de cor', patch: { effect: 'colorshift', color: [255, 0, 90], color2: [0, 200, 255], speed: 1 } },
  { id: 'white', name: 'Branco', patch: { effect: 'static', color: [255, 255, 255] } },
  { id: 'yellow', name: 'Amarelo', patch: { effect: 'static', color: [236, 232, 26] } },
];

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Whether the lighting is exactly this scene (and nothing is painted over it). */
export function sceneOn(l: Lighting, patch: Partial<Lighting>): boolean {
  if (Object.keys(l.painted).length) return false;
  return Object.entries(patch).every(([k, v]) => same(l[k as keyof Lighting], v));
}

/** The lighting a scene's swatch shows: the scene over a neutral base. */
export function sceneLighting(patch: Partial<Lighting>): Lighting {
  return { effect: 'static', color: [255, 255, 255], color2: [0, 0, 0], speed: 1, brightness: 100, painted: {}, ...patch };
}
