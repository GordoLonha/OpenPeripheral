import { useEffect, useRef, useState } from 'react';
import { api, toHex, type AppSettings, type Bindings, type Lighting, type Profiles, type RGB, type Status } from './api';
import { Keyboard, geometryFor } from './Keyboard';
import { Equalizer } from './Equalizer';
import { KeyActions } from './KeyActions';
import { HeadsetOptions, KeyboardOptions } from './DeviceOptions';
import { ColorField, Slider, Switch, Tabs } from './controls';
import { Icon, type IconName } from './icons';
import { MousePage } from './MousePage';
import { RELEASES_URL, installKind, openUrl, useUpdater, type InstallKind } from './updates';
import { ProfileSwitcher, ProfilesPage } from './Profiles';
import { Home } from './Home';
import { EFFECTS, batteryColor } from './display';
import './App.css';


const SWATCHES: RGB[] = [
  [255, 255, 255], [255, 0, 0], [255, 110, 0], [255, 220, 0], [0, 255, 90],
  [0, 200, 255], [0, 70, 255], [140, 0, 255], [255, 0, 170], [0, 0, 0],
];

type Page = 'home' | 'keyboard' | 'headset' | 'mouse' | 'profiles' | 'settings';
type Tool = 'none' | 'paint' | 'erase';

