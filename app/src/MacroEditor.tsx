import { useEffect, useRef, useState } from 'react';
import type { MacroOptions, MacroStep } from './api';
import { Icon } from './icons';
import { Switch } from './controls';

const MOUSE_BUTTONS = ['left', 'middle', 'right', 'back', 'forward'];
const MOUSE_NAMES: Record<string, string> = {
  left: 'Clique esquerdo',
  right: 'Clique direito',
  middle: 'Clique do meio',
  back: 'Voltar',
  forward: 'Avançar',
};

const TRIGGERS: { id: MacroOptions['trigger']; name: string }[] = [
  { id: 'press', name: 'Ao pressionar' },
  { id: 'release', name: 'Ao soltar' },
  { id: 'hold', name: 'Enquanto pressionado' },
  { id: 'toggle', name: 'Alternar (aperta liga, aperta desliga)' },
];

const REPEATS: { id: MacroOptions['repeat']; name: string }[] = [
  { id: 'once', name: 'Sem repetição' },
  { id: 'times', name: 'Repetir algumas vezes' },
  { id: 'continuous', name: 'Repetir continuamente' },
];

type Props = {
  steps: MacroStep[];
  options: MacroOptions;
  onChange: (steps: MacroStep[], options: MacroOptions) => void;
  /** Turns a recorded key name into a label. */
  keyName: (key: string) => string | null;
  /** Browser key event -> key name understood by the backend. */
  keyFromEvent: (e: KeyboardEvent) => string | null;
};

