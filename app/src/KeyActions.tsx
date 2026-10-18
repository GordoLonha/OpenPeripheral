import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { DEFAULT_MACRO_OPTIONS, type Action, type Bindings } from './api';
import { Icon, type IconName } from './icons';
import { KEYS } from './layout';
import { MacroEditor } from './MacroEditor';

type Kind = Action['kind'];

export const GROUPS: { title: string; items: { id: Kind; name: string; icon: IconName }[] }[] = [
  {
    title: 'Remapear',
    items: [
      { id: 'key', name: 'Tecla', icon: 'key' },
      { id: 'shortcut', name: 'Atalho', icon: 'combo' },
      { id: 'mouse', name: 'Mouse', icon: 'mouse' },
    ],
  },
  {
    title: 'Ações',
    items: [
      { id: 'text', name: 'Texto', icon: 'text' },
      { id: 'media', name: 'Mídia', icon: 'media' },
      { id: 'launch', name: 'Abrir app', icon: 'launch' },
      { id: 'macro', name: 'Macro', icon: 'macro' },
      { id: 'profile', name: 'Trocar perfil', icon: 'profile' },
      { id: 'disabled', name: 'Desativar', icon: 'disabled' },
    ],
  },
];
export const KIND_INFO = Object.fromEntries(GROUPS.flatMap((g) => g.items).map((i) => [i.id, i]));

// Names understood by app/src-tauri/src/remap.rs::key_from_name.
const TARGET_ROWS: { id: string; name: string; w?: number }[][] = [
  [{ id: 'Escape', name: 'Esc' }, ...Array.from({ length: 12 }, (_, i) => ({ id: `F${i + 1}`, name: `F${i + 1}` })),
    { id: 'PrintScreen', name: 'PrtSc' }, { id: 'Pause', name: 'Pause' }],
  [{ id: "'", name: "'" }, ...'1234567890'.split('').map((c) => ({ id: c, name: c })), { id: '-', name: '-' }, { id: '=', name: '=' },
    { id: 'Backspace', name: '⌫', w: 2 }, { id: 'Insert', name: 'Ins' }, { id: 'Home', name: 'Home' }, { id: 'PageUp', name: 'PgUp' }],
  [{ id: 'Tab', name: 'Tab', w: 1.5 }, ...'QWERTYUIOP'.split('').map((c) => ({ id: c, name: c })), { id: '[', name: '[' }, { id: ']', name: ']' },
    { id: 'Delete', name: 'Del' }, { id: 'End', name: 'End' }, { id: 'PageDown', name: 'PgDn' }],
  [{ id: 'CapsLock', name: 'Caps', w: 1.75 }, ...'ASDFGHJKL'.split('').map((c) => ({ id: c, name: c })), { id: 'ç', name: 'Ç' },
    { id: 'Enter', name: 'Enter', w: 1.75 }],
  [{ id: 'Shift', name: 'Shift', w: 2 }, { id: '\\', name: '\\' }, ...'ZXCVBNM'.split('').map((c) => ({ id: c, name: c })),
    { id: ',', name: ',' }, { id: '.', name: '.' }, { id: ';', name: ';' }, { id: '/', name: '/' }, { id: 'Up', name: '↑' }],
  [{ id: 'Control', name: 'Ctrl', w: 1.5 }, { id: 'Meta', name: 'Win', w: 1.25 }, { id: 'Alt', name: 'Alt', w: 1.25 },
    { id: 'Space', name: 'Espaço', w: 5 }, { id: 'Left', name: '←' }, { id: 'Down', name: '↓' }, { id: 'Right', name: '→' },
    ...['F13', 'F14', 'F15', 'F16'].map((f) => ({ id: f, name: f }))],
];
const ALL_TARGETS = TARGET_ROWS.flat();
const targetName = (id: string) => ALL_TARGETS.find((t) => t.id === id)?.name ?? id;

const MODS = ['Control', 'Shift', 'Alt', 'Meta'];
const MEDIA = [
  { id: 'playpause', name: 'Tocar / pausar' }, { id: 'stop', name: 'Parar' },
  { id: 'prev', name: 'Faixa anterior' }, { id: 'next', name: 'Próxima faixa' },
  { id: 'volup', name: 'Aumentar volume' }, { id: 'voldown', name: 'Baixar volume' }, { id: 'mute', name: 'Mudo' },
];
const MOUSE = [
  { id: 'left', name: 'Clique esquerdo' }, { id: 'right', name: 'Clique direito' }, { id: 'middle', name: 'Clique do meio' },
  { id: 'back', name: 'Voltar' }, { id: 'forward', name: 'Avançar' },
  { id: 'scrollup', name: 'Rolar para cima' }, { id: 'scrolldown', name: 'Rolar para baixo' },
];
const LAUNCH = [
  { id: 'https://', name: 'Navegador' }, { id: 'calc', name: 'Calculadora' }, { id: 'notepad', name: 'Bloco de notas' },
  { id: 'explorer', name: 'Explorador de arquivos' }, { id: 'ms-settings:', name: 'Configurações do Windows' },
];

