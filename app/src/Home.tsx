import { useEffect, useState } from 'react';
import { api, type Eq, type Lighting, type Profiles, type Status } from './api';
import { Keyboard } from './Keyboard';
import { Switch } from './controls';
import { Icon } from './icons';
import { POLLING, useAppSettings } from './settings';
import { H, PRESETS, W, eqPath, yOf } from './eqmath';
import { EFFECTS, SCENES, batteryColor, sceneOf } from './display';

type Props = {
  status: Status | null;
  profiles: Profiles | null;
  lighting: Lighting;
  ledmap: Record<string, number | null>;
  onOpen: (page: 'keyboard' | 'headset' | 'mouse') => void;
  onLighting: (patch: Partial<Lighting>) => void;
  onScene: (patch: Partial<Lighting>) => void;
};

/** Início: the keyboard lit as it is now, quick controls, one card per other device, scenes. */
export function Home({ status, profiles, lighting, ledmap, onOpen, onLighting, onScene }: Props) {
  const [settings, patchSettings] = useAppSettings();
  const kb = status?.keyboard;
  const hs = status?.headset;
  const anyDevice = !!(kb || hs || status?.mouse);
  const active = profiles?.profiles.find((p) => p.id === profiles.active);
  const scene = sceneOf(lighting);
  const lightName = scene?.name ?? EFFECTS.find((e) => e.id === lighting.effect)?.name ?? '';
  // The Win Lock key toggles it on the keyboard: the live state comes with the status.
  const winLock = status?.win_lock ?? settings?.win_lock ?? false;
  const polling = POLLING.find((p) => p.ms === settings?.polling_ms)?.label;

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Início</h1>
          <p className="page-sub">
            Perfil em uso: <strong className="accent">{active?.name ?? '—'}</strong>
            {profiles?.auto_app && <> · ativado por {profiles.auto_app}</>}
          </p>
        </div>
      </header>

      {!anyDevice && (
        <section className="card empty-state">
          <h2 className="card-title">Nenhum aparelho Corsair encontrado</h2>
          <p className="muted">
            Conecte o teclado, o fone ou o mouse: eles aparecem aqui sozinhos.
            {status?.icue_running && ' Feche o iCUE (bandeja → Sair), senão ele fica com os aparelhos.'}
          </p>
        </section>
      )}

      {kb && (
        <section className="card home-hero">
          <div className="home-hero-stage">
            <div className="card-head">
              <div>
                <h2 className="card-title">K70 RGB MK.2 Low Profile</h2>
                <p className="muted small">
                  conectado · firmware {kb.firmware}
                  {polling && ` · ${polling}`}
                </p>
              </div>
              <button className="ghost" onClick={() => onOpen('keyboard')}>
                Abrir teclado
              </button>
            </div>
            <Keyboard lighting={lighting} ledmap={ledmap} painting={false} />
          </div>
          <div className="home-hero-controls">
            <div>
              <div className="eyebrow">{scene ? 'Cena' : 'Efeito'}</div>
              <div className="card-title home-light-name">{lightName}</div>
            </div>
            <label className="slider">
              <span>
                Brilho <output>{lighting.brightness}%</output>
              </span>
              <input
                type="range"
                min={0}
                max={100}
                step={1}
                value={lighting.brightness}
                onChange={(e) => onLighting({ brightness: Number(e.target.value) })}
              />
            </label>
            <div className="home-row">
              <span>
                Win Lock
                <small>{winLock ? 'Tecla Windows bloqueada' : 'Tecla Windows liberada'}</small>
              </span>
              <Switch on={winLock} disabled={!settings} label="Win Lock" onClick={() => patchSettings({ win_lock: !winLock })} />
            </div>
            <div className="home-row">
              <span>
                Perfil automático
                <small>{profiles?.auto_app ? `${active?.name} enquanto ${profiles.auto_app} estiver na frente` : 'Nenhum programa ligado na frente'}</small>
              </span>
            </div>
          </div>
        </section>
      )}

      {(hs || status?.mouse) && (
        // With the mouse too, headset and mouse share a row and the EQ takes the next one.
        <section className={'home-devices' + (hs && status?.mouse ? ' three' : '')}>
          {hs && <HeadsetCard hs={hs} sleepMin={settings?.headset_sleep_min ?? null} onOpen={() => onOpen('headset')} />}
          {status?.mouse && <MouseCard mouse={status.mouse} onOpen={() => onOpen('mouse')} />}
          {hs && <EqCard />}
        </section>
      )}

      {(kb || status?.mouse) && (
        <section className="home-section">
          <div className="home-section-head">
            <h2 className="card-title">Cenas de iluminação</h2>
            <p className="muted small">Aplica no perfil em uso, no teclado e na luz do mouse que segue o teclado.</p>
          </div>
          <div className="scenes">
            {SCENES.map((s) => (
              <button
                key={s.id}
                className={'scene' + (s.id === scene?.id ? ' on' : '')}
                aria-pressed={s.id === scene?.id}
                onClick={() => onScene(s.patch)}
              >
                <span className="scene-swatch" style={{ background: s.css }} aria-hidden />
                <span className="scene-name">
                  {s.name}
                  {s.id === scene?.id && <em>em uso</em>}
                </span>
              </button>
            ))}
          </div>
        </section>
      )}
    </>
  );
}