/** iCUE-style macro editor: Gravando / Geral / Avançado. */
export function MacroEditor({ steps, options, onChange, keyName, keyFromEvent }: Props) {
  const [tab, setTab] = useState<'rec' | 'general' | 'advanced'>('rec');
  const [rec, setRec] = useState(false);
  const [capture, setCapture] = useState({ delays: true, keys: true, clicks: true, wheel: false });
  const buf = useRef<MacroStep[]>([]);
  const last = useRef(0);
  const setSteps = (s: MacroStep[]) => onChange(s, options);
  const setOpts = (p: Partial<MacroOptions>) => onChange(steps, { ...options, ...p });

  useEffect(() => {
    if (!rec) return;
    buf.current = [];
    last.current = performance.now();
    const push = (key: string, down: boolean) => {
      const now = performance.now();
      const delay = buf.current.length && capture.delays ? Math.round(now - last.current) : 0;
      buf.current.push({ key, down, delay_ms: delay });
      last.current = now;
      onChange([...buf.current], options);
    };
    const onKey = (down: boolean) => (e: KeyboardEvent) => {
      if (!capture.keys) return;
      e.preventDefault();
      if (e.repeat) return;
      const key = keyFromEvent(e);
      if (key) push(key, down);
    };
    const onPointer = (down: boolean) => (e: PointerEvent) => {
      // The click on "Parar" ends the recording; it is not part of it.
      if (!capture.clicks || (e.target as Element | null)?.closest?.('.rec-btn')) return;
      const name = MOUSE_BUTTONS[e.button];
      if (name) push(`Mouse:${name}`, down);
    };
    const onWheel = (e: WheelEvent) => {
      if (capture.wheel && e.deltaY !== 0) push(`Wheel:${e.deltaY < 0 ? 'up' : 'down'}`, true);
    };
    const kd = onKey(true), ku = onKey(false), pd = onPointer(true), pu = onPointer(false);
    window.addEventListener('keydown', kd, true);
    window.addEventListener('keyup', ku, true);
    window.addEventListener('pointerdown', pd, true);
    window.addEventListener('pointerup', pu, true);
    window.addEventListener('wheel', onWheel, true);
    return () => {
      window.removeEventListener('keydown', kd, true);
      window.removeEventListener('keyup', ku, true);
      window.removeEventListener('pointerdown', pd, true);
      window.removeEventListener('pointerup', pu, true);
      window.removeEventListener('wheel', onWheel, true);
    };
  }, [rec]);

  const move = (i: number, by: number) => {
    const j = i + by;
    if (j < 0 || j >= steps.length) return;
    const next = [...steps];
    [next[i], next[j]] = [next[j], next[i]];
    setSteps(next);
  };
  const label = (key: string) => {
    if (key.startsWith('Mouse:')) return MOUSE_NAMES[key.slice(6)] ?? key;
    if (key === 'Wheel:up') return 'Roda ↑';
    if (key === 'Wheel:down') return 'Roda ↓';
    return keyName(key) ?? key;
  };

  return (
    <>
      <div className="detail-head">
        <h3>Macro</h3>
        <div className="seg" role="tablist" aria-label="Macro">
          <button role="tab" aria-selected={tab === 'rec'} className={tab === 'rec' ? 'on' : ''} onClick={() => setTab('rec')}>Gravando</button>
          <button role="tab" aria-selected={tab === 'general'} className={tab === 'general' ? 'on' : ''} onClick={() => setTab('general')}>Geral</button>
          <button role="tab" aria-selected={tab === 'advanced'} className={tab === 'advanced' ? 'on' : ''} onClick={() => setTab('advanced')}>Avançado</button>
        </div>
      </div>

      {tab === 'rec' && (
        <>
          <div className="macro-toolbar">
            <button className={'rec-btn' + (rec ? ' on' : '')} onClick={() => setRec(!rec)}>
              <Icon name="record" size={14} /> {rec ? 'Parar' : 'Gravar'}
            </button>
            {steps.length > 0 && !rec && (
              <button className="ghost" onClick={() => setSteps([])}>
                Limpar
              </button>
            )}
          </div>
          {steps.length === 0 ? (
            <p className="muted">Clique em Gravar e use o teclado e o mouse dentro deste app. Clique em Parar quando terminar.</p>
          ) : (
            <ol className="macro-list">
              {steps.map((s, i) => (
                <li key={i}>
                  <span className="n">{i + 1}</span>
                  <label className="delay-field" title="Pausa antes deste passo">
                    ⏱
                    <input
                      type="number"
                      min={0}
                      max={60000}
                      value={s.delay_ms}
                      aria-label={`Pausa antes do passo ${i + 1} (ms)`}
                      onChange={(e) => setSteps(steps.map((x, k) => (k === i ? { ...x, delay_ms: Math.max(0, Number(e.target.value)) } : x)))}
                    />
                    ms
                  </label>
                  <span className={s.down ? 'down' : 'up'}>{s.down ? '↓' : '↑'}</span>
                  <kbd>{label(s.key)}</kbd>
                  <span className="row-actions">
                    <button className="icon-btn" aria-label="Subir passo" disabled={i === 0} onClick={() => move(i, -1)}>▲</button>
                    <button className="icon-btn" aria-label="Descer passo" disabled={i === steps.length - 1} onClick={() => move(i, 1)}>▼</button>
                    <button className="icon-btn" aria-label="Remover passo" onClick={() => setSteps(steps.filter((_, j) => j !== i))}>
                      <Icon name="close" size={12} />
                    </button>
                  </span>
                </li>
              ))}
            </ol>
          )}
        </>
      )}

      {tab === 'general' && (
        <div className="macro-options">
          <p className="muted">O que a gravação registra:</p>
          {(
            [
              ['delays', 'Pausas entre os passos'],
              ['keys', 'Teclas do teclado'],
              ['clicks', 'Cliques do mouse'],
              ['wheel', 'Roda do mouse'],
            ] as const
          ).map(([k, name]) => (
            <div key={k} className="setting">
              <span>{name}</span>
              <Switch on={capture[k]} onClick={() => setCapture({ ...capture, [k]: !capture[k] })} />
            </div>
          ))}
        </div>
      )}

      {tab === 'advanced' && (
        <div className="macro-options">
          <div className="setting">
            <span>Manter a função original da tecla</span>
            <Switch on={options.passthrough} onClick={() => setOpts({ passthrough: !options.passthrough })} />
          </div>
          <label className="field">
            Acionamento
            <select value={options.trigger} onChange={(e) => setOpts({ trigger: e.target.value as MacroOptions['trigger'] })}>
              {TRIGGERS.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          {(options.trigger === 'press' || options.trigger === 'release') && (
            <label className="check">
              <input type="checkbox" checked={options.uninterruptible} onChange={(e) => setOpts({ uninterruptible: e.target.checked })} />
              Executar ininterruptamente (apertar de novo não reinicia)
            </label>
          )}
          <label className="field">
            Repetição
            <select value={options.repeat} onChange={(e) => setOpts({ repeat: e.target.value as MacroOptions['repeat'] })}>
              {REPEATS.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
          </label>
          {options.repeat === 'times' && (
            <label className="field">
              Quantas vezes
              <input type="number" min={1} max={1000} value={options.times} onChange={(e) => setOpts({ times: Math.max(1, Number(e.target.value)) })} />
            </label>
          )}
          {options.repeat === 'continuous' && (
            <p className="muted small">
              {options.trigger === 'hold'
                ? 'Repete enquanto a tecla estiver pressionada.'
                : 'Repete até apertar a tecla de novo.'}
            </p>
          )}
          <label className="field">
            Som ao iniciar (arquivo .wav, opcional)
            <input
              type="text"
              placeholder="/home/…/som.wav"
              value={options.sound ?? ''}
              onChange={(e) => setOpts({ sound: e.target.value.trim() ? e.target.value : null })}
            />
          </label>
        </div>
      )}
    </>
  );
}
