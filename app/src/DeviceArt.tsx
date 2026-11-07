// Device drawings with their live lighting, for the home cards, the device strip
// and the device pages. Colours come from preview.ts, like the keyboard drawing.
import { useEffect, useRef, type RefObject } from 'react';
import type { Lighting, MouseOptions, RGB } from './api';
import { Keyboard } from './Keyboard';
import { previewColor } from './preview';
import { useAppSettings } from './settings';

export type DeviceId = 'keyboard' | 'headset' | 'mouse';

/** Sets `--<name>` CSS colours on the element every frame, without re-rendering React. */
function useLiveColors(ref: RefObject<SVGSVGElement | null>, compute: (t: number) => Record<string, RGB>) {
  const fn = useRef(compute);
  fn.current = compute;
  useEffect(() => {
    let raf = 0;
    const t0 = performance.now();
    const tick = () => {
      const el = ref.current;
      if (el) {
        const colors = fn.current((performance.now() - t0) / 1000);
        for (const [name, [r, g, b]] of Object.entries(colors)) el.style.setProperty(`--${name}`, `rgb(${r | 0},${g | 0},${b | 0})`);
      }
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  }, [ref]);
}

/** The headset's logo shows the keyboard effect at the keyboard's centre (effects::headset). */
const centre = (l: Lighting, t: number) => previewColor(l, null, 0.5, 0.5, t);

export function HeadsetArt({ lighting }: { lighting: Lighting }) {
  const ref = useRef<SVGSVGElement>(null);
  useLiveColors(ref, (t) => ({ led: centre(lighting, t) }));
  return (
    <svg ref={ref} className="art headset-art" viewBox="0 0 220 180" role="img" aria-label="Headset HS80 RGB">
      <path className="hs-band" d="M42 104V86a68 68 0 0 1 136 0v18" />
      <path className="hs-band-in" d="M50 100V88a60 60 0 0 1 120 0v12" />
      <path className="boom" d="M52 140q10 22 34 30" />
      <circle className="boom-tip" cx="88" cy="171" r="4" />
      {[30, 158].map((x) => (
        <g key={x}>
          <rect className="cup" x={x} y="92" width="32" height="64" rx="12" />
          <circle className="ring-glow" cx={x + 16} cy="124" r="13" />
          <circle className="ring-led" cx={x + 16} cy="124" r="10" />
          <circle className="ring-core" cx={x + 16} cy="124" r="5" />
        </g>
      ))}
    </svg>
  );
}

/** M65: logo and wheel follow the keyboard or have their own effect (mouse.rs zones).
 *  `zones` overrides the saved ones (the mouse page passes what is being edited). */
export function MouseArt({ keyboard, zones: edited }: { keyboard: Lighting; zones?: MouseOptions['lighting'] }) {
  const [settings] = useAppSettings();
  const ref = useRef<SVGSVGElement>(null);
  const zones = edited ?? settings?.mouse.lighting;
  useLiveColors(ref, (t) => {
    if (!zones || zones.follow_keyboard) {
      const c = centre(keyboard, t);
      return { logo: c, wheel: c };
    }
    return { logo: centre(zones.logo, t), wheel: centre(zones.wheel, t) };
  });
  return (
    <svg ref={ref} className="art mouse-art" viewBox="0 0 120 190" role="img" aria-label="Mouse M65 RGB Elite">
      <path className="ms-body" d="M60 8C30 8 14 34 14 70v58c0 34 20 54 46 54s46-20 46-54V70C106 34 90 8 60 8z" />
      <path className="split" d="M60 8v62M14 70h92" />
      <rect className="wheel-glow" x="53" y="26" width="14" height="30" rx="7" />
      <rect className="wheel" x="55" y="28" width="10" height="26" rx="5" />
      <path className="logo-glow" d="M60 118l14 10-14 22-14-22z" />
      <path className="logo" d="M60 122l10 7-10 16-10-16z" />
      <circle className="sniper" cx="20" cy="104" r="4" />
    </svg>
  );
}

export function KeyboardArt({ lighting, ledmap }: { lighting: Lighting; ledmap: Record<string, number | null> }) {
  return (
    <div className="art keyboard-art" aria-hidden>
      <Keyboard lighting={lighting} ledmap={ledmap} painting={false} />
    </div>
  );
}

/** `headset`: the headset's own lighting, when it does not follow the keyboard. */
export function DeviceArt({ id, lighting, ledmap, headset }: { id: DeviceId; lighting: Lighting; ledmap: Record<string, number | null>; headset?: Lighting | null }) {
  if (id === 'keyboard') return <KeyboardArt lighting={lighting} ledmap={ledmap} />;
  if (id === 'headset') return <HeadsetArt lighting={headset ?? lighting} />;
  return <MouseArt keyboard={lighting} />;
}

/** A small grid painted with a lighting, animated: the swatch of a scene. */
export function LightSwatch({ lighting, cols = 16, rows = 6 }: { lighting: Lighting; cols?: number; rows?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const live = useRef(lighting);
  live.current = lighting;
  useEffect(() => {
    let raf = 0;
    const t0 = performance.now();
    const tick = () => {
      const ctx = canvas.current?.getContext('2d');
      if (ctx) {
        const t = (performance.now() - t0) / 1000;
        for (let gx = 0; gx < cols; gx++) {
          for (let gy = 0; gy < rows; gy++) {
            const [r, g, b] = previewColor(live.current, null, (gx + 0.5) / cols, (gy + 0.5) / rows, t);
            ctx.fillStyle = `rgb(${r | 0},${g | 0},${b | 0})`;
            ctx.fillRect(gx, gy, 1, 1);
          }
        }
      }
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  }, [cols, rows]);
  return <canvas ref={canvas} className="light-swatch" width={cols} height={rows} aria-hidden />;
}
