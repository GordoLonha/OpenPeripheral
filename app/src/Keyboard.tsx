import { useEffect, useRef } from 'react';
import type { Lighting } from './api';
import { BOARD_H, BOARD_W, KEYS } from './layout';
import { previewColor } from './preview';

const U = 52; // px per key unit

// Symbol legends drawn as vectors (24x24 box): symbol fonts are missing on many
// Linux installs and render as empty boxes.
const GLYPHS: Record<string, string> = {
  '☀': 'M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8zM12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2',
  '🔒': 'M7 11h10v9H7zM9 11V8a3 3 0 0 1 6 0v3',
  '🔇': 'M4 9h4l5-4v14l-5-4H4zM16 9l5 6M21 9l-5 6',
  '■': 'M7 7h10v10H7z',
  '⏮': 'M6 6v12M18 6l-9 6 9 6z',
  '⏯': 'M5 6l8 6-8 6zM16 6v12M20 6v12',
  '⏭': 'M18 6v12M6 6l9 6-9 6z',
  '⌫': 'M9 6h11v12H9l-6-6zM12 9l5 6M17 9l-5 6',
  '⏎': 'M19 6v7H6M10 9l-4 4 4 4',
  '☰': 'M5 7h14M5 12h14M5 17h14',
};
const GAP = 4;

type Props = {
  lighting: Lighting;
  /** Key name -> LED slot. */
  ledmap: Record<string, number | null>;
  /** Called with the LED slot under the pointer while painting (button held). */
  onPaint?: (slot: number) => void;
  painting: boolean;
  /** Key-selection mode (remapping): click selects a key by id. */
  onSelectKey?: (id: string) => void;
  selectedKey?: string | null;
  /** Keys to badge (e.g. remapped). */
  marked?: Set<string>;
};

export function Keyboard({ lighting, ledmap, onPaint, painting, onSelectKey, selectedKey, marked }: Props) {
  const svg = useRef<SVGSVGElement>(null);
  const live = useRef(lighting);
  live.current = lighting;
  const down = useRef(false);

  // Animate colours without re-rendering React: update fills directly.
  useEffect(() => {
    let raf = 0;
    const t0 = performance.now();
    const tick = () => {
      const t = (performance.now() - t0) / 1000;
      const nodes = svg.current?.querySelectorAll<SVGGElement>('g[data-key]') ?? [];
      nodes.forEach((g) => {
        const slot = g.dataset.slot ? Number(g.dataset.slot) : null;
        const [r, gg, b] = previewColor(live.current, slot, Number(g.dataset.x), Number(g.dataset.y), t);
        const css = `rgb(${r | 0},${gg | 0},${b | 0})`;
        g.style.setProperty('--led', css);
      });
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  }, []);

  useEffect(() => {
    const up = () => (down.current = false);
    window.addEventListener('pointerup', up);
    return () => window.removeEventListener('pointerup', up);
  }, []);

  const paint = (slot: number | null) => {
    if (painting && slot != null) onPaint?.(slot);
  };

  return (
    <svg
      ref={svg}
      className={'keyboard' + (painting ? ' painting' : '') + (onSelectKey ? ' selecting' : '')}
      viewBox={`-12 -12 ${BOARD_W * U + 24} ${BOARD_H * U + 24}`}
      role="img"
      aria-label="Teclado K70 RGB MK.2 Low Profile"
    >
      <defs>
        <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="7" />
        </filter>
      </defs>
      <rect className="chassis" x={-12} y={-12} width={BOARD_W * U + 24} height={BOARD_H * U + 24} rx={18} />
      {KEYS.map((key) => {
        const slot = ledmap[key.id] ?? null;
        const x = key.x * U, y = key.y * U, w = key.w * U - GAP, h = key.h * U - GAP;
        return (
          <g
            key={key.id}
            data-key={key.id}
            data-slot={slot ?? ''}
            data-x={(key.x + key.w / 2) / BOARD_W}
            data-y={(key.y + key.h / 2) / BOARD_H}
            className={[slot == null ? 'unmapped' : '', selectedKey === key.id ? 'selected' : '', marked?.has(key.id) ? 'marked' : ''].join(' ')}
            onPointerDown={() => {
              if (onSelectKey) return onSelectKey(key.id);
              down.current = true;
              paint(slot);
            }}
            onPointerEnter={() => down.current && paint(slot)}
          >
            <title>{key.label || key.id}{slot == null ? ' (sem LED mapeado)' : ''}</title>
            <rect className="glow" x={x + 4} y={y + 4} width={w - 8} height={h - 8} rx={6} filter="url(#glow)" />
            <rect className="cap" x={x} y={y} width={w} height={h} rx={7} />
            {GLYPHS[key.label] ? (
              <path
                className="legend-glyph"
                d={GLYPHS[key.label]}
                transform={`translate(${x + w / 2 - (key.small ? 7 : 10)} ${y + h / 2 - (key.small ? 7 : 10)}) scale(${key.small ? 0.58 : 0.83})`}
              />
            ) : (
              <text className={'legend' + (key.small ? ' small' : '')} x={x + w / 2} y={y + h / 2}>
                {key.label}
              </text>
            )}
            {marked?.has(key.id) && <circle className="badge" cx={x + w - 8} cy={y + 8} r={4} />}
          </g>
        );
      })}
    </svg>
  );
}

/** Normalised key centres per LED slot, sent to the engine for spatial effects. */
export function geometryFor(ledmap: Record<string, number | null>): Record<number, [number, number]> {
  const out: Record<number, [number, number]> = {};
  for (const key of KEYS) {
    const slot = ledmap[key.id];
    if (slot != null) out[slot] = [(key.x + key.w / 2) / BOARD_W, (key.y + key.h / 2) / BOARD_H];
  }
  return out;
}
