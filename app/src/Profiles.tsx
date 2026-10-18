import { useEffect, useRef, useState } from 'react';
import { api, type Profiles } from './api';

/** Compact switcher for the sidebar. */
export function ProfileSwitcher({ data, onManage }: { data: Profiles; onManage: () => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const active = data.profiles.find((p) => p.id === data.active);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    window.addEventListener('mousedown', close);
    return () => window.removeEventListener('mousedown', close);
  }, [open]);

  return (
    <div className="profile-switch" ref={ref}>
      <button className="profile-current" onClick={() => setOpen(!open)} aria-expanded={open}>
        <span>
          <small>Perfil</small>
          <strong>{active?.name ?? '—'}</strong>
        </span>
        <span className="chev" aria-hidden>▾</span>
      </button>
      {data.auto_app && <div className="auto-badge">auto · {data.auto_app}</div>}
      {open && (
        <div className="profile-menu" role="menu">
          {data.profiles.map((p) => (
            <button
              key={p.id}
              role="menuitemradio"
              aria-checked={p.id === data.active}
              className={p.id === data.active ? 'on' : ''}
              onClick={() => {
                api.selectProfile(p.id);
                setOpen(false);
              }}
            >
              {p.name}
              {p.apps.length > 0 && <small>{p.apps.length} app{p.apps.length > 1 ? 's' : ''}</small>}
            </button>
          ))}
          <button className="manage" onClick={() => { setOpen(false); onManage(); }}>
            Gerenciar perfis…
          </button>
        </div>
      )}
    </div>
  );
}

/** Full page: create, rename, delete, and link programs for automatic switching. */
export function ProfilesPage({ data }: { data: Profiles }) {
  const [name, setName] = useState('');
  const [editing, setEditing] = useState<string | null>(data.active);
  const [error, setError] = useState<string | null>(null);
  const p = data.profiles.find((x) => x.id === editing) ?? null;
  const [draftName, setDraftName] = useState(p?.name ?? '');
  const [draftApp, setDraftApp] = useState('');
  const [detecting, setDetecting] = useState(0);

  useEffect(() => setDraftName(p?.name ?? ''), [editing, p?.name]);

  const run = (f: Promise<unknown>) => f.then(() => setError(null), (e) => setError(String(e)));

  const create = async () => {
    if (!name.trim()) return;
    try {
      const id = await api.createProfile(name.trim());
      setName('');
      setEditing(id);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const addApp = (exe: string) => {
    if (!p || !exe.trim()) return;
    const apps = Array.from(new Set([...p.apps, exe.trim()]));
    run(api.updateProfile(p.id, p.name, apps));
    setDraftApp('');
  };

  // Countdown, then read the window in front (user switches to the game meanwhile).
  const detect = () => {
    setDetecting(5);
    let n = 5;
    const t = window.setInterval(async () => {
      n -= 1;
      setDetecting(n);
      if (n === 0) {
        window.clearInterval(t);
        const exe = await api.foregroundApp();
        if (exe && !exe.toLowerCase().startsWith('openperipheral')) addApp(exe);
        else setError('Não detectei outro programa na frente. Tente de novo trocando de janela durante a contagem.');
      }
    }, 1000);
  };

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Perfis</h1>
          <p className="muted">Cada perfil guarda iluminação, teclas e equalizador. Ligue programas a um perfil para ele ativar sozinho.</p>
        </div>
      </header>

      <section className="profiles">
        <div className="panel col">
          <h3>Seus perfis</h3>
          <ul className="assign-list">
            {data.profiles.map((x) => (
              <li key={x.id} className={x.id === editing ? 'on' : ''}>
                <button className="assign-item" onClick={() => setEditing(x.id)}>
                  <span className={'dot' + (x.id === data.active ? ' online' : '')} aria-hidden />
                  <span>
                    {x.name}
                    <small>
                      {x.id === data.active ? 'em uso' : x.id === data.selected ? 'selecionado' : ''}
                      {x.apps.length ? ` · ${x.apps.join(', ')}` : ''}
                    </small>
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="new-profile">
            <input
              value={name}
              placeholder="Nome do novo perfil"
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && create()}
            />
            <button className="primary" onClick={create} disabled={!name.trim()}>Criar</button>
          </div>
          <p className="muted small">O novo perfil começa como cópia do perfil em uso.</p>
        </div>

        {p && (
          <div className="panel col">
            <h3>Editar perfil</h3>
            <label className="field">
              Nome
              <input
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                onBlur={() => draftName.trim() && draftName !== p.name && run(api.updateProfile(p.id, draftName, p.apps))}
              />
            </label>

            <div className="field">
              Ativar automaticamente com
              <ul className="app-list">
                {p.apps.map((a) => (
                  <li key={a}>
                    <code>{a}</code>
                    <button className="icon-btn" aria-label={`Remover ${a}`} onClick={() => run(api.updateProfile(p.id, p.name, p.apps.filter((x) => x !== a)))}>✕</button>
                  </li>
                ))}
                {p.apps.length === 0 && <li className="muted">Nenhum programa. O perfil só muda quando você escolhe.</li>}
              </ul>
              <div className="new-profile">
                <input value={draftApp} placeholder="ex.: cs2.exe" onChange={(e) => setDraftApp(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && addApp(draftApp)} />
                <button className="ghost" onClick={() => addApp(draftApp)} disabled={!draftApp.trim()}>Adicionar</button>
                <button className="ghost" onClick={detect} disabled={detecting > 0}>
                  {detecting > 0 ? `Troque para o jogo… ${detecting}` : 'Detectar programa'}
                </button>
              </div>
            </div>

            <div className="row">
              {p.id !== data.active && <button className="primary" onClick={() => run(api.selectProfile(p.id))}>Usar este perfil</button>}
              <button className="ghost danger" disabled={data.profiles.length <= 1} onClick={() => run(api.deleteProfile(p.id))}>Apagar perfil</button>
            </div>
          </div>
        )}
      </section>
      {error && <p className="err">{error}</p>}
    </>
  );
}
