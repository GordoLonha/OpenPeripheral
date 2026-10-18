import { useEffect, useRef, useState } from 'react';
import { api, fromHex, toHex, type DpiStage, type Effect, type Lighting, type MouseOptions, type Status } from './api';
import { ColorField, Slider, Switch, Tabs } from './controls';
import { POLLING, useAppSettings } from './settings';
import { MouseButtons } from './MouseButtons';

const DPI_MIN = 100;
const DPI_MAX = 18000;
const DPI_STEP = 50;

const ZONE_EFFECTS: { id: Effect; name: string; two?: boolean }[] = [
  { id: 'static', name: 'Cor fixa' },
  { id: 'rainbow', name: 'Arco-íris' },
  { id: 'wave', name: 'Onda', two: true },
  { id: 'breathing', name: 'Respiração', two: true },
];

/** M65 RGB Elite: DPI stages, sniper, sensor options and lighting per zone. */
export function MousePage({ status, profiles }: { status: Status | null; profiles: { id: string; name: string }[] }) {
  const [settings, patch] = useAppSettings();
  const [opts, setOpts] = useState<MouseOptions | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const mouse = status?.mouse;
  const [tab, setTab] = useState<'dpi' | 'buttons' | 'light' | 'settings'>('dpi');

  useEffect(() => {
    if (settings && !opts) setOpts(settings.mouse);
  }, [settings, opts]);

  // Sliders fire on every step: show at once, send once the hand stops.
  const change = (next: MouseOptions) => {
    setOpts(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => patch({ mouse: next }), 150);
  };
  const setStage = (i: number, s: DpiStage) => opts && change({ ...opts, stages: opts.stages.map((o, k) => (k === i ? s : o)) });

  if (!opts) return null;
  const active = mouse?.dpi_stage ?? settings?.mouse_stage ?? 0;
  const zone = (key: 'logo' | 'wheel', l: Lighting) => change({ ...opts, lighting: { ...opts.lighting, [key]: l } });

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Mouse</h1>
          <p className="page-sub">{mouse ? `${mouse.name} · firmware ${mouse.firmware}` : 'M65 RGB Elite desconectado'}</p>
        </div>
        <Tabs
          label="Seção do mouse"
          value={tab}
          onChange={setTab}
          options={[['dpi', 'DPI'], ['buttons', 'Botões'], ['light', 'Iluminação'], ['settings', 'Ajustes']]}
        />
      </header>

      {tab === 'dpi' && (
        <section className="card">
          <div className="card-head">
            <div>
              <h2 className="card-title">Estágios de DPI</h2>
              <p className="muted small">Os botões atrás da roda trocam o estágio, e o LED do DPI mostra a cor dele. Clique no nome para usar um estágio.</p>
            </div>
          </div>
          <div className="dpi-stages">
            {opts.stages.map((s, i) => (
              <StageRow
                key={i}
                label={`Estágio ${i + 1}`}
                stage={s}
                active={i === active}
                onSelect={() => api.setMouseStage(i)}
                onChange={(n) => setStage(i, n)}
              />
            ))}
            <StageRow label="Sniper" stage={opts.sniper} hint="Enquanto segura o botão vermelho" onChange={(n) => change({ ...opts, sniper: n })} />
          </div>
        </section>
      )}

      {tab === 'buttons' && <MouseButtons profiles={profiles} />}

      {tab === 'light' && (
        <section className="card">
          <div className="card-head">
            <div>
              <h2 className="card-title">Iluminação</h2>
              <p className="muted small">
                {opts.lighting.follow_keyboard ? 'Logo e roda usam o mesmo efeito do teclado no perfil em uso.' : 'Cada zona com o seu efeito.'}
              </p>
            </div>
            <div className="seg" role="group" aria-label="Iluminação do mouse">
              <button className={opts.lighting.follow_keyboard ? 'on' : ''} onClick={() => change({ ...opts, lighting: { ...opts.lighting, follow_keyboard: true } })}>
                Seguir o teclado
              </button>
              <button className={opts.lighting.follow_keyboard ? '' : 'on'} onClick={() => change({ ...opts, lighting: { ...opts.lighting, follow_keyboard: false } })}>
                Por zona
              </button>
            </div>
          </div>
          {!opts.lighting.follow_keyboard && (
            <div className="zones">
              <ZoneEditor title="Logo" value={opts.lighting.logo} onChange={(l) => zone('logo', l)} />
              <ZoneEditor title="Roda" value={opts.lighting.wheel} onChange={(l) => zone('wheel', l)} />
            </div>
          )}
        </section>
      )}

      {tab === 'settings' && (
        <section className="list">
          <div className="list-row">
            <div>
              <strong>Taxa de polling</strong>
              <p>Com que frequência o mouse envia o movimento. Ao trocar, ele se reconecta por um instante.</p>
            </div>
            <div className="seg" role="group" aria-label="Taxa de polling do mouse">
              {POLLING.map((p) => (
                <button key={p.ms} className={opts.polling_ms === p.ms ? 'on' : ''} onClick={() => change({ ...opts, polling_ms: p.ms })}>
                  {p.label}
                </button>
              ))}
            </div>
          </div>
          <div className="list-row">
            <div>
              <strong>Correção de ângulo</strong>
              <p>Endireita movimentos quase retos (útil para desenhar linhas).</p>
            </div>
            <Switch on={opts.angle_snapping} onClick={() => change({ ...opts, angle_snapping: !opts.angle_snapping })} />
          </div>
          <div className="list-row">
            <div>
              <strong>Otimização de resposta dos botões</strong>
              <p>Cliques registrados mais rápido.</p>
            </div>
            <Switch on={opts.button_optimization} onClick={() => change({ ...opts, button_optimization: !opts.button_optimization })} />
          </div>
        </section>
      )}
    </>
  );
}