/** Browser KeyboardEvent -> remap.rs key name. */
export function keyNameFromEvent(e: KeyboardEvent): string | null {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit\d$/.test(c)) return c.slice(5);
  if (/^F\d+$/.test(c)) return c;
  const named: Record<string, string> = {
    ControlLeft: 'Control', ControlRight: 'Control', ShiftLeft: 'Shift', ShiftRight: 'Shift', AltLeft: 'Alt', AltRight: 'Alt',
    MetaLeft: 'Meta', MetaRight: 'Meta', Escape: 'Escape', Enter: 'Enter', NumpadEnter: 'Enter', Tab: 'Tab', Space: 'Space',
    Backspace: 'Backspace', Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
    ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', CapsLock: 'CapsLock', PrintScreen: 'PrintScreen', Pause: 'Pause',
  };
  if (named[c]) return named[c];
  return e.key.length === 1 ? e.key.toLowerCase() : null;
}

export const keyLabel = (id: string) => KEYS.find((k) => k.id === id)?.label || (id === 'Space' ? 'Espaço' : id);

export function describe(a: Action): string {
  switch (a.kind) {
    case 'disabled': return 'Desativada';
    case 'key': return `Tecla ${targetName(a.key)}`;
    case 'shortcut': return a.keys.map(targetName).join(' + ');
    case 'text': return `“${a.text.length > 18 ? a.text.slice(0, 18) + '…' : a.text}”`;
    case 'launch': return LAUNCH.find((l) => l.id === a.target)?.name ?? a.target;
    case 'media': return MEDIA.find((m) => m.id === a.action)?.name ?? a.action;
    case 'mouse': return MOUSE.find((m) => m.id === a.button)?.name ?? a.button;
    case 'macro': return `Macro (${a.steps.filter((s) => s.down).length} ${a.steps.filter((s) => s.down).length === 1 ? 'passo' : 'passos'})`;
    case 'profile': return a.target === 'next' ? 'Próximo perfil' : `Perfil ${profileNames[a.target] ?? a.target}`;
  }
}

export const defaults: Record<Kind, Action> = {
  key: { kind: 'key', key: 'B' },
  shortcut: { kind: 'shortcut', keys: ['Control', 'C'] },
  mouse: { kind: 'mouse', button: 'left' },
  text: { kind: 'text', text: '' },
  media: { kind: 'media', action: 'playpause' },
  launch: { kind: 'launch', target: 'calc' },
  macro: { kind: 'macro', steps: [], options: DEFAULT_MACRO_OPTIONS },
  profile: { kind: 'profile', target: 'next' },
  disabled: { kind: 'disabled' },
};

// Filled from props so describe() can show profile names.
let profileNames: Record<string, string> = {};

type Item = { uid: number; keyId: string | null; action: Action | null };

type Props = {
  bindings: Bindings;
  onChange: (b: Bindings) => void;
  /** Key clicked on the keyboard drawing (selection mode). */
  pickedKey: string | null;
  onPickedKeyHandled: () => void;
  keymap: Record<string, number | null>;
  onSelectedKeyChange: (id: string | null) => void;
  profiles: { id: string; name: string }[];
};

let uidSeq = 1;