export default function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [lighting, setLighting] = useState<Lighting | null>(null);
  const [keymap, setKeymap] = useState<Record<string, number | null>>({});
  const [ledmap, setLedmap] = useState<Record<string, number | null>>({});
  const [page, setPage] = useState<Page>('home');
  const [tool, setTool] = useState<Tool>('none');
  const [brush, setBrush] = useState<RGB>([255, 0, 0]);
  const [kbTab, setKbTab] = useState<'light' | 'keys' | 'settings'>('light');
  const [bindings, setBindings] = useState<Bindings>({});
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [pickedKey, setPickedKey] = useState<string | null>(null);
  // Bumped whenever per-profile data is (re)loaded, to re-mount editors with it.
  const [loadGen, setLoadGen] = useState(0);
  const [profiles, setProfiles] = useState<Profiles | null>(null);
  const activeRef = useRef<string | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);
  const updater = useUpdater();

  const loadProfileData = () =>
    Promise.all([api.lighting(), api.bindings()]).then(([l, b]) => {
      setLighting(l);
      setBindings(b);
      setLoadGen((g) => g + 1);
    });

  useEffect(() => {
    api.status().then(setStatus);
    loadProfileData();
    api.profiles().then((p) => {
      activeRef.current = p.active;
      setProfiles(p);
    });
    api.keymap().then(setKeymap);
    api.ledmap().then((m) => {
      setLedmap(m);
      api.setGeometry(geometryFor(m));
    });
    const unStatus = api.onStatus(setStatus);
    const unNav = api.onNavigate((p) => setPage(p as Page));
    const unLighting = api.onLighting(setLighting);
    const unProfile = api.onProfile((p) => {
      setProfiles(p);
      if (p.active !== activeRef.current) {
        activeRef.current = p.active;
        loadProfileData();
      }
    });
    return () => {
      unStatus.then((f) => f());
      unNav.then((f) => f());
      unLighting.then((f) => f());
      unProfile.then((f) => f());
    };
  }, []);

  // Like iCUE: a device's page closes when the device goes away. A short grace
  // period keeps it open while a device re-connects (e.g. after a polling change).
  const pageDevice = page === 'keyboard' ? status?.keyboard : page === 'headset' ? status?.headset : page === 'mouse' ? status?.mouse : true;
  useEffect(() => {
    if (pageDevice || !status) return;
    const t = window.setTimeout(() => setPage('home'), 3000);
    return () => window.clearTimeout(t);
  }, [pageDevice, status]);

  const update = (patch: Partial<Lighting>) => {
    setLighting((cur) => {
      if (!cur) return cur;
      const next = { ...cur, ...patch };
      window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => api.setLighting(next), 60);
      return next;
    });
  };

  const paintSlot = (slot: number) => {
    if (!lighting) return;
    const painted = { ...lighting.painted };
    if (tool === 'erase') delete painted[slot];
    else painted[slot] = brush;
    update({ painted });
  };

  const changeBindings = (b: Bindings) => {
    setBindings(b);
    api.setBindings(b);
  };

  if (!lighting) return <div className="boot">Carregando…</div>;
  const fx = EFFECTS.find((e) => e.id === lighting.effect)!;
  const paintedCount = Object.keys(lighting.painted).length;

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden />
          OpenPeripheral
        </div>
        {profiles && <ProfileSwitcher data={profiles} onManage={() => setPage('profiles')} />}
        <nav className="nav-group" aria-label="Principal">
          <NavItem icon="home" label="Início" active={page === 'home'} onClick={() => setPage('home')} />
        </nav>
        <nav className="nav-group" aria-label="Dispositivos">
          <div className="nav-label">Dispositivos</div>
          {!status?.keyboard && !status?.headset && !status?.mouse && <p className="nav-empty">Nenhum conectado</p>}
          {(status?.keyboard || page === 'keyboard') && (
          <NavItem
            icon="keyboard"
            label="Teclado"
            sub={status?.keyboard ? `K70 MK.2 LP · fw ${status.keyboard.firmware}` : 'desconectado'}
            online={!!status?.keyboard}
            active={page === 'keyboard'}
            onClick={() => setPage('keyboard')}
          />
          )}
          {(status?.headset || page === 'headset') && (
          <NavItem
            icon="headset"
            label="Headset"
            sub={
              status?.headset
                ? status.headset.battery != null
                  ? `HS80 · ${status.headset.battery.toFixed(0)}%${status.headset.charging ? ' · carregando' : ''}`
                  : 'HS80 · desligado'
                : 'desconectado'
            }
            online={!!status?.headset}
            active={page === 'headset'}
            onClick={() => setPage('headset')}
          />
          )}
          {(status?.mouse || page === 'mouse') && (
            <NavItem
              icon="mouse"
              label="Mouse"
              sub={status?.mouse ? `M65 · DPI ${status.mouse.dpi_stage + 1}` : 'desconectado'}
              online={!!status?.mouse}
              active={page === 'mouse'}
              onClick={() => setPage('mouse')}
            />
          )}
        </nav>
        <nav className="nav-group" aria-label="Geral">
          <NavItem icon="layers" label="Perfis" active={page === 'profiles'} onClick={() => setPage('profiles')} />
          <NavItem icon="settings" label="Configurações" active={page === 'settings'} onClick={() => setPage('settings')} />
        </nav>
        <footer className="sidebar-foot">
          <span>Fechar a janela mantém as luzes pela bandeja.</span>
          <span>Versão {__APP_VERSION__}</span>
        </footer>
      </aside>

      <main className="main">
        <div className="page">
        {updater.state.kind === 'available' && page !== 'settings' && (
          <div className="banner info" role="status">
            <span>Nova versão {updater.state.update.version} disponível.</span>
            <button className="ghost" onClick={() => setPage('settings')}>
              Ver atualização
            </button>
          </div>
        )}
        {status?.icue_running && (
          <div className="banner" role="status">
            O iCUE está aberto e controla o teclado. Feche-o (bandeja → Sair) para o OpenPeripheral assumir.
          </div>
        )}

        {page === 'home' ? (
          <Home
            status={status}
            profiles={profiles}
            lighting={lighting}
            ledmap={ledmap}
            onOpen={setPage}
            onLighting={update}
            onScene={(patch) => update({ ...patch, painted: {} })}
          />
        ) : page === 'keyboard' ? (
          <>
            <header className="page-head">
              <div>
                <h1>Teclado</h1>
                <p className="page-sub">
                  {status?.keyboard ? `K70 RGB MK.2 Low Profile · firmware ${status.keyboard.firmware}` : 'K70 desconectado'}
                </p>
              </div>
              <Tabs
                label="Seção do teclado"
                value={kbTab}
                onChange={setKbTab}
                options={[['light', 'Iluminação'], ['keys', 'Teclas'], ['settings', 'Ajustes']]}
              />
            </header>

            {kbTab === 'keys' ? (
              <>
                <section className="stage">
                  <div className="stage-bar">
                    <p className="muted">Clique numa tecla do desenho para escolher o que ela faz. Funciona em qualquer programa.</p>
                  </div>
                  <Keyboard
                    lighting={lighting}
                    ledmap={ledmap}
                    painting={false}
                    onSelectKey={setPickedKey}
                    selectedKey={selectedKey}
                    marked={new Set(Object.keys(bindings))}
                  />
                </section>
                <KeyActions
                  // Re-mount once saved bindings arrive so the list starts from them.
                  key={loadGen}
                  bindings={bindings}
                  onChange={changeBindings}
                  pickedKey={pickedKey}
                  onPickedKeyHandled={() => setPickedKey(null)}
                  keymap={keymap}
                  onSelectedKeyChange={setSelectedKey}
                  profiles={profiles?.profiles ?? []}
                />
              </>
            ) : kbTab === 'settings' ? (
              <KeyboardOptions status={status} />
            ) : (
              <>
                <section className="stage">
                  <div className="stage-bar">
                    <p className="muted">{tool === 'none' ? fx.hint : tool === 'paint' ? 'Clique ou arraste sobre as teclas para pintar.' : 'Clique ou arraste para devolver as teclas ao efeito.'}</p>
                    <div className="seg" role="group" aria-label="Ferramenta">
                      <button className={tool === 'none' ? 'on' : ''} onClick={() => setTool('none')}>Visualizar</button>
                      <button className={tool === 'paint' ? 'on' : ''} onClick={() => setTool('paint')}>Pintar teclas</button>
                      <button className={tool === 'erase' ? 'on' : ''} onClick={() => setTool('erase')}>Borracha</button>
                    </div>
                  </div>
                  <Keyboard lighting={lighting} ledmap={ledmap} painting={tool !== 'none'} onPaint={paintSlot} />
                </section>

                <section className="controls">
                  {tool === 'none' ? (
                    <>
                      <div className="panel">
                        <h2>Efeito</h2>
                        <div className="chips">
                          {EFFECTS.map((e) => (
                            <button key={e.id} className={'chip' + (e.id === lighting.effect ? ' on' : '')} onClick={() => update({ effect: e.id })}>
                              {e.name}
                            </button>
                          ))}
                        </div>
                      </div>
                      <div className="panel">
                        <h2>Cores</h2>
                        <div className="colors">
                          {lighting.effect !== 'rainbow' && (
                            <ColorField label={fx.two ? (lighting.effect === 'reactive' ? 'Ao apertar' : 'Cor 1') : 'Cor'} value={lighting.color} onChange={(c) => update({ color: c })} />
                          )}
                          {fx.two && (
                            <ColorField label={lighting.effect === 'reactive' ? 'Fundo' : 'Cor 2'} value={lighting.color2} onChange={(c) => update({ color2: c })} />
                          )}
                          {lighting.effect === 'rainbow' && <p className="muted">O arco-íris usa todas as cores.</p>}
                        </div>
                      </div>
                      <div className="panel">
                        <h2>Ajustes</h2>
                        <Slider label="Brilho" value={lighting.brightness} min={0} max={100} step={1} unit="%" onChange={(v) => update({ brightness: v })} />
                        {lighting.effect !== 'static' && (
                          <Slider label="Velocidade" value={lighting.speed} min={0.1} max={3} step={0.1} unit="×" onChange={(v) => update({ speed: v })} />
                        )}
                      </div>
                    </>
                  ) : (
                    <div className="panel wide">
                      <h2>{tool === 'paint' ? 'Cor do pincel' : 'Borracha'}</h2>
                      <p className="muted">
                        {tool === 'paint'
                          ? 'As teclas pintadas ficam por cima do efeito.'
                          : 'As teclas apagadas voltam a mostrar o efeito.'}
                      </p>
                      {tool === 'paint' && (
                        <div className="swatches">
                          {SWATCHES.map((s) => (
                            <button
                              key={toHex(s)}
                              className={'swatch' + (toHex(s) === toHex(brush) ? ' on' : '')}
                              style={{ background: toHex(s) }}
                              aria-label={toHex(s)}
                              onClick={() => setBrush(s)}
                            />
                          ))}
                          <ColorField label="Outra" value={brush} onChange={setBrush} />
                        </div>
                      )}
                      <button className="ghost" disabled={!paintedCount} onClick={() => update({ painted: {} })}>
                        Limpar pintura ({paintedCount})
                      </button>
                    </div>
                  )}
                </section>
              </>
            )}
          </>
        ) : page === 'headset' ? (
          <HeadsetPage key={loadGen} status={status} />
        ) : page === 'mouse' ? (
          <MousePage key={loadGen} status={status} profiles={profiles?.profiles ?? []} />
        ) : page === 'profiles' ? (
          profiles && <ProfilesPage data={profiles} />
        ) : (
          <SettingsPage updater={updater} />
        )}
        </div>
      </main>
    </div>
  );
}

