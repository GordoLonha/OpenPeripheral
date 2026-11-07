import type { Lighting, Status } from './api';
import { DeviceArt, LightSwatch, type DeviceId } from './DeviceArt';
import { SCENES, connected, deviceName, effectName, sceneLighting, sceneOn } from './fx';
import { Icon } from './icons';
import { HomeSensors } from './Dashboard';

type Common = {
  status: Status | null;
  lighting: Lighting;
  ledmap: Record<string, number | null>;
  onScene: (patch: Partial<Lighting>) => void;
};

/** Scene swatches, each one animated with its own lighting. */
export function SceneGrid({ lighting, onScene, cols }: { lighting: Lighting; onScene: (patch: Partial<Lighting>) => void; cols: 2 | 3 }) {
  return (
    <div className={`scene-grid cols-${cols}`}>
      {SCENES.map((s) => {
        const on = sceneOn(lighting, s.patch);
        return (
          <button key={s.id} className={'scene' + (on ? ' on' : '')} aria-pressed={on} onClick={() => onScene(s.patch)}>
            <LightSwatch lighting={sceneLighting(s.patch)} />
            <span>{s.name}</span>
          </button>
        );
      })}
    </div>
  );
}

export function Home({ status, lighting, ledmap, onScene, onOpen, onScenes, onDashboard }: Common & {
  onOpen: (id: DeviceId, settings?: boolean) => void;
  onScenes: () => void;
  onDashboard: () => void;
}) {
  const devices = connected(status);
  const hs = status?.headset;
  return (
    <div className="home">
      <aside className="home-left">
        <div className="home-left-head">
          <h2>Cenas</h2>
          <button className="icon-btn" title="Abrir Cenas" aria-label="Abrir Cenas" onClick={onScenes}>
            <Icon name="plus" size={18} />
          </button>
        </div>
        <SceneGrid lighting={lighting} onScene={onScene} cols={2} />
        <p className="muted small">Aplica no perfil em uso: teclado, headset e a luz do mouse que segue o teclado.</p>
        <div className="home-left-head">
          <h2>Sensores</h2>
          <button className="icon-btn" title="Abrir Painel" aria-label="Abrir Painel" onClick={onDashboard}>
            <Icon name="plus" size={18} />
          </button>
        </div>
        <HomeSensors />
      </aside>

      <section className="home-devices">
        {devices.length === 0 && (
          <div className="empty-devices">
            <h2>Nenhum aparelho Corsair encontrado</h2>
            <p className="muted">
              Conecte o teclado, o fone ou o mouse: eles aparecem aqui sozinhos.
              {status?.icue_running && ' Feche o iCUE (bandeja → Sair), senão ele fica com os aparelhos.'}
            </p>
          </div>
        )}
        {devices.map((id) => (
          <article key={id} className="dev-card">
            <button className="dev-card-open" aria-label={`Abrir ${deviceName(id, status)}`} onClick={() => onOpen(id)} />
            <header>
              <h3>{deviceName(id, status)}</h3>
              <button className="icon-btn" title="Ajustes do aparelho" aria-label={`Ajustes de ${deviceName(id, status)}`} onClick={() => onOpen(id, true)}>
                <Icon name="settings" size={17} />
              </button>
            </header>
            <div className="dev-card-art">
              <DeviceArt id={id} lighting={lighting} ledmap={ledmap} />
            </div>
            <footer>
              <span>
                {id === 'headset'
                  ? hs?.battery == null
                    ? 'Headset desligado'
                    : effectName(lighting.effect)
                  : id === 'mouse'
                    ? `DPI ${(status?.mouse?.dpi_stage ?? 0) + 1} · ${effectName(lighting.effect)}`
                    : effectName(lighting.effect)}
              </span>
              {id === 'headset' && hs?.battery != null && <BatteryPill pct={hs.battery} charging={hs.charging} />}
            </footer>
          </article>
        ))}
      </section>
    </div>
  );
}

/** "Cenas": every scene on the left, the devices showing the lighting on the right. */
export function ScenesPage({ status, lighting, ledmap, onScene }: Common) {
  const devices = connected(status);
  return (
    <div className="scenes-page">
      <aside className="scenes-left">
        <h2>Cenas</h2>
        <p className="muted small">Clique numa cena para usar no perfil em uso. Para ajustar cores e velocidade, abra o teclado.</p>
        <SceneGrid lighting={lighting} onScene={onScene} cols={3} />
      </aside>
      <section className="scenes-stage">
        <div className="scenes-now">
          Em uso: <strong>{effectName(lighting.effect)}</strong>
        </div>
        <div className="scenes-devices">
          {devices.length === 0 && <p className="muted">Conecte um aparelho para ver a cena nele.</p>}
          {devices.map((id) => (
            <figure key={id} className={'scene-dev ' + id}>
              <figcaption>{deviceName(id, status)}</figcaption>
              <DeviceArt id={id} lighting={lighting} ledmap={ledmap} />
            </figure>
          ))}
        </div>
      </section>
    </div>
  );
}

export function batteryColor(pct: number, charging: boolean): string {
  if (charging) return 'var(--accent)';
  if (pct < 15) return 'var(--danger)';
  if (pct < 40) return 'var(--warn)';
  return 'var(--ok)';
}

export function BatteryPill({ pct, charging }: { pct: number; charging: boolean }) {
  return (
    <span className="battery-pill" style={{ color: batteryColor(pct, charging) }}>
      {pct.toFixed(0)}%{charging && ' ⚡'}
      <span className="battery-shape" aria-hidden>
        <span style={{ width: `${Math.max(8, pct)}%`, background: batteryColor(pct, charging) }} />
      </span>
    </span>
  );
}
