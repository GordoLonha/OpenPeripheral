import { useEffect, useRef, useState } from 'react';
import { api, isWindows, type ProfileInfo, type Profiles } from './api';
import { Icon } from './icons';

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
  const [editing, setEditing] = useState<string>(data.active);
  const [error, setError] = useState<string | null>(null);
  // The profile being edited; falls back to the one in use if it was deleted.
  const p = data.profiles.find((x) => x.id === editing) ?? data.profiles.find((x) => x.id === data.active) ?? data.profiles[0];
  const [draftName, setDraftName] = useState(p?.name ?? '');
  const [draftApp, setDraftApp] = useState('');
  const [detecting, setDetecting] = useState(0);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => setDraftName(p?.name ?? ''), [p?.id, p?.name]);
  useEffect(() => setConfirmDelete(false), [p?.id]);
  useEffect(() => () => window.clearInterval(timer.current), []);

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

  const rename = () => {
    if (!p) return;
    const n = draftName.trim();
    if (!n) setDraftName(p.name);
    else if (n !== p.name) run(api.updateProfile(p.id, n, p.apps));
  };

  const addApp = (exe: string) => {
    if (!p || !exe.trim()) return;
    if (p.apps.some((a) => a.toLowerCase() === exe.trim().toLowerCase())) {
      setDraftApp('');
      return;
    }
    run(api.updateProfile(p.id, p.name, [...p.apps, exe.trim()]));
    setDraftApp('');
  };

  // Countdown, then read the window in front (user switches to the game meanwhile).
  const detect = () => {
    setDetecting(5);
    let n = 5;
    window.clearInterval(timer.current);
    timer.current = window.setInterval(async () => {
      n -= 1;
      setDetecting(n);
      if (n === 0) {
        window.clearInterval(timer.current);
        const exe = await api.foregroundApp();
        if (exe && !exe.toLowerCase().startsWith('openperipheral')) addApp(exe);
        else
          setError(
            isWindows
              ? 'Não detectei outro programa na frente. Tente de novo trocando de janela durante a contagem.'
              : 'Não detectei outro programa na frente. No Wayland só aparecem programas X11/XWayland (como jogos pelo Proton); digite o nome se for outro.',
          );
      }
    }, 1000);
  };

  const status = (x: ProfileInfo) =>
    [x.id === data.active ? (data.auto_app && x.id !== data.selected ? `em uso · ${data.auto_app}` : 'em uso') : x.id === data.selected ? 'selecionado' : '', x.apps.join(', ')]
      .filter(Boolean)
      .join(' · ') || 'só manual';

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Perfis</h1>
          <p className="page-sub">Cada perfil guarda iluminação, teclas, botões do mouse e equalizador.</p>
        </div>
      </header>

      <section className="profiles">
        <div className="card">
          <div className="card-head">
            <h2 className="card-title">Seus perfis</h2>
          </div>
          <ul className="assign-list">
            {data.profiles.map((x) => (
              <li key={x.id} className={x.id === p?.id ? 'on' : ''}>
                <button className="assign-item" onClick={() => setEditing(x.id)} aria-current={x.id === p?.id}>
                  <span className={'dot' + (x.id === data.active ? ' online' : '')} aria-hidden />
                  <span>
                    {x.name}
                    <small>{status(x)}</small>
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="inline-form">
            <input
              className="input"
              value={name}
              aria-label="Nome do novo perfil"
              placeholder="Nome do novo perfil"
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && create()}
            />
            <button className="primary" onClick={create} disabled={!name.trim()}>Criar</button>
          </div>
          <p className="hint">O novo perfil começa como cópia do perfil em uso.</p>
        </div>

        {p && (
          <div className="stack">
            <div className="list">
              <div className="list-row">
                <div>
                  <strong>Nome</strong>
                  <p>{p.id === data.active ? 'Este é o perfil em uso.' : 'Mude para este perfil quando quiser.'}</p>
                </div>
                <div className="row">
                  <input
                    className="input name-input"
                    value={draftName}
                    aria-label="Nome do perfil"
                    onChange={(e) => setDraftName(e.target.value)}
                    onBlur={rename}
                    onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
                  />
                  {p.id !== data.active && <button className="primary" onClick={() => run(api.selectProfile(p.id))}>Usar este perfil</button>}
                </div>
              </div>
            </div>

            <div className="card">
              <div className="card-head">
                <div>
                  <h2 className="card-title">Ativar sozinho com</h2>
                  <p className="muted small">Enquanto um destes programas estiver na frente, este perfil fica em uso; ao sair dele, volta o perfil selecionado.</p>
                </div>
                <button className="ghost" onClick={detect} disabled={detecting > 0}>
                  {detecting > 0 ? `Troque para o programa… ${detecting}` : 'Detectar programa'}
                </button>
              </div>
              {p.apps.length > 0 ? (
                <ul className="app-list">
                  {p.apps.map((a) => (
                    <li key={a}>
                      <code>{a}</code>
                      {data.auto_app?.toLowerCase() === a.toLowerCase() && p.id === data.active && <span className="tag">na frente agora</span>}
                      <button className="icon-btn" aria-label={`Remover ${a}`} onClick={() => run(api.updateProfile(p.id, p.name, p.apps.filter((x) => x !== a)))}>
                        <Icon name="close" size={16} />
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="empty">Nenhum programa ligado. O perfil só muda quando você escolhe.</p>
              )}
              <div className="inline-form">
                <input
                  className="input"
                  value={draftApp}
                  aria-label="Nome do programa"
                  placeholder={isWindows ? 'ex.: cs2.exe' : 'ex.: cs2, ou cs2.exe para jogos pelo Proton'}
                  onChange={(e) => setDraftApp(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && addApp(draftApp)}
                />
                <button className="ghost" onClick={() => addApp(draftApp)} disabled={!draftApp.trim()}>Adicionar</button>
              </div>
            </div>

            <div className="list">
              <div className="list-row">
                <div>
                  <strong>Apagar perfil</strong>
                  <p>{data.profiles.length <= 1 ? 'O último perfil não pode ser apagado.' : 'Apaga a iluminação, as teclas e o equalizador deste perfil. Não dá para desfazer.'}</p>
                </div>
                {confirmDelete ? (
                  <div className="row">
                    <button className="ghost" onClick={() => setConfirmDelete(false)}>Cancelar</button>
                    <button className="danger-btn" onClick={() => { setConfirmDelete(false); run(api.deleteProfile(p.id)); }}>Apagar “{p.name}”</button>
                  </div>
                ) : (
                  <button className="ghost danger" disabled={data.profiles.length <= 1} onClick={() => setConfirmDelete(true)}>Apagar…</button>
                )}
              </div>
            </div>
          </div>
        )}
      </section>
      {error && <p className="err" role="alert">{error}</p>}
    </>
  );
}
