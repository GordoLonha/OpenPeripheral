import { useEffect, useRef, useState } from 'react';
import { api, type Effect, type Lighting } from './api';
import { ColorField, Slider } from './controls';
import { EFFECTS } from './fx';

/** Effects that make sense on a single light (mouse zones, headset logo): no key presses. */
const ZONE_EFFECTS: Effect[] = ['static', 'rainbow', 'wave', 'breathing', 'colorshift', 'temperature'];

/** Effect, colours, brightness and speed of one light. */
export function ZoneEditor({ title, value, onChange }: { title: string; value: Lighting; onChange: (l: Lighting) => void }) {
  const list = EFFECTS.filter((e) => ZONE_EFFECTS.includes(e.id));
  const fx = list.find((e) => e.id === value.effect) ?? list[0];
  const set = (p: Partial<Lighting>) => onChange({ ...value, ...p });
  return (
    <div className="zone panel">
      <h3>{title}</h3>
      <div className="chips">
        {list.map((e) => (
          <button key={e.id} className={'chip' + (e.id === value.effect ? ' on' : '')} onClick={() => set({ effect: e.id })}>
            {e.name}
          </button>
        ))}
      </div>
      <div className="colors">
        {fx.colors.length > 0 && <ColorField label={fx.colors[0]} value={value.color} onChange={(c) => set({ color: c })} />}
        {fx.colors.length > 1 && <ColorField label={fx.colors[1]} value={value.color2} onChange={(c) => set({ color2: c })} />}
      </div>
      <Slider label="Brilho" value={value.brightness} min={0} max={100} step={1} unit="%" onChange={(v) => set({ brightness: v })} />
      {!fx.still && <Slider label="Velocidade" value={value.speed} min={0.1} max={3} step={0.1} unit="×" onChange={(v) => set({ speed: v })} />}
    </div>
  );
}

/** Headset logo: follow the keyboard (default) or its own lighting, per profile. */
export function HeadsetLighting({ keyboard, onChange }: { keyboard: Lighting; onChange: (own: Lighting | null) => void }) {
  const [own, setOwn] = useState<Lighting | null | undefined>(undefined);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    api.headsetLighting().then((l) => {
      setOwn(l);
      onChange(l);
    });
    // Load once per mount (the page re-mounts on a profile change).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sliders fire on every step: show at once, save once the hand stops.
  const change = (next: Lighting | null) => {
    setOwn(next);
    onChange(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => api.setHeadsetLighting(next), 120);
  };

  if (own === undefined) return null;
  return (
    <section className="card">
      <div className="card-head">
        <div>
          <h2 className="card-title">Iluminação</h2>
          <p className="muted small">
            {own ? 'O logo do headset tem o seu próprio efeito neste perfil.' : 'O logo do headset mostra o mesmo efeito do teclado, no centro dele.'}
          </p>
        </div>
        <div className="seg" role="group" aria-label="Iluminação do headset">
          <button className={own ? '' : 'on'} onClick={() => change(null)}>
            Seguir o teclado
          </button>
          <button className={own ? 'on' : ''} onClick={() => change(own ?? { ...keyboard, effect: ZONE_EFFECTS.includes(keyboard.effect) ? keyboard.effect : 'static', painted: {} })}>
            Própria
          </button>
        </div>
      </div>
      {own && <ZoneEditor title="Logo" value={own} onChange={change} />}
      <p className="muted small">A luz de status mostra a bateria e a do microfone fica vermelha quando ele está mudo.</p>
    </section>
  );
}
