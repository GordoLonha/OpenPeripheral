// Window chrome: the black top bar and the profile rail on the left.
import { useState } from 'react';
import { api, type Profiles } from './api';
import { Icon } from './icons';

export type TopTab = 'home' | 'dashboard' | 'scenes';

const TABS: [TopTab, string][] = [
  ['home', 'Início'],
  ['dashboard', 'Painel'],
  ['scenes', 'Cenas'],
];

export function TopBar(p: {
  tab: TopTab | null;
  onTab: (t: TopTab) => void;
  brightness: number;
  onBrightness: (v: number) => void;
  inSettings: boolean;
  onSettings: () => void;
  onAbout: () => void;
}) {
  return (
    <header className="topbar">
      <span className="brand-mark" title="OpenPeripheral" aria-hidden />
      <nav className="top-tabs" aria-label="Principal">
        {TABS.map(([id, label]) => (
          <button key={id} className={p.tab === id ? 'on' : ''} aria-current={p.tab === id ? 'page' : undefined} onClick={() => p.onTab(id)}>
            {label}
          </button>
        ))}
      </nav>
      <div className="top-right">
        <label className="top-bright" title="Brilho geral">
          <Icon name="sun" size={15} />
          <input
            type="range"
            min={0}
            max={100}
            step={1}
            value={p.brightness}
            aria-label="Brilho geral"
            onChange={(e) => p.onBrightness(Number(e.target.value))}
          />
        </label>
        <button className="top-icon" title="Sobre o OpenPeripheral" aria-label="Sobre o OpenPeripheral" onClick={p.onAbout}>
          ?
        </button>
        <button className={'top-icon' + (p.inSettings ? ' on' : '')} title="Configurações" aria-label="Configurações" onClick={p.onSettings}>
          <Icon name="settings" size={17} />
        </button>
      </div>
    </header>
  );
}

const Hexagon = () => (
  <svg className="hex" viewBox="0 0 24 24" aria-hidden>
    <path d="M12 3.5l7.4 4.25v8.5L12 20.5l-7.4-4.25v-8.5z" />
  </svg>
);

/** Narrow column of profiles; the lit tile is the profile in use. ">" shows the names. */
export function ProfileRail({ data }: { data: Profiles | null }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);

  const add = async () => {
    if (!data || busy) return;
    setBusy(true);
    try {
      const names = new Set(data.profiles.map((p) => p.name));
      let n = data.profiles.length + 1;
      while (names.has(`Perfil ${n}`)) n++;
      const id = await api.createProfile(`Perfil ${n}`);
      await api.selectProfile(id);
    } finally {
      setBusy(false);
    }
  };

  return (
    <aside className={'rail' + (open ? ' open' : '')} aria-label="Perfis">
      <button className="rail-toggle" title={open ? 'Esconder nomes' : 'Mostrar nomes dos perfis'} aria-expanded={open} onClick={() => setOpen(!open)}>
        {open ? '<' : '>'}
      </button>
      <div className="rail-list">
        {data?.profiles.map((p) => {
          const on = p.id === data.active;
          const auto = on && data.auto_app ? ` · ativado por ${data.auto_app}` : '';
          return (
            <button
              key={p.id}
              className={'rail-item' + (on ? ' on' : '')}
              title={p.name + auto}
              aria-pressed={on}
              onClick={() => api.selectProfile(p.id)}
            >
              <span className="rail-tile">
                <Hexagon />
              </span>
              <span className="rail-name">
                {p.name}
                {auto && <small>auto · {data.auto_app}</small>}
              </span>
            </button>
          );
        })}
        <button className="rail-item rail-add" title="Novo perfil" disabled={busy} onClick={add}>
          <span className="rail-tile">+</span>
          <span className="rail-name">Novo perfil</span>
        </button>
      </div>
      {open && (
        <footer className="rail-foot">
          <span>Fechar a janela mantém as luzes pela bandeja.</span>
          <span>Versão {__APP_VERSION__}</span>
        </footer>
      )}
    </aside>
  );
}
