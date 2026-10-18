import { useEffect, useState } from 'react';
import { api, type Action, type Bindings } from './api';
import { Icon } from './icons';
import { Detail, GROUPS, defaults, describe, valid } from './KeyActions';

/** Same names as op_core::m65::BUTTONS. */
const BUTTONS: { id: string; name: string; note?: string }[] = [
  { id: 'left', name: 'Clique esquerdo', note: 'Cuidado: remapeado, o clique esquerdo deixa de clicar.' },
  { id: 'right', name: 'Clique direito' },
  { id: 'middle', name: 'Clique do meio' },
  { id: 'back', name: 'Voltar (lateral)' },
  { id: 'forward', name: 'Avançar (lateral)' },
  { id: 'dpi_up', name: 'DPI +', note: 'Remapeado, deixa de trocar o estágio de DPI.' },
  { id: 'dpi_down', name: 'DPI −', note: 'Remapeado, deixa de trocar o estágio de DPI.' },
  { id: 'sniper', name: 'Sniper', note: 'Remapeado, deixa de reduzir o DPI.' },
];

/** M65 button assignments for the profile in use (iCUE "Atribuições de Tecla"). */
export function MouseButtons({ profiles }: { profiles: { id: string; name: string }[] }) {
  const [bindings, setBindings] = useState<Bindings | null>(null);
  const [sel, setSel] = useState('back');
  // Edits in progress, kept even while not yet valid (e.g. an empty macro).
  const [draft, setDraft] = useState<Record<string, Action>>({});

  useEffect(() => {
    api.mouseBindings().then(setBindings);
  }, []);
  if (!bindings) return null;

  const action = draft[sel] ?? bindings[sel] ?? null;
  const save = (id: string, a: Action | null) => {
    const next = { ...bindings };
    if (a && valid(a)) next[id] = a;
    else delete next[id];
    setBindings(next);
    api.setMouseBindings(next);
  };
  const change = (a: Action | null) => {
    const d = { ...draft };
    if (a) d[sel] = a;
    else delete d[sel];
    setDraft(d);
    save(sel, a);
  };
  const info = BUTTONS.find((b) => b.id === sel);

  return (
    <section className="assign mouse-assign">
      <div className="panel col">
        <h3>Botões</h3>
        <ul className="assign-list">
          {BUTTONS.map((b) => (
            <li key={b.id} className={b.id === sel ? 'on' : ''}>
              <button className="assign-item" onClick={() => setSel(b.id)}>
                <Icon name={bindings[b.id] ? 'macro' : 'mouse'} size={16} />
                <span>
                  {b.name}
                  <small>{bindings[b.id] ? describe(bindings[b.id]) : 'Padrão'}</small>
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="panel col">
        <h3>Tipo de atribuição</h3>
        <div className="types">
          <div>
            <div className="group-title">Padrão</div>
            <div className="tiles">
              <button className={'tile' + (action ? '' : ' on')} onClick={() => change(null)}>
                <Icon name="mouse" size={22} />
                <span>Função original</span>
              </button>
            </div>
          </div>
          {GROUPS.map((g) => (
            <div key={g.title}>
              <div className="group-title">{g.title}</div>
              <div className="tiles">
                {g.items.map((t) => (
                  <button
                    key={t.id}
                    className={'tile' + (action?.kind === t.id ? ' on' : '')}
                    onClick={() => change(action?.kind === t.id ? action : defaults[t.id])}
                  >
                    <Icon name={t.icon} size={22} />
                    <span>{t.name}</span>
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
        {info?.note && <p className="muted small">{info.note}</p>}
      </div>

      <div className={'panel col detail' + (action ? '' : ' dim')}>
        {action ? (
          <Detail action={action} onChange={change} profiles={profiles} />
        ) : (
          <>
            <h3>Detalhes</h3>
            <p className="muted">O botão faz o que faz de fábrica. Escolha um tipo para mudar.</p>
          </>
        )}
      </div>
    </section>
  );
}
