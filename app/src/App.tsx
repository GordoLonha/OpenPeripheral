import { useEffect, useRef, useState } from 'react';
import { api, toHex, type AppSettings, type Bindings, type Lighting, type Profiles, type RGB, type Status } from './api';
import { Keyboard, geometryFor } from './Keyboard';
import { Equalizer } from './Equalizer';
import { KeyActions } from './KeyActions';
import { HeadsetOptions, KeyboardOptions, MemoryNote } from './DeviceOptions';
import { ColorField, Slider, Switch } from './controls';
import { Icon } from './icons';
import { MousePage, type MouseTab } from './MousePage';
import { RELEASES_URL, installKind, openUrl, useUpdater, type InstallKind } from './updates';
import { ProfilesPage } from './Profiles';
import { BatteryPill, Home, ScenesPage, batteryColor } from './Home';
import { TopBar, ProfileRail } from './Chrome';
import { DeviceArt, HeadsetArt, type DeviceId } from './DeviceArt';
import { EFFECTS, connected, deviceName } from './fx';
import { Dashboard } from './Dashboard';
import { HeadsetLighting, TempSliders } from './LightEditor';
import { setFahrenheit, startSensors } from './sensors';
import './App.css';

const SWATCHES: RGB[] = [
  [255, 255, 255], [255, 0, 0], [255, 110, 0], [255, 220, 0], [0, 255, 90],
  [0, 200, 255], [0, 70, 255], [140, 0, 255], [255, 0, 170], [0, 0, 0],
];

type Page = 'home' | 'dashboard' | 'scenes' | 'device' | 'settings';
type Tool = 'none' | 'paint' | 'erase';
type KbTab = 'light' | 'keys' | 'settings';
type HsTab = 'eq' | 'light' | 'settings';
type SettingsTab = 'general' | 'profiles' | 'updates';
type DevTabs = { keyboard: KbTab; headset: HsTab; mouse: MouseTab };

/** Menu of each device page, left of the drawing. */
const DEV_MENU: { [K in DeviceId]: [DevTabs[K], string][] } = {
  keyboard: [['light', 'Efeitos de iluminação'], ['keys', 'Atribuições de tecla'], ['settings', 'Ajustes']],
  headset: [['eq', 'Equalizador'], ['light', 'Efeitos de iluminação'], ['settings', 'Ajustes']],
  mouse: [['dpi', 'DPI'], ['buttons', 'Atribuições de botão'], ['light', 'Efeitos de iluminação'], ['settings', 'Ajustes']],
};

const SETTINGS_MENU: [SettingsTab, string][] = [
  ['general', 'Geral'],
  ['profiles', 'Perfis'],
  ['updates', 'Atualizações'],
];

const DEVICES: DeviceId[] = ['keyboard', 'headset', 'mouse'];

