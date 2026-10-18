import { useEffect, useRef, useState } from 'react';
import { api, toHex, type AppSettings, type Bindings, type Effect, type Lighting, type Profiles, type RGB, type Status } from './api';
import { Keyboard, geometryFor } from './Keyboard';
import { Equalizer } from './Equalizer';
import { KeyActions } from './KeyActions';
import { HeadsetOptions, KeyboardOptions } from './DeviceOptions';
import { ColorField, Slider } from './controls';
import { MousePage } from './MousePage';
import { ProfileSwitcher, ProfilesPage } from './Profiles';
import { Home, batteryColor } from './Home';
import './App.css';

const EFFECTS: { id: Effect; name: string; hint: string; two?: boolean }[] = [
  { id: 'static', name: 'Cor fixa', hint: 'Uma cor em todo o teclado' },
  { id: 'rainbow', name: 'Arco-íris', hint: 'Espectro correndo da esquerda para a direita' },
  { id: 'wave', name: 'Onda', hint: 'Onda entre duas cores', two: true },
  { id: 'breathing', name: 'Respiração', hint: 'Pulsa suavemente entre duas cores', two: true },
  { id: 'reactive', name: 'Reativo', hint: 'A tecla acende quando você aperta', two: true },
];

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
  const [kbTab, setKbTab] = useState<'light' | 'keys'>('light');
  const [bindings, setBindings] = useState<Bindings>({});
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [pickedKey, setPickedKey] = useState<string | null>(null);
  // Bumped whenever per-profile data is (re)loaded, to re-mount editors with it.
  const [loadGen, setLoadGen] = useState(0);
  const [profiles, setProfiles] = useState<Profiles | null>(null);
  const activeRef = useRef<string | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);

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
        <button className={'nav-link' + (page === 'home' ? ' active' : '')} onClick={() => setPage('home')}>
          Início
        </button>
        <nav>
          <DeviceTab
            active={page === 'keyboard'}
            onClick={() => setPage('keyboard')}
            title="Teclado"
            subtitle={status?.keyboard ? `K70 MK.2 LP · fw ${status.keyboard.firmware}` : 'desconectado'}
            online={!!status?.keyboard}
          />
          <DeviceTab
            active={page === 'headset'}
            onClick={() => setPage('headset')}
            title="Headset"
            subtitle={
              status?.headset
                ? status.headset.battery != null
                  ? `HS80 · ${status.headset.battery.toFixed(0)}%${status.headset.charging ? ' · carregando' : ''}`
                  : 'HS80 · desligado'
                : 'desconectado'
            }
            online={!!status?.headset}
          />
          {(status?.mouse || page === 'mouse') && (
            <DeviceTab
              active={page === 'mouse'}
              onClick={() => setPage('mouse')}
              title="Mouse"
              subtitle={status?.mouse ? `M65 · DPI ${status.mouse.dpi_stage + 1}` : 'desconectado'}
              online={!!status?.mouse}
            />
          )}
        </nav>
        <button className={'nav-link' + (page === 'profiles' ? ' active' : '')} onClick={() => setPage('profiles')}>
          Perfis
        </button>
        <button className={'nav-link' + (page === 'settings' ? ' active' : '')} onClick={() => setPage('settings')}>
          Configurações
        </button>
        <footer className="sidebar-foot">Fechar a janela mantém as luzes ativas pela bandeja.</footer>
      </aside>

      <main className="main">
        {status?.icue_running && (
          <div className="banner" role="status">
            O iCUE está aberto e controla o teclado. Feche-o (bandeja → Sair) para o OpenPeripheral assumir.
          </div>
        )}

        {page === 'home' ? (
          <Home status={status} profiles={profiles} onOpen={setPage} onScene={(patch) => update({ ...patch, painted: {} })} />
        ) : page === 'keyboard' && kbTab === 'keys' ? (
          <>
            <header className="page-head">
              <div>
                <h1>Teclas</h1>
                <p className="muted">Troque o que cada tecla faz. Funciona em qualquer programa, sem o iCUE.</p>
              </div>
              <TabSwitch tab={kbTab} onTab={setKbTab} />
            </header>
            <section className="stage">
              <Keyboard
                lighting={lighting}
                ledmap={ledmap}
                painting={false}
                onSelectKey={setPickedKey}
                selectedKey={selectedKey}
                marked={new Set(Object.keys(bindings))}
              />
            </section>
            {(
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
            )}
            <KeyboardOptions status={status} />
          </>
        ) : page === 'keyboard' ? (
          <>
            <header className="page-head">
              <div>
                <h1>Iluminação</h1>
                <p className="muted">{fx.hint}</p>
              </div>
              <TabSwitch tab={kbTab} onTab={setKbTab} />
              <div className="seg" role="group" aria-label="Ferramenta">
                <button className={tool === 'none' ? 'on' : ''} onClick={() => setTool('none')}>Visualizar</button>
                <button className={tool === 'paint' ? 'on' : ''} onClick={() => setTool('paint')}>Pintar teclas</button>
                <button className={tool === 'erase' ? 'on' : ''} onClick={() => setTool('erase')}>Borracha</button>
              </div>
            </header>

            <section className="stage">
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
                  <h2>{tool === 'paint' ? 'Pintar teclas' : 'Borracha'}</h2>
                  <p className="muted">
                    {tool === 'paint'
                      ? 'Clique ou arraste sobre as teclas. As teclas pintadas ficam por cima do efeito.'
                      : 'Clique ou arraste para devolver as teclas ao efeito.'}
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
        ) : page === 'headset' ? (
          <HeadsetPage key={loadGen} status={status} />
        ) : page === 'mouse' ? (
          <MousePage key={loadGen} status={status} profiles={profiles?.profiles ?? []} />
        ) : page === 'profiles' ? (
          profiles && <ProfilesPage data={profiles} />
        ) : (
          <SettingsPage />
        )}
      </main>
    </div>
  );
}

