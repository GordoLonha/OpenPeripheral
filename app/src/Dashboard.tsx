import type { Reading } from './api';
import { GROUPS, UNIT, cpuTemp, formatValue, sensorName, sensorSource, useSensors } from './sensors';

/** One sensor: name, value, min / max of the kept readings and a small graph. */
export function SensorTile({ r, hist, sub }: { r: Reading; hist: number[]; sub?: string }) {
  const values = hist.length ? hist : [r.value];
  const min = Math.min(...values), max = Math.max(...values);
  // Graph range: percents on 0..100, the rest around their own values.
  const lo = r.unit === 'percent' ? 0 : min - Math.max(1, (max - min) * 0.2);
  const hi = r.unit === 'percent' ? 100 : max + Math.max(1, (max - min) * 0.2);
  const W = 100, H = 30;
  const pts = values.map((v, i) => [values.length < 2 ? W : (i / (values.length - 1)) * W, H - ((v - lo) / (hi - lo || 1)) * H]);
  const line = pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' ');
  const source = sub ?? sensorSource(r);
  return (
    <div className={'sensor-tile unit-' + r.unit}>
      <div className="sensor-head">
        <span className="sensor-name">{sensorName(r)}</span>
        <span className="sensor-dot" aria-hidden />
      </div>
      {source && <span className="sensor-source">{source}</span>}
      <span className="sensor-value">
        {formatValue(r)} <small>{UNIT[r.unit]}</small>
      </span>
      <span className="sensor-range">
        mín {formatValue(min, r.unit)} · máx {formatValue(max, r.unit)}
      </span>
      <svg className="sensor-graph" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden>
        <polygon points={`0,${H} ${line} ${W},${H}`} />
        <polyline points={line} />
      </svg>
    </div>
  );
}

/** "Painel": every sensor of the PC, grouped by part. */
export function Dashboard() {
  const { readings, history } = useSensors();
  const groups = GROUPS.map(([id, name]) => [name, readings.filter((r) => r.group === id)] as const).filter(([, list]) => list.length);
  return (
    <div className="dashboard">
      {groups.length === 0 && (
        <div className="empty-devices">
          <h2>Lendo os sensores…</h2>
          <p className="muted">No Linux eles vêm do hwmon; no Windows, por enquanto, só o uso do processador e da memória.</p>
        </div>
      )}
      {groups.map(([name, list]) => (
        <section key={name} className="sensor-group">
          <h2>{name}</h2>
          <div className="sensor-grid">
            {list.map((r) => (
              <SensorTile key={r.id} r={r} hist={history.get(r.id) ?? []} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

/** The few sensors shown on Início: CPU and GPU temperature and load, memory in use. */
export function HomeSensors() {
  const { readings, history } = useSensors();
  const t = cpuTemp(readings);
  const pick = [
    readings.find((r) => r.group === 'cpu' && r.unit === 'celsius' && r.value === t),
    readings.find((r) => r.group === 'cpu' && r.unit === 'percent'),
    readings.find((r) => r.group === 'gpu' && r.unit === 'celsius'),
    readings.find((r) => r.group === 'gpu' && r.unit === 'percent'),
    readings.find((r) => r.id === 'mem/used'),
    readings.find((r) => r.group === 'gpu' && r.unit === 'watt'),
  ].filter((r): r is Reading => !!r);
  if (!pick.length) return null;
  const groupName = (r: Reading) => GROUPS.find(([g]) => g === r.group)?.[1] ?? '';
  return (
    <div className="home-sensors">
      {pick.map((r) => (
        <SensorTile key={r.id} r={r} sub={groupName(r)} hist={history.get(r.id) ?? []} />
      ))}
    </div>
  );
}