function NavItem(p: { icon: IconName; label: string; sub?: string; online?: boolean; active: boolean; onClick: () => void }) {
  return (
    <button className={'nav-item' + (p.active ? ' active' : '')} aria-current={p.active ? 'page' : undefined} onClick={p.onClick}>
      <span className="ico">
        <Icon name={p.icon} size={18} />
      </span>
      <span>
        {p.sub ? <strong>{p.label}</strong> : p.label}
        {p.sub && <small>{p.sub}</small>}
      </span>
      {p.online !== undefined && <span className={'dot' + (p.online ? ' online' : '')} aria-label={p.online ? 'conectado' : 'desconectado'} />}
    </button>
  );
}

function SettingsPage({ updater }: { updater: ReturnType<typeof useUpdater> }) {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  useEffect(() => {
    api.autostart().then(setAutostart);
    api.appSettings().then(setAppSettings);
  }, []);
  const toggleTray = async () => {
    if (!appSettings) return;
    setAppSettings(await api.patchAppSettings({ battery_tray: !appSettings.battery_tray }));
  };
  const toggle = async () => {
    const next = !autostart;
    await api.setAutostart(next);
    setAutostart(await api.autostart());
  };
  return (
    <>
      <header className="page-head">
        <div>
          <h1>Configurações</h1>
          <p className="page-sub">Preferências do OpenPeripheral</p>
        </div>
      </header>
      <section className="list">
        <div className="list-row">
          <div>
            <strong>Iniciar com o sistema</strong>
            <p>Abre minimizado na bandeja e aplica a iluminação ao ligar o PC.</p>
          </div>
          <Switch on={!!autostart} disabled={autostart === null} onClick={toggle} />
        </div>
        <div className="list-row">
          <div>
            <strong>Bateria do fone na bandeja</strong>
            <p>Ícone colorido perto do relógio: verde, amarelo, vermelho; azul com raio ao carregar.</p>
          </div>
          <Switch on={!!appSettings?.battery_tray} disabled={!appSettings} onClick={toggleTray} />
        </div>
      </section>
      <UpdatesCard updater={updater} />
      <section className="list">
        <div className="list-row">
          <div>
            <strong>OpenPeripheral {__APP_VERSION__}</strong>
            <p>Drivers abertos em espaço de usuário, sem driver de kernel. Licença GPL-3.0. Não é afiliado à Corsair.</p>
          </div>
        </div>
      </section>
    </>
  );
}

