import type { Lighting, Profiles, Status } from './api';

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

type Props = {
  status: Status | null;
  profiles: Profiles | null;
  onOpen: (page: 'keyboard' | 'headset') => void;
  onScene: (patch: Partial<Lighting>) => void;
};

export function Home({ status, profiles, onOpen, onScene }: Props) {
  const hs = status?.headset;
  const active = profiles?.profiles.find((p) => p.id === profiles.active);
  return (
    <>
      <header className="page-head">
        <div>
          <h1>Início</h1>
          <p className="muted">
            Perfil em uso: <strong className="accent">{active?.name ?? '—'}</strong>
            {profiles?.auto_app && <> · ativado por {profiles.auto_app}</>}
          </p>
        </div>
      </header>

      <section className="device-cards">
        <button className="device-card" onClick={() => onOpen('keyboard')}>
          <KeyboardGlyph />
          <div>
            <strong>K70 RGB MK.2 Low Profile</strong>
            <small>{status?.keyboard ? `conectado · firmware ${status.keyboard.firmware}` : 'desconectado'}</small>
          </div>
          <span className={'dot' + (status?.keyboard ? ' online' : '')} aria-hidden />
        </button>
        <button className="device-card" onClick={() => onOpen('headset')}>
          <HeadsetGlyph />
          <div>
            <strong>HS80 RGB Wireless</strong>
            <small>
              {!hs ? 'receptor desconectado' : hs.battery == null ? 'headset desligado' : hs.charging ? 'carregando' : 'sem fio'}
            </small>
          </div>
          {hs?.battery != null && <BatteryPill pct={hs.battery} charging={hs.charging} />}
        </button>
        {status?.mouse && (
          <div className="device-card">
            <MouseGlyph />
            <div>
              <strong>{status.mouse.name}</strong>
              <small>
                firmware {status.mouse.firmware} · DPI estágio {status.mouse.dpi_stage + 1} · luz segue o teclado
              </small>
            </div>
            <span className="dot online" aria-hidden />
          </div>
        )}
      </section>

      <section>
        <h2>Cenas de iluminação</h2>
        <div className="scenes">
          {SCENES.map((s) => (
            <button key={s.id} className="scene" onClick={() => onScene(s.patch)}>
              <span className="scene-swatch" style={{ background: s.css }} aria-hidden />
              <span>{s.name}</span>
            </button>
          ))}
        </div>
        <p className="muted small">Aplica no perfil em uso. Ajuste fino em Teclado → Iluminação.</p>
      </section>
    </>
  );
}

export function batteryColor(pct: number, charging: boolean): string {
  if (charging) return 'var(--accent)';
  if (pct < 15) return 'var(--danger)';
  if (pct < 40) return 'var(--warn)';
  return 'var(--ok)';
}

export function BatteryPill({ pct, charging }: { pct: number; charging: boolean }) {
  return (
    <span className="battery-pill" style={{ color: batteryColor(pct, charging) }}>
      <span className="battery-shape" aria-hidden>
        <span style={{ width: `${Math.max(8, pct)}%`, background: batteryColor(pct, charging) }} />
      </span>
      {pct.toFixed(0)}%{charging && ' ⚡'}
    </span>
  );
}

const KeyboardGlyph = () => (
  <svg className="device-glyph" viewBox="0 0 64 32" aria-hidden>
    <rect x="1" y="4" width="62" height="24" rx="4" />
    {Array.from({ length: 3 }, (_, r) =>
      Array.from({ length: 11 }, (_, c) => <rect key={`${r}-${c}`} className="k" x={5 + c * 5} y={8 + r * 5} width="3.6" height="3.6" rx="0.8" />),
    )}
    <rect className="k" x="15" y="23" width="30" height="3" rx="0.8" />
  </svg>
);

const MouseGlyph = () => (
  <svg className="device-glyph" viewBox="0 0 64 32" aria-hidden>
    <rect x="24" y="2" width="16" height="28" rx="8" fill="none" strokeWidth="3" />
    <path d="M32 2v10" fill="none" strokeWidth="3" />
  </svg>
);

const HeadsetGlyph = () => (
  <svg className="device-glyph" viewBox="0 0 64 32" aria-hidden>
    <path d="M18 22v-6a14 14 0 0 1 28 0v6" fill="none" strokeWidth="3" />
    <rect x="13" y="18" width="8" height="12" rx="3" />
    <rect x="43" y="18" width="8" height="12" rx="3" />
  </svg>
);