function TabSwitch({ tab, onTab }: { tab: 'light' | 'keys'; onTab: (t: 'light' | 'keys') => void }) {
  return (
    <div className="seg big" role="tablist" aria-label="Seção do teclado">
      <button role="tab" aria-selected={tab === 'light'} className={tab === 'light' ? 'on' : ''} onClick={() => onTab('light')}>Iluminação</button>
      <button role="tab" aria-selected={tab === 'keys'} className={tab === 'keys' ? 'on' : ''} onClick={() => onTab('keys')}>Teclas</button>
    </div>
  );
}

function DeviceTab(p: { active: boolean; onClick: () => void; title: string; subtitle: string; online: boolean }) {
  return (
    <button className={'device' + (p.active ? ' active' : '')} onClick={p.onClick}>
      <span className={'dot' + (p.online ? ' online' : '')} aria-hidden />
      <span>
        <strong>{p.title}</strong>
        <small>{p.subtitle}</small>
      </span>
    </button>
  );
}

function SettingsPage() {
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
          <p className="muted">Preferências do OpenPeripheral</p>
        </div>
      </header>
      <section className="settings">
        <div className="panel setting">
          <div>
            <strong>Iniciar com o sistema</strong>
            <p className="muted">Abre minimizado na bandeja e aplica a iluminação ao ligar o PC.</p>
          </div>
          <button
            role="switch"
            aria-checked={!!autostart}
            className={'switch' + (autostart ? ' on' : '')}
            disabled={autostart === null}
            onClick={toggle}
          >
            <span />
          </button>
        </div>
        <div className="panel setting">
          <div>
            <strong>Bateria do fone na bandeja</strong>
            <p className="muted">Ícone colorido perto do relógio: verde, amarelo, vermelho, azul com raio ao carregar.</p>
          </div>
          <button
            role="switch"
            aria-checked={!!appSettings?.battery_tray}
            className={'switch' + (appSettings?.battery_tray ? ' on' : '')}
            disabled={!appSettings}
            onClick={toggleTray}
          >
            <span />
          </button>
        </div>
        <div className="panel setting">
          <div>
            <strong>Sobre</strong>
            <p className="muted">
              OpenPeripheral 0.2 — drivers abertos em espaço de usuário (sem driver de kernel). Não é afiliado à Corsair.
            </p>
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
          <p className="muted">{hs ? hs.name : 'Receptor não encontrado'}</p>
        </div>
      </header>
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
        <Equalizer />
      </section>
      <HeadsetOptions status={status} />
    </>
  );
}
