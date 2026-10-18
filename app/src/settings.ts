import { useEffect, useState } from 'react';
import { api, type AppSettings } from './api';

export const POLLING = [
  { ms: 1, label: '1000 Hz' },
  { ms: 2, label: '500 Hz' },
  { ms: 4, label: '250 Hz' },
  { ms: 8, label: '125 Hz' },
];

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  useEffect(() => {
    api.appSettings().then(setSettings);
  }, []);
  const patch = async (p: Partial<AppSettings>) => setSettings(await api.patchAppSettings(p));
  return [settings, patch] as const;
}
