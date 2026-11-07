// PC sensors: the latest readings and a short history per sensor, shared by every page.
import { useEffect, useReducer } from 'react';
import { api, type Reading, type SensorGroup, type SensorUnit } from './api';
import { setPreviewTemp } from './preview';

/** Readings kept per sensor for the graphs (one every 1.5 s: about 1.5 minutes). */
const KEEP = 60;

let latest: Reading[] = [];
const history = new Map<string, number[]>();
const listeners = new Set<() => void>();
let started = false;

/** Same choice as sensors::cpu_temp: the CPU's control sensor, else its hottest one. */
export function cpuTemp(readings: Reading[]): number | null {
  const cpu = readings.filter((r) => r.group === 'cpu' && r.unit === 'celsius');
  const main = cpu.find((r) => ['Tctl', 'Tdie', 'Package id 0', 'CPU', 'cpu'].some((m) => r.name.startsWith(m)));
  if (main) return main.value;
  return cpu.length ? Math.max(...cpu.map((r) => r.value)) : null;
}

function take(readings: Reading[]) {
  latest = readings;
  for (const r of readings) {
    const h = history.get(r.id) ?? [];
    h.push(r.value);
    if (h.length > KEEP) h.shift();
    history.set(r.id, h);
  }
  const t = cpuTemp(readings);
  if (t != null) setPreviewTemp(t);
  listeners.forEach((f) => f());
}

/** Starts listening once (the app calls it at start, so the effect preview has the temperature). */
export function startSensors() {
  if (started) return;
  started = true;
  api.sensors().then(take, () => {});
  api.onSensors(take);
}

export function useSensors() {
  const [, redraw] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    startSensors();
    listeners.add(redraw);
    return () => {
      listeners.delete(redraw);
    };
  }, []);
  return { readings: latest, history };
}

export const GROUPS: [SensorGroup, string][] = [
  ['cpu', 'Processador'],
  ['gpu', 'Placa de vídeo'],
  ['memory', 'Memória'],
  ['storage', 'Armazenamento'],
  ['board', 'Placa-mãe'],
  ['other', 'Outros'],
];

/** Friendlier names for the labels the drivers use. */
const LABELS: Record<string, string> = {
  Tctl: 'Temperatura',
  Tdie: 'Temperatura',
  'Package id 0': 'Temperatura',
  edge: 'Temperatura',
  junction: 'Ponto mais quente',
  mem: 'Temp. da memória',
  PPT: 'Consumo',
  Composite: 'Temperatura',
  'Temp 1': 'Temperatura',
  'Ventoinha 1': 'Ventoinha',
};

export function sensorName(r: Reading): string {
  const ccd = /^Tccd(\d+)$/.exec(r.name);
  if (ccd) return `CCD ${ccd[1]}`;
  return LABELS[r.name] ?? r.name;
}

/** The chip names users won't know are left out of the subtitle, or named plainly. */
export function sensorSource(r: Reading): string | null {
  if (['k10temp', 'zenpower', 'coretemp', 'amdgpu', 'Processador', 'Memória RAM'].includes(r.source)) return null;
  if (/^(mt7\d|iwlwifi|ath\d|rtw)/.test(r.source)) return 'Wi-Fi';
  return r.source;
}

export const UNIT: Record<SensorUnit, string> = { celsius: '°C', rpm: 'RPM', percent: '%', watt: 'W' };

export function formatValue(r: Reading | number, unit?: SensorUnit): string {
  const v = typeof r === 'number' ? r : r.value;
  const u = typeof r === 'number' ? unit : r.unit;
  return u === 'rpm' || u === 'percent' || u === 'watt' ? v.toFixed(0) : v.toFixed(1);
}