function BatteryRing({ pct, charging, size = 96 }: { pct: number | null; charging: boolean; size?: number }) {
  const stroke = 9;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const color = pct == null ? 'var(--faint)' : batteryColor(pct, charging);
  return (
    <div className="battery-ring" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
        <circle cx={size / 2} cy={size / 2} r={r} className="track" strokeWidth={stroke} />
        {pct != null && (
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke={color}
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={`${(c * pct) / 100} ${c}`}
            transform={`rotate(-90 ${size / 2} ${size / 2})`}
          />
        )}
      </svg>
      <div className="battery-ring-label">
        <strong>{pct == null ? '—' : `${pct.toFixed(0)}%`}</strong>
        <small>{charging ? 'carregando' : 'bateria'}</small>
      </div>
    </div>
  );
}

function HeadsetCard({ hs, sleepMin, onOpen }: { hs: NonNullable<Status['headset']>; sleepMin: number | null; onOpen: () => void }) {
  const wired = hs.name.includes('cabo');
  const parts = [
    hs.battery == null ? 'headset desligado' : wired ? 'cabo USB' : 'sem fio',
    hs.mic_muted == null ? null : hs.mic_muted ? 'microfone mudo' : 'microfone ligado',
    sleepMin ? `desliga em ${sleepMin} min parado` : null,
  ].filter(Boolean);
  return (
    <div className="card home-device">
      <BatteryRing pct={hs.battery} charging={hs.charging} />
      <div className="home-device-body">
        <div className="card-head">
          <div>
            <h2 className="card-title">HS80 RGB Wireless</h2>
            <p className="muted small">{parts.join(' · ')}</p>
          </div>
          <button className="ghost" onClick={onOpen}>
            Abrir
          </button>
        </div>
      </div>
    </div>
  );
}

/** Preset chips and the curve; the full editor is on the headset page. */
function EqCard() {
  const [eq, setEq] = useState<Eq | null>(null);
  useEffect(() => {
    api.eq().then(setEq);
    const un = api.onEq(setEq);
    return () => {
      un.then((f) => f());
    };
  }, []);
  if (!eq) return null;
  const pick = (id: string, gains: number[]) => {
    const next = { ...eq, enabled: true, preset: id, gains: [...gains] };
    setEq(next);
    api.setEq(next).catch(() => api.eq().then(setEq));
  };
  const curve = eqPath(eq.gains);
  return (
    <div className="card home-eq">
      <div className="card-head">
        <h2 className="card-title">Equalizador</h2>
        <span className="muted small">{eq.enabled ? 'clique na roda do fone troca' : 'desligado'}</span>
      </div>
      <div className="chips">
        {PRESETS.map((p) => (
          <button key={p.id} className={'chip' + (eq.enabled && eq.preset === p.id ? ' on' : '')} onClick={() => pick(p.id, p.gains)}>
            {p.name}
          </button>
        ))}
      </div>
      <svg className={'home-eq-curve' + (eq.enabled ? '' : ' off')} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden>
        <line x1={0} x2={W} y1={yOf(0)} y2={yOf(0)} className="zero" />
        <path d={`${curve} L${W},${H} L0,${H} Z`} className="fill" />
        <path d={curve} className="line" />
      </svg>
    </div>
  );
}

function MouseCard({ mouse, onOpen }: { mouse: NonNullable<Status['mouse']>; onOpen: () => void }) {
  return (
    <div className="card home-device">
      <span className="home-device-glyph">
        <Icon name="mouse" size={44} />
      </span>
      <div className="home-device-body">
        <div className="card-head">
          <div>
            <h2 className="card-title">{mouse.name}</h2>
            <p className="muted small">firmware {mouse.firmware}</p>
          </div>
          <button className="ghost" onClick={onOpen}>
            Abrir
          </button>
        </div>
        <div className="home-row">
          <span>Estágio de DPI</span>
          <div className="seg" role="group" aria-label="Estágio de DPI">
            {[0, 1, 2, 3, 4].map((i) => (
              <button key={i} className={mouse.dpi_stage === i ? 'on' : ''} aria-pressed={mouse.dpi_stage === i} onClick={() => api.setMouseStage(i)}>
                {i + 1}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