export function KeyActions({ bindings, onChange, pickedKey, onPickedKeyHandled, keymap, onSelectedKeyChange, profiles }: Props) {
  profileNames = Object.fromEntries(profiles.map((p) => [p.id, p.name]));
  const [items, setItems] = useState<Item[]>(() =>
    Object.entries(bindings).map(([keyId, action]) => ({ uid: uidSeq++, keyId, action })),
  );
  const [sel, setSel] = useState<number | null>(items[0]?.uid ?? null);
  const [recordingKey, setRecordingKey] = useState(false);
  const current = items.find((i) => i.uid === sel) ?? null;

  // Persist complete items whenever they change.
  const commit = (next: Item[]) => {
    setItems(next);
    const b: Bindings = {};
    for (const i of next) if (i.keyId && i.action && valid(i.action)) b[i.keyId] = i.action;
    onChange(b);
  };
  const patch = (p: Partial<Item>) => current && commit(items.map((i) => (i.uid === current.uid ? { ...i, ...p } : i)));

  const setKey = (keyId: string) => {
    if (!current) return;
    // One action per physical key: drop the key from any other assignment.
    commit(items.map((i) => (i.uid === current.uid ? { ...i, keyId } : i.keyId === keyId ? { ...i, keyId: null } : i)));
  };

  useEffect(() => onSelectedKeyChange(current?.keyId ?? null), [current?.keyId]);

  // Key clicked on the drawing.
  useEffect(() => {
    if (pickedKey) {
      if (current) setKey(pickedKey);
      onPickedKeyHandled();
    }
  }, [pickedKey]);

  // "Record" the key by pressing it on the K70 (device event, works for swallowed keys too).
  useEffect(() => {
    if (!recordingKey) return;
    const slotToId = Object.fromEntries(Object.entries(keymap).filter(([, s]) => s != null).map(([id, s]) => [s, id]));
    const un = listen<number>('key-press', (e) => {
      const id = slotToId[e.payload];
      if (id) {
        setKey(id);
        setRecordingKey(false);
      }
    });
    return () => void un.then((f) => f());
  }, [recordingKey, current?.uid, items]);

  const add = () => {
    const it: Item = { uid: uidSeq++, keyId: null, action: null };
    setItems([...items, it]);
    setSel(it.uid);
  };

  const remove = (uid: number) => {
    const next = items.filter((i) => i.uid !== uid);
    commit(next);
    if (sel === uid) setSel(next[0]?.uid ?? null);
  };

  return (
    <section className="assign">
      {/* 1. Assignments */}
      <div className="panel col">
        <h3>Atribuições</h3>
        <button className="add" onClick={add} aria-label="Nova atribuição">
          <Icon name="plus" />
        </button>
        <ul className="assign-list">
          {items.map((i) => (
            <li key={i.uid} className={i.uid === sel ? 'on' : ''}>
              <button className="assign-item" onClick={() => setSel(i.uid)}>
                <Icon name={i.action ? KIND_INFO[i.action.kind].icon : 'plus'} size={16} />
                <span>
                  {i.keyId ? <kbd>{keyLabel(i.keyId)}</kbd> : <em>sem tecla</em>}
                  <small>{i.action ? describe(i.action) : 'Nova atribuição'}</small>
                </span>
              </button>
              <button className="icon-btn" aria-label="Remover" onClick={() => remove(i.uid)}>
                <Icon name="close" size={14} />
              </button>
            </li>
          ))}
        </ul>
        {items.length === 0 && <p className="muted small">Clique em + para criar a primeira.</p>}
      </div>

      <div className="stack">
      {/* 2. Type */}
      <div className={'panel col' + (current ? '' : ' dim')}>
        <h3>Tipo de atribuição</h3>
        <div className="types">
          {GROUPS.map((g) => (
            <div key={g.title}>
              <div className="group-title">{g.title}</div>
              <div className="tiles">
                {g.items.map((t) => (
                  <button
                    key={t.id}
                    disabled={!current}
                    className={'tile' + (current?.action?.kind === t.id ? ' on' : '')}
                    onClick={() => patch({ action: current?.action?.kind === t.id ? current.action : defaults[t.id] })}
                  >
                    <Icon name={t.icon} size={22} />
                    <span>{t.name}</span>
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* 3. Key */}
      <div className={'panel col' + (current ? '' : ' dim')}>
        <h3>Tecla</h3>
        <button
          className={'key-slot' + (recordingKey ? ' rec' : '')}
          disabled={!current}
          onClick={() => setRecordingKey(!recordingKey)}
        >
          {current?.keyId && !recordingKey ? <kbd className="big">{keyLabel(current.keyId)}</kbd> : <Icon name={recordingKey ? 'record' : 'plus'} size={26} />}
        </button>
        <p className="muted small center">
          {recordingKey
            ? 'Aperte a tecla no K70…'
            : 'Clique numa tecla do desenho acima, ou clique aqui e aperte a tecla no teclado.'}
        </p>
      </div>

      </div>

      {/* 4. Details */}
      <div className={'panel col detail' + (current?.action ? '' : ' dim')}>
        {current?.action ? (
          <Detail action={current.action} onChange={(action) => patch({ action })} profiles={profiles} />
        ) : (
          <>
            <h3>Detalhes</h3>
            <p className="muted">Escolha um tipo de atribuição.</p>
          </>
        )}
      </div>
    </section>
  );
}

export function valid(a: Action): boolean {
  if (a.kind === 'text') return a.text.length > 0;
  if (a.kind === 'launch') return a.target.trim().length > 0 && a.target !== 'https://';
  if (a.kind === 'macro') return a.steps.length > 0;
  if (a.kind === 'shortcut') return a.keys.length > 0;
  return true;
}

export function Detail({ action, onChange, profiles }: { action: Action; onChange: (a: Action) => void; profiles: { id: string; name: string }[] }) {
  switch (action.kind) {
    case 'profile':
      return (
        <>
          <h3>Trocar perfil</h3>
          <Radio
            options={[{ id: 'next', name: 'Próximo perfil (em ciclo)' }, ...profiles.map((p) => ({ id: p.id, name: p.name }))]}
            value={action.target}
            onChange={(target) => onChange({ kind: 'profile', target })}
          />
          <p className="muted small">Dica: coloque a mesma tecla em todos os perfis para poder voltar.</p>
        </>
      );
    case 'key':
      return (
        <>
          <h3>Remapear: tecla</h3>
          <div className="mini-kb">
            {TARGET_ROWS.map((row, r) => (
              <div key={r} className="mini-row">
                {row.map((k) => (
                  <button
                    key={k.id}
                    style={{ flexGrow: k.w ?? 1 }}
                    className={'mini-key' + (action.key === k.id ? ' on' : '')}
                    onClick={() => onChange({ kind: 'key', key: k.id })}
                  >
                    {k.name}
                  </button>
                ))}
              </div>
            ))}
          </div>
        </>
      );
    case 'shortcut':
      return <ShortcutDetail keys={action.keys} onChange={(keys) => onChange({ kind: 'shortcut', keys })} />;
    case 'mouse':
      return (
        <>
          <h3>Remapear: mouse</h3>
          <Radio options={MOUSE} value={action.button} onChange={(button) => onChange({ kind: 'mouse', button })} />
        </>
      );
    case 'text':
      return (
        <>
          <h3>Texto</h3>
          <textarea
            className="big-input"
            rows={5}
            value={action.text}
            placeholder="Digite o texto que a tecla vai escrever. Pode colar da área de transferência."
            onChange={(e) => onChange({ kind: 'text', text: e.target.value })}
          />
        </>
      );
    case 'media':
      return (
        <>
          <h3>Mídia</h3>
          <Radio options={MEDIA} value={action.action} onChange={(v) => onChange({ kind: 'media', action: v })} />
        </>
      );
    case 'launch': {
      const preset = LAUNCH.some((l) => l.id === action.target && l.id !== 'https://');
      return (
        <>
          <h3>Abrir aplicativo</h3>
          <Radio options={LAUNCH.filter((l) => l.id !== 'https://')} value={action.target} onChange={(v) => onChange({ kind: 'launch', target: v })} />
          <label className="field">
            Ou um programa, arquivo ou site
            <input
              value={preset ? '' : action.target}
              placeholder="C:\Jogos\jogo.exe   ·   https://…"
              onChange={(e) => onChange({ kind: 'launch', target: e.target.value })}
            />
          </label>
        </>
      );
    }
    case 'macro':
      return (
        <MacroEditor
          steps={action.steps}
          options={action.options ?? DEFAULT_MACRO_OPTIONS}
          onChange={(steps, options) => onChange({ kind: 'macro', steps, options })}
          keyName={targetName}
          keyFromEvent={keyNameFromEvent}
        />
      );
    case 'disabled':
      return (
        <>
          <h3>Desativar</h3>
          <p className="muted">A tecla não faz nada enquanto este perfil estiver ativo. Útil para a tecla Windows em jogos.</p>
        </>
      );
  }
}

function Radio({ options, value, onChange }: { options: { id: string; name: string }[]; value: string; onChange: (v: string) => void }) {
  return (
    <div className="radio-grid" role="radiogroup">
      {options.map((o) => (
        <label key={o.id} className="radio">
          <input type="radio" checked={value === o.id} onChange={() => onChange(o.id)} />
          {o.name}
        </label>
      ))}
    </div>
  );
}

function ShortcutDetail({ keys, onChange }: { keys: string[]; onChange: (k: string[]) => void }) {
  const [capturing, setCapturing] = useState(false);
  useEffect(() => {
    if (!capturing) return;
    const down = (e: KeyboardEvent) => {
      e.preventDefault();
      const name = keyNameFromEvent(e);
      if (!name || MODS.includes(name)) return; // wait for the main key
      const mods = MODS.filter((m) => ({ Control: e.ctrlKey, Shift: e.shiftKey, Alt: e.altKey, Meta: e.metaKey })[m]);
      onChange([...mods, name]);
      setCapturing(false);
    };
    window.addEventListener('keydown', down, true);
    return () => window.removeEventListener('keydown', down, true);
  }, [capturing]);
  return (
    <>
      <h3>Remapear: atalho</h3>
      <p className="muted small">Clique na caixa e pressione a combinação (pode incluir Ctrl, Shift, Alt, Win).</p>
      <button className={'capture' + (capturing ? ' rec' : '')} onClick={() => setCapturing(true)}>
        {capturing ? 'Pressione a combinação…' : keys.length ? keys.map((k) => <kbd key={k}>{targetName(k)}</kbd>) : 'Clique para gravar'}
      </button>
    </>
  );
}