function HeadsetPage({ status }: { status: Status | null }) {
  const hs = status?.headset;
  const pct = hs?.battery ?? null;
  const r = 70, c = 2 * Math.PI * r;
  return (
    <>
      <header className="page-head">
        <div>
          <h1>Headset</h1>
          <p className="page-sub">{hs ? `${hs.name}${hs.firmware ? ` · firmware ${hs.firmware}` : ''}` : 'Receptor não encontrado'}</p>
        </div>
      </header>
      <section className="headset">
        <div className="stack">
        <div className="panel battery">
          <svg viewBox="0 0 180 180" width="180" height="180" aria-hidden>
            <circle cx="90" cy="90" r={r} className="ring-bg" />
            <circle
              cx="90" cy="90" r={r}
              className="ring"
              style={{ stroke: pct != null ? batteryColor(pct, !!hs?.charging) : undefined }}
              strokeDasharray={`${((pct ?? 0) / 100) * c} ${c}`}
              transform="rotate(-90 90 90)"
            />
          </svg>
          <div className="battery-text">
            <strong>{pct != null ? `${pct.toFixed(0)}%` : '—'}</strong>
            <small>{!hs ? 'sem receptor' : pct == null ? 'headset desligado' : hs.charging ? '⚡ carregando' : 'na bateria'}</small>
          </div>
        </div>
        <HeadsetOptions status={status} />
        </div>
        <Equalizer />
      </section>
    </>
  );
}

function UpdatesCard({ updater }: { updater: ReturnType<typeof useUpdater> }) {
  const [kind, setKind] = useState<InstallKind | null>(null);
  useEffect(() => {
    installKind().then(setKind, () => setKind('package'));
  }, []);
  const st = updater.state;
  const selfUpdate = kind === 'windows' || kind === 'appimage';
  return (
    <section className="list">
      <div className="list-row">
        <div>
          <strong>Atualizações</strong>
          <p>
            {st.kind === 'checking'
              ? 'Procurando no GitHub…'
              : st.kind === 'none'
                ? `Você já tem a versão mais nova (${__APP_VERSION__}).`
                : st.kind === 'available'
                  ? `Versão ${st.update.version} disponível (você tem ${__APP_VERSION__}).`
                  : st.kind === 'downloading'
                    ? `Baixando… ${st.percent != null ? `${st.percent}%` : ''} O app reinicia sozinho no fim.`
                    : st.kind === 'error'
                      ? `Não deu para verificar: ${st.message}`
                      : `Versão instalada: ${__APP_VERSION__}. O app também procura sozinho ao abrir.`}
          </p>
          {st.kind === 'available' && !selfUpdate && (
            <p>Instalado pelo pacote do sistema: baixe o .rpm ou .deb novo na página de versões.</p>
          )}
        </div>
        {st.kind === 'available' ? (
          selfUpdate ? (
            <button className="primary" onClick={() => updater.install(st.update)}>
              Atualizar agora
            </button>
          ) : (
            <button className="primary" onClick={() => openUrl(RELEASES_URL)}>
              Baixar versão nova
            </button>
          )
        ) : (
          <button className="ghost" disabled={st.kind === 'checking' || st.kind === 'downloading'} onClick={() => updater.find()}>
            Procurar atualização
          </button>
        )}
      </div>
      {st.kind === 'available' && st.update.body && (
        <div className="list-row release-notes">
          <p>{st.update.body.split('\n').filter((l) => l.trim().startsWith('-')).slice(0, 6).join('\n') || st.update.body.slice(0, 400)}</p>
        </div>
      )}
    </section>
  );
}
