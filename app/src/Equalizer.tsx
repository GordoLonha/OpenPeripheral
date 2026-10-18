import { useEffect, useMemo, useRef, useState } from 'react';
import { api, type Eq, type EqBackend } from './api';

export const FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const Q = 1.41;
const MAX = 12;

const PRESETS: { id: string; name: string; gains: number[] }[] = [
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

const W = 640, H = 200;
const xOf = (f: number) => (Math.log10(f / 20) / Math.log10(20000 / 20)) * W;
const yOf = (db: number) => H / 2 - (db / (MAX + 3)) * (H / 2);

export function Equalizer() {
  const [eq, setEq] = useState<Eq | null>(null);
  const [backend, setBackend] = useState<EqBackend | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    api.eq().then(setEq);
    api.eqBackend().then(setBackend);
    const un = api.onEq(setEq);
    return () => {
      un.then((f) => f());
    };
  }, []);

  const push = (next: Eq) => {
    setEq(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      api.setEq(next).then(() => setError(null), (e) => setError(String(e)));
    }, 120);
  };

  const curve = useMemo(() => {
    if (!eq) return '';
    const pts: string[] = [];
    for (let i = 0; i <= 160; i++) {
      const f = 20 * Math.pow(1000, i / 160);
      const db = eq.gains.reduce((s, g, k) => s + peakDb(f, FREQS[k], g), 0);
      pts.push(`${xOf(f).toFixed(1)},${yOf(db).toFixed(1)}`);
    }
    return 'M' + pts.join(' L');
  }, [eq]);

  if (!eq || !backend) return null;

  if (!backend.available) {
    return (
      <div className="panel">
        <h2>Equalizador</h2>
        <p className="muted">{backend.detail}</p>
      </div>
    );
  }

  return (
    <div className="panel eq">
      <div className="eq-head">
        <h2>Equalizador</h2>
        <button
          role="switch"
          aria-checked={eq.enabled}
          aria-label="Ativar equalizador"
          className={'switch' + (eq.enabled ? ' on' : '')}
          onClick={() => push({ ...eq, enabled: !eq.enabled })}
        >
          <span />
        </button>
      </div>

      <div className={'eq-body' + (eq.enabled ? '' : ' off')}>
        <div className="chips">
          {PRESETS.map((p) => (
            <button
              key={p.id}
              className={'chip' + (eq.preset === p.id ? ' on' : '')}
              onClick={() => push({ ...eq, enabled: true, preset: p.id, gains: [...p.gains] })}
            >
              {p.name}
            </button>
          ))}
          {eq.preset === 'custom' && <span className="chip on">Personalizado</span>}
        </div>

        <svg className="eq-curve" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden>
          {[-12, -6, 0, 6, 12].map((db) => (
            <line key={db} x1={0} x2={W} y1={yOf(db)} y2={yOf(db)} className={db === 0 ? 'zero' : 'grid'} />
          ))}
          {FREQS.map((f) => (
            <line key={f} x1={xOf(f)} x2={xOf(f)} y1={0} y2={H} className="grid" />
          ))}
          <path d={`${curve} L${W},${H} L0,${H} Z`} className="fill" />
          <path d={curve} className="line" />
        </svg>

        <div className="eq-bands">
          {FREQS.map((f, i) => (
            <label key={f} className="band">
              <output>{eq.gains[i] > 0 ? '+' : ''}{eq.gains[i].toFixed(1)}</output>
              <input
                type="range"
                min={-MAX}
                max={MAX}
                step={0.5}
                value={eq.gains[i]}
                aria-label={`${f} Hz`}
                onChange={(e) => {
                  const gains = [...eq.gains];
                  gains[i] = Number(e.target.value);
                  push({ ...eq, enabled: true, preset: 'custom', gains });
                }}
              />
              <span>{f >= 1000 ? `${f / 1000}k` : f}</span>
            </label>
          ))}
        </div>
      </div>
      <p className="muted small">
        {backend.kind === 'pipewire'
          ? 'Aplicado por um filtro do PipeWire na frente do HS80 (vira a saída de som padrão enquanto ligado).'
          : 'Aplicado pelo Equalizer APO no HS80.'}{' '}
        {error && <span className="err">Erro: {error}</span>}
      </p>
    </div>
  );
}
