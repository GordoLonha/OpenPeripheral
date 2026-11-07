import type { Direction, Effect, Lighting, Status } from './api';
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

const FOUR: [Direction, string, string][] = [
  ['right', '→', 'Para a direita'],
  ['left', '←', 'Para a esquerda'],
  ['down', '↓', 'Para baixo'],
  ['up', '↑', 'Para cima'],
];

/** `colors`: names of the colours the effect uses (none for the rainbow ones). `still`: no speed.
 *  `dirs`: directions it can take (value, label, accessible name); `double`: can mirror from the
 *  middle; `random`: can take random colours. */
export const EFFECTS: {
  id: Effect;
  name: string;
  hint: string;
  colors: string[];
  still?: boolean;
  dirs?: [Direction, string, string][];
  double?: boolean;
  random?: boolean;
}[] = [
  { id: 'static', name: 'Cor fixa', hint: 'Uma cor em todo o teclado', colors: ['Cor'], still: true },
  { id: 'rainbow', name: 'Arco-íris', hint: 'Espectro correndo pelo teclado', colors: [], dirs: FOUR, double: true },
  { id: 'wave', name: 'Onda', hint: 'Onda entre duas cores', colors: ['Cor 1', 'Cor 2'], dirs: FOUR, double: true },
  { id: 'breathing', name: 'Respiração', hint: 'Pulsa suavemente entre duas cores', colors: ['Cor 1', 'Cor 2'] },
  { id: 'reactive', name: 'Reativo', hint: 'A tecla acende quando você aperta', colors: ['Ao apertar', 'Fundo'] },
  { id: 'ripple', name: 'Ondulação', hint: 'Cada tecla apertada solta um anel de luz', colors: ['Anel', 'Fundo'] },
  { id: 'colorshift', name: 'Troca de cor', hint: 'O teclado inteiro vai de uma cor à outra e volta', colors: ['Cor 1', 'Cor 2'] },
  { id: 'colorwave', name: 'Faixa de cor', hint: 'Uma faixa de cor atravessa o teclado', colors: ['Faixa', 'Fundo'], dirs: FOUR, double: true },
  { id: 'visor', name: 'Visor', hint: 'Uma faixa vai e volta de um lado ao outro', colors: ['Faixa', 'Fundo'], dirs: FOUR },
  {
    id: 'rain',
    name: 'Chuva',
    hint: 'Gotas de cor caem pelas colunas',
    colors: ['Gotas', 'Fundo'],
    dirs: [['down', 'Caindo', 'Gotas caindo'], ['up', 'Subindo', 'Gotas subindo']],
    random: true,
  },
  {
    id: 'spiral',
    name: 'Espiral',
    hint: 'Arco-íris girando em volta do centro',
    colors: [],
    dirs: [['right', 'Anti-horário', 'Girando no sentido anti-horário'], ['left', 'Horário', 'Girando no sentido horário']],
  },
  { id: 'gradient', name: 'Degradê', hint: 'Uma cor passando para outra', colors: ['Início', 'Fim'], still: true, dirs: FOUR, double: true },
  { id: 'type', name: 'Digitação', hint: 'Cada tecla apertada acende, alternando as cores, e apaga aos poucos', colors: ['Cor 1', 'Cor 2'], random: true },
  { id: 'temperature', name: 'Temperatura', hint: 'A cor acompanha a temperatura do processador', colors: ['Frio', 'Quente'], still: true },
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
