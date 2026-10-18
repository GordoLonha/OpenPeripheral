import { type Status } from './api';
import { POLLING, useAppSettings } from './settings';
import { Switch } from './controls';



const SLEEP = [0, 1, 5, 10, 15, 30, 60];


/** Win Lock and polling rate: kept by the keyboard itself, also with the app closed. */
export function KeyboardOptions({ status }: { status: Status | null }) {
  const [s, patch] = useAppSettings();
  // The Win Lock key toggles it on the keyboard: the live state comes with the status.
  const winLock = status?.win_lock ?? s?.win_lock ?? false;
  const combos = [
    { key: 'lock_alt_tab', label: 'Alt+Tab' },
    { key: 'lock_alt_f4', label: 'Alt+F4' },
    { key: 'lock_shift_tab', label: 'Shift+Tab' },
  ] as const;
  return (
    <section className="list">
      <div className="list-row">
        <div>
          <strong>Win Lock</strong>
          <p className="muted">
            Bloqueia a tecla Windows (e o que marcar abaixo) no próprio teclado: continua valendo com o app fechado. A tecla do
            cadeado liga e desliga.
          </p>
          <div className="chips">
            {combos.map((c) => (
              <label key={c.key} className="check">
                <input type="checkbox" checked={!!s?.[c.key]} disabled={!s} onChange={(e) => patch({ [c.key]: e.target.checked })} />
                também {c.label}
              </label>
            ))}
          </div>
        </div>
        <Switch on={winLock} disabled={!s} onClick={() => patch({ win_lock: !winLock })} />
      </div>
      <div className="list-row">
        <div>
          <strong>Taxa de polling</strong>
          <p className="muted">Com que frequência o teclado envia as teclas. Ao trocar, ele se reconecta por um instante.</p>
        </div>
        <div className="seg" role="group" aria-label="Taxa de polling">
          {POLLING.map((p) => (
            <button key={p.ms} className={s?.polling_ms === p.ms ? 'on' : ''} disabled={!s} onClick={() => patch({ polling_ms: p.ms })}>
              {p.label}
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}

/** Headset settings: sleep timer and mic mute. */
export function HeadsetOptions({ status }: { status: Status | null }) {
  const [s, patch] = useAppSettings();
  const hs = status?.headset;
  const muted = hs?.mic_muted;
  return (
    <section className="list">
      <div className="list-row">
        <div>
          <strong>Microfone</strong>
          <p className="muted">
            {muted == null ? 'Estado desconhecido' : muted ? 'Mudo (haste levantada) — LED da ponta em vermelho' : 'Ligado'}
          </p>
        </div>
      </div>
      <div className="list-row">
        <div>
          <strong>Mutar o microfone do sistema junto</strong>
          <p className="muted">Ao levantar a haste, o microfone padrão do sistema também fica mudo.</p>
        </div>
        <Switch on={!!s?.mic_sync} disabled={!s} onClick={() => patch({ mic_sync: !s?.mic_sync })} />
      </div>
      <div className="list-row">
        <div>
          <strong>Desligar sozinho</strong>
          <p className="muted">Depois de quanto tempo parado o fone se desliga para poupar bateria.</p>
        </div>
        <label className="field">
          <select
            value={s?.headset_sleep_min ?? ''}
            disabled={!s}
            onChange={(e) => patch({ headset_sleep_min: e.target.value === '' ? null : Number(e.target.value) })}
          >
            <option value="">Não alterar</option>
            {SLEEP.map((m) => (
              <option key={m} value={m}>
                {m === 0 ? 'Nunca' : m === 60 ? '1 hora' : `${m} min`}
              </option>
            ))}
          </select>
        </label>
      </div>
    </section>
  );
}
