import { useEffect, useMemo, useRef, useState } from 'react';
import { api, type Eq, type EqBackend } from './api';
import { FREQS, H, MAX, PRESETS, W, eqPath, xOf, yOf } from './eqmath';

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

  const curve = useMemo(() => (eq ? eqPath(eq.gains) : ''), [eq]);

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