export default function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [lighting, setLighting] = useState<Lighting | null>(null);
  /** The headset's own lighting in the profile in use; null = it follows the keyboard. */
  const [headsetOwn, setHeadsetOwn] = useState<Lighting | null>(null);
  const [keymap, setKeymap] = useState<Record<string, number | null>>({});
  const [ledmap, setLedmap] = useState<Record<string, number | null>>({});
  const [page, setPage] = useState<Page>('home');
  const [device, setDevice] = useState<DeviceId>('keyboard');
  const [devTab, setDevTab] = useState<DevTabs>({ keyboard: 'light', headset: 'eq', mouse: 'dpi' });
  const [settingsTab, setSettingsTab] = useState<SettingsTab>('general');
  const [tool, setTool] = useState<Tool>('none');
  const [brush, setBrush] = useState<RGB>([255, 0, 0]);
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
    Promise.all([api.lighting(), api.bindings(), api.headsetLighting()]).then(([l, b, h]) => {
      setLighting(l);
      setBindings(b);
      setHeadsetOwn(h);
      setLoadGen((g) => g + 1);
    });

  const openDevice = (id: DeviceId, settings = false) => {
    setPage('device');
    setDevice(id);
    if (settings) setDevTab((t) => ({ ...t, [id]: 'settings' }));
  };
  const openSettings = (tab: SettingsTab) => {
    setPage('settings');
    setSettingsTab(tab);
  };

  useEffect(() => {
    startSensors();
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
    // The tray asks for a page by name: a device, "profiles", "settings" or "home".
    const unNav = api.onNavigate((p) => {
      if ((DEVICES as string[]).includes(p)) {
        setPage('device');
        setDevice(p as DeviceId);
      } else if (p === 'profiles' || p === 'settings') {
        setPage('settings');
        setSettingsTab(p === 'profiles' ? 'profiles' : 'general');
      } else {
        setPage('home');
      }
    });
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
  const deviceHere = page !== 'device' || !!status?.[device];
  useEffect(() => {
    if (deviceHere || !status) return;
    const t = window.setTimeout(() => setPage('home'), 3000);
    return () => window.clearTimeout(t);
  }, [deviceHere, status]);

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
  const random = !!fx.random && !!lighting.random;
  // Random colours replace the drops' colour (Rain) or both colours (Type).
  const showFirst = fx.colors.length > 0 && !random;
  const showSecond = fx.colors.length > 1 && !(random && lighting.effect === 'type');
  // An effect without the saved direction shows (and the engine uses) its first one.
  const dir = fx.dirs?.some(([d]) => d === lighting.direction) ? lighting.direction : fx.dirs?.[0][0];
  const paintedCount = Object.keys(lighting.painted).length;
  const onScene = (patch: Partial<Lighting>) => update({ ...patch, painted: {} });

  const keyboardPage = () => {
    const tab = devTab.keyboard;
    if (tab === 'settings') return <KeyboardOptions status={status} />;
    if (tab === 'keys')
      return (
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
      );
    return (
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

        {tool === 'none' ? (
          <section className="panel effect-panel">
            <h2>Efeitos de iluminação</h2>
            <div className="eyebrow">Efeito</div>
            <div className="effect-tiles">
              {EFFECTS.map((e) => (
                <button key={e.id} className={'option' + (e.id === lighting.effect ? ' on' : '')} aria-pressed={e.id === lighting.effect} onClick={() => update({ effect: e.id })}>
                  {e.name}
                </button>
              ))}
            </div>
            <div className="effect-settings">
              <div className="colors">
                {showFirst && <ColorField label={fx.colors[0]} value={lighting.color} onChange={(c) => update({ color: c })} />}
                {showSecond && <ColorField label={fx.colors[1]} value={lighting.color2} onChange={(c) => update({ color2: c })} />}
                {fx.colors.length === 0 && <p className="muted">Este efeito usa todas as cores do arco-íris.</p>}
                {random && lighting.effect === 'type' && <p className="muted">Cada tecla acende numa cor sorteada.</p>}
                {fx.dirs && (
                  <div className="effect-option">
                    <span>Direção</span>
                    <div className="seg" role="group" aria-label="Direção do efeito">
                      {fx.dirs.map(([d, label, name]) => (
                        <button key={d} className={dir === d ? 'on' : ''} aria-label={name} title={name} onClick={() => update({ direction: d })}>
                          {label}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
                {fx.double && (
                  <div className="effect-option">
                    <span>Dos dois lados (a partir do meio)</span>
                    <Switch on={!!lighting.double_sided} onClick={() => update({ double_sided: !lighting.double_sided })} />
                  </div>
                )}
                {fx.random && (
                  <div className="effect-option">
                    <span>Cores aleatórias</span>
                    <Switch on={random} onClick={() => update({ random: !random })} />
                  </div>
                )}
              </div>
              <div>
                <Slider label="Brilho" value={lighting.brightness} min={0} max={100} step={1} unit="%" onChange={(v) => update({ brightness: v })} />
                {!fx.still && <Slider label="Velocidade" value={lighting.speed} min={0.1} max={3} step={0.1} unit="×" onChange={(v) => update({ speed: v })} />}
                {lighting.effect === 'temperature' && <TempSliders value={lighting.temps} onChange={(temps) => update({ temps })} />}
              </div>
            </div>
          </section>
        ) : (
          <section className="panel wide">
            <h2>{tool === 'paint' ? 'Cor do pincel' : 'Borracha'}</h2>
            <p className="muted">{tool === 'paint' ? 'As teclas pintadas ficam por cima do efeito.' : 'As teclas apagadas voltam a mostrar o efeito.'}</p>
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
          </section>
        )}
      </>
    );
  };

  const devicePage = () => {
    const here = connected(status);
    // Keep the open device in the strip while it re-connects.
    const strip = here.includes(device) ? here : [...here, device];
    const online = !!status?.[device];
    const sub =
      device === 'keyboard'
        ? status?.keyboard && `firmware ${status.keyboard.firmware}`
        : device === 'headset'
          ? status?.headset?.firmware && `firmware ${status.headset.firmware}`
          : status?.mouse && `firmware ${status.mouse.firmware}`;
    const menu = DEV_MENU[device] as [string, string][];
    const tab: string = devTab[device];
    const setTab = (t: string) => setDevTab((cur) => ({ ...cur, [device]: t }));
    return (
      <div className="device-page">
        <nav className="dev-strip" aria-label="Aparelhos">
          {strip.map((id) => (
            <button key={id} className={'strip-item ' + id + (id === device ? ' on' : '')} title={deviceName(id, status)} aria-current={id === device} onClick={() => openDevice(id)}>
              <DeviceArt id={id} lighting={lighting} headset={headsetOwn} ledmap={ledmap} />
            </button>
          ))}
        </nav>
        {/* Re-mounted per device and section, so each one opens scrolled to the top. */}
        <div className="side-layout" key={device + tab}>
          <aside className="side-panel">
            <div className="side-head">
              <h2>{deviceName(device, status)}</h2>
              <button className="icon-btn" title="Ajustes do aparelho" aria-label="Ajustes do aparelho" onClick={() => setTab('settings')}>
                <Icon name="settings" size={17} />
              </button>
            </div>
            {sub && <p className="muted small">{sub}</p>}
            {device === 'headset' && status?.headset?.battery != null && (
              <BatteryPill pct={status.headset.battery} charging={status.headset.charging} />
            )}
            <nav className="side-menu" aria-label="Seções do aparelho">
              {menu.map(([id, label]) => (
                <button key={id} className={tab === id ? 'on' : ''} aria-current={tab === id} onClick={() => setTab(id)}>
                  {label}
                </button>
              ))}
            </nav>
            <p className={'dev-state' + (online ? ' ok' : '')}>{online ? 'Conectado' : 'Desconectado'}</p>
          </aside>
          <div className="side-content">
            {tab !== 'settings' && <MemoryNote device={device} status={status} />}
            {device === 'keyboard' ? (
              keyboardPage()
            ) : device === 'headset' ? (
              <HeadsetPage key={loadGen} status={status} tab={devTab.headset} lighting={lighting} own={headsetOwn} onOwn={setHeadsetOwn} />
            ) : (
              <MousePage key={loadGen} status={status} profiles={profiles?.profiles ?? []} tab={devTab.mouse} keyboard={lighting} />
            )}
          </div>
        </div>
      </div>
    );
  };

  const settingsPage = () => (
    <div className="side-layout settings-page">
      <aside className="side-panel">
        <div className="side-head">
          <h2>Configurações</h2>
        </div>
        <nav className="side-menu" aria-label="Seções das configurações">
          {SETTINGS_MENU.map(([id, label]) => (
            <button key={id} className={settingsTab === id ? 'on' : ''} aria-current={settingsTab === id} onClick={() => setSettingsTab(id)}>
              {label}
            </button>
          ))}
        </nav>
      </aside>
      <div className="side-content">
        {settingsTab === 'profiles' ? profiles && <ProfilesPage data={profiles} /> : settingsTab === 'updates' ? <UpdatesCard updater={updater} /> : <GeneralSettings />}
      </div>
    </div>
  );

  return (
    <div className="shell">
      <TopBar
        tab={page === 'home' || page === 'dashboard' || page === 'scenes' ? page : null}
        onTab={setPage}
        brightness={lighting.brightness}
        onBrightness={(v) => update({ brightness: v })}
        inSettings={page === 'settings'}
        onSettings={() => openSettings('general')}
        onAbout={() => openSettings('general')}
      />
      <ProfileRail data={profiles} />
      <main className={'main page-' + page}>
        {(updater.state.kind === 'available' && page !== 'settings') || status?.icue_running ? (
          <div className="banners">
            {updater.state.kind === 'available' && page !== 'settings' && (
              <div className="banner info" role="status">
                <span>Nova versão {updater.state.update.version} disponível.</span>
                <button className="ghost" onClick={() => openSettings('updates')}>
                  Ver atualização
                </button>
              </div>
            )}
            {status?.icue_running && (
              <div className="banner" role="status">
                O iCUE está aberto e controla o teclado. Feche-o (bandeja → Sair) para o OpenPeripheral assumir.
              </div>
            )}
          </div>
        ) : null}

        {page === 'home' ? (
          <Home status={status} lighting={lighting} headset={headsetOwn} ledmap={ledmap} onScene={onScene} onOpen={openDevice} onScenes={() => setPage('scenes')} onDashboard={() => setPage('dashboard')} />
        ) : page === 'dashboard' ? (
          <Dashboard />
        ) : page === 'scenes' ? (
          <ScenesPage status={status} lighting={lighting} headset={headsetOwn} ledmap={ledmap} onScene={onScene} />
        ) : page === 'device' ? (
          devicePage()
        ) : (
          settingsPage()
        )}
      </main>
    </div>
  );
}

function GeneralSettings() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  useEffect(() => {
    api.autostart().then(setAutostart);
    api.appSettings().then(setAppSettings);
  }, []);
  const flip = async (key: 'battery_tray' | 'notify_battery' | 'notify_profile') => {
    if (!appSettings) return;
    setAppSettings(await api.patchAppSettings({ [key]: !appSettings[key] }));
  };
  const setUnit = async (f: boolean) => {
    setFahrenheit(f);
    setAppSettings(await api.patchAppSettings({ fahrenheit: f }));
  };
  const toggle = async () => {
    const next = !autostart;
    await api.setAutostart(next);
    setAutostart(await api.autostart());
  };
  return (
    <>
      <h3 className="section-label">Geral</h3>
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
          <Switch on={!!appSettings?.battery_tray} disabled={!appSettings} onClick={() => flip('battery_tray')} />
        </div>
      </section>
      <h3 className="section-label">Notificações</h3>
      <section className="list">
        <div className="list-row">
          <div>
            <strong>Bateria do headset</strong>
            <p>Avisa quando a bateria do HS80 chega a 20% e a 10%, e quando termina de carregar no cabo.</p>
          </div>
          <Switch on={!!appSettings?.notify_battery} disabled={!appSettings} onClick={() => flip('notify_battery')} />
        </div>
        <div className="list-row">
          <div>
            <strong>Troca automática de perfil</strong>
            <p>Avisa quando um jogo ou programa ativa outro perfil.</p>
          </div>
          <Switch on={!!appSettings?.notify_profile} disabled={!appSettings} onClick={() => flip('notify_profile')} />
        </div>
      </section>
      <h3 className="section-label">Sensores</h3>
      <section className="list">
        <div className="list-row">
          <div>
            <strong>Unidade de temperatura</strong>
            <p>No Painel, no Início, no efeito Temperatura e no registro em CSV.</p>
          </div>
          <div className="seg" role="group" aria-label="Unidade de temperatura">
            <button className={appSettings && !appSettings.fahrenheit ? 'on' : ''} disabled={!appSettings} onClick={() => setUnit(false)}>
              °C
            </button>
            <button className={appSettings?.fahrenheit ? 'on' : ''} disabled={!appSettings} onClick={() => setUnit(true)}>
              °F
            </button>
          </div>
        </div>
      </section>
      <h3 className="section-label">Sobre</h3>
      <section className="list">
        <div className="list-row">
          <div>
            <strong>OpenPeripheral {__APP_VERSION__}</strong>
            <p>Drivers abertos em espaço de usuário, sem driver de kernel. Licença GPL-3.0. Não é afiliado à Corsair.</p>
            <p>Fechar a janela mantém as luzes pela bandeja.</p>
          </div>
        </div>
      </section>
    </>
  );
}

function HeadsetPage({ status, tab, lighting, own, onOwn }: {
  status: Status | null;
  tab: HsTab;
  /** The keyboard's lighting, which the logo shows while it follows the keyboard. */
  lighting: Lighting;
  own: Lighting | null;
  onOwn: (l: Lighting | null) => void;
}) {
  const hs = status?.headset;
  const pct = hs?.battery ?? null;
  const r = 70, c = 2 * Math.PI * r;
  return (
    <>
      <section className="stage art-stage">
        <HeadsetArt lighting={own ?? lighting} />
      </section>
      {tab === 'eq' ? (
        <Equalizer />
      ) : tab === 'light' ? (
        <HeadsetLighting keyboard={lighting} onChange={onOwn} />
      ) : (
        <section className="headset">
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
        </section>
      )}
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
    <>
      <h3 className="section-label">Atualizações</h3>
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
            {st.kind === 'available' && !selfUpdate && <p>Instalado pelo pacote do sistema: baixe o .rpm ou .deb novo na página de versões.</p>}
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
    </>
  );
}