function StageRow(p: { label: string; stage: DpiStage; active?: boolean; hint?: string; onSelect?: () => void; onChange: (s: DpiStage) => void }) {
  const linked = p.stage.x === p.stage.y;
  const [split, setSplit] = useState(!linked);
  const clamp = (v: number) => Math.min(DPI_MAX, Math.max(DPI_MIN, Math.round(v / DPI_STEP) * DPI_STEP));
  const setX = (v: number) => p.onChange({ ...p.stage, x: clamp(v), y: split ? p.stage.y : clamp(v) });
  return (
    <div className={'dpi-row' + (p.active ? ' active' : '')}>
      {p.onSelect ? (
        <button className="dpi-label" title="Usar este estágio" onClick={p.onSelect}>
          {p.label}
          {p.active && <span className="badge">em uso</span>}
        </button>
      ) : (
        <span className="dpi-label" title={p.hint}>
          {p.label}
        </span>
      )}
      <input type="range" min={DPI_MIN} max={DPI_MAX} step={DPI_STEP} value={p.stage.x} aria-label={`DPI ${p.label}`} onChange={(e) => setX(Number(e.target.value))} />
      <input className="dpi-num" type="number" min={DPI_MIN} max={DPI_MAX} step={DPI_STEP} value={p.stage.x} aria-label={`DPI X ${p.label}`} onChange={(e) => setX(Number(e.target.value))} />
      {split && (
        <input
          className="dpi-num"
          type="number"
          min={DPI_MIN}
          max={DPI_MAX}
          step={DPI_STEP}
          value={p.stage.y}
          aria-label={`DPI Y ${p.label}`}
          onChange={(e) => p.onChange({ ...p.stage, y: clamp(Number(e.target.value)) })}
        />
      )}
      <label className="check" title="DPI diferente na vertical">
        <input
          type="checkbox"
          checked={split}
          onChange={(e) => {
            setSplit(e.target.checked);
            if (!e.target.checked) p.onChange({ ...p.stage, y: p.stage.x });
          }}
        />
        X/Y
      </label>
      <input type="color" className="color-dot" aria-label={`Cor ${p.label}`} value={toHex(p.stage.color)} onChange={(e) => p.onChange({ ...p.stage, color: fromHex(e.target.value) })} />
    </div>
  );
}

function ZoneEditor({ title, value, onChange }: { title: string; value: Lighting; onChange: (l: Lighting) => void }) {
  const fx = ZONE_EFFECTS.find((e) => e.id === value.effect) ?? ZONE_EFFECTS[0];
  const set = (p: Partial<Lighting>) => onChange({ ...value, ...p });
  return (
    <div className="zone panel">
      <h3>{title}</h3>
      <div className="chips">
        {ZONE_EFFECTS.map((e) => (
          <button key={e.id} className={'chip' + (e.id === value.effect ? ' on' : '')} onClick={() => set({ effect: e.id })}>
            {e.name}
          </button>
        ))}
      </div>
      <div className="colors">
        {value.effect !== 'rainbow' && <ColorField label={fx.two ? 'Cor 1' : 'Cor'} value={value.color} onChange={(c) => set({ color: c })} />}
        {fx.two && <ColorField label="Cor 2" value={value.color2} onChange={(c) => set({ color2: c })} />}
      </div>
      <Slider label="Brilho" value={value.brightness} min={0} max={100} step={1} unit="%" onChange={(v) => set({ brightness: v })} />
      {value.effect !== 'static' && <Slider label="Velocidade" value={value.speed} min={0.1} max={3} step={0.1} unit="×" onChange={(v) => set({ speed: v })} />}
    </div>
  );
}
