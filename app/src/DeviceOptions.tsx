import { api, type Status } from './api';
import { LAYOUTS, type KbLayout } from './layout';
import { POLLING, useAppSettings } from './settings';
import { Switch } from './controls';



const SLEEP = [0, 1, 5, 10, 15, 30, 60];


type MemoryDevice = 'keyboard' | 'mouse' | 'headset';
const MEMORY_KEY = { keyboard: 'memory_keyboard', mouse: 'memory_mouse', headset: 'memory_headset' } as const;
const MEMORY_LOSES: Record<MemoryDevice, string> = {
  keyboard: 'Enquanto isso, as atribuições de tecla, as macros e a tecla Perfil não funcionam; o Win Lock e o brilho ficam com o próprio teclado.',
  mouse: 'Enquanto isso, as atribuições de botão não funcionam e o DPI fica com o próprio mouse.',
  headset: 'A bateria continua aparecendo e o equalizador continua valendo; a luz de status e o desligamento automático ficam com o próprio headset.',
};

/** Device Memory Mode switch: the device runs on what it keeps itself, in every profile. */
export function MemoryRow({ device }: { device: MemoryDevice }) {
  const [s, patch] = useAppSettings();
  const key = MEMORY_KEY[device];
  const on = !!s?.[key];
  return (
    <div className="list-row">
      <div>
        <strong>Modo Memória</strong>
        <p className="muted">
          O aparelho usa a iluminação e os ajustes guardados nele, como com o app fechado. Os perfis, as cenas e o app deixam ele de lado,
          em todos os perfis. {MEMORY_LOSES[device]}
        </p>
      </div>
      <Switch on={on} disabled={!s} onClick={() => patch({ [key]: !on })} />
    </div>
  );
}

/** Note on the device's other pages while it is in Memory Mode. */
export function MemoryNote({ device, status }: { device: MemoryDevice; status: Status | null }) {
  if (!status?.[device]?.memory) return null;
  return (
    <div className="memory-note" role="status">
      <p>
        <strong>Modo Memória ligado.</strong> O aparelho está usando a iluminação e os ajustes guardados nele. O que você mudar aqui fica salvo
        no perfil e vale quando desligar o Modo Memória.
      </p>
      <button className="ghost" onClick={() => api.patchAppSettings({ [MEMORY_KEY[device]]: false })}>
        Desligar
      </button>
    </div>
  );
}

/** Win Lock and polling rate: kept by the keyboard itself, also with the app closed. */
export function KeyboardOptions({ status, onLayout }: { status: Status | null; onLayout: (l: KbLayout) => void }) {
  const [s, patch] = useAppSettings();
  const setLayout = (l: KbLayout) => {
    onLayout(l);
    patch({ keyboard_layout: l });
  };
  // The Win Lock key toggles it on the keyboard: the live state comes with the status.
  const winLock = status?.win_lock ?? s?.win_lock ?? false;
  const combos = [
    { key: 'lock_alt_tab', label: 'Alt+Tab' },
    { key: 'lock_alt_f4', label: 'Alt+F4' },
    { key: 'lock_shift_tab', label: 'Shift+Tab' },
  ] as const;
  return (
    <section className="list">
      <MemoryRow device="keyboard" />
      <div className="list-row">
        <div>
          <strong>Layout do teclado</strong>
          <p className="muted">
            O formato do seu K70: muda o desenho e o nome das teclas. Para digitar, o sistema continua usando o layout configurado nele.
          </p>
        </div>
        <div className="seg" role="group" aria-label="Layout do teclado">
          {LAYOUTS.map(([id, label]) => (
            <button key={id} className={s?.keyboard_layout === id ? 'on' : ''} disabled={!s} onClick={() => setLayout(id)}>
              {label}
            </button>
          ))}
        </div>
      </div>
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
      <MemoryRow device="headset" />
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
