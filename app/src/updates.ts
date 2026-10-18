import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { useCallback, useEffect, useState } from 'react';

export const RELEASES_URL = 'https://github.com/GordoLonha/OpenPeripheral/releases/latest';

/** "windows" and "appimage" update themselves; "package" (.rpm/.deb) goes through the package manager. */
export type InstallKind = 'windows' | 'appimage' | 'package';

export type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'none' }
  | { kind: 'available'; update: Update }
  | { kind: 'downloading'; percent: number | null }
  | { kind: 'error'; message: string };

export const installKind = () => invoke<InstallKind>('install_kind');
export const openUrl = (url: string) => invoke<void>('open_url', { url });

/** Looks for a newer release on GitHub (latest.json) and installs it on request.
 *  Checks quietly once, `autoCheckMs` after mounting. */
export function useUpdater(autoCheckMs = 5000) {
  const [state, setState] = useState<UpdateState>({ kind: 'idle' });

  const find = useCallback(async (quiet = false) => {
    if (!quiet) setState({ kind: 'checking' });
    try {
      const update = await check();
      setState(update ? { kind: 'available', update } : { kind: quiet ? 'idle' : 'none' });
    } catch (e) {
      setState(quiet ? { kind: 'idle' } : { kind: 'error', message: String(e) });
    }
  }, []);

  useEffect(() => {
    const t = window.setTimeout(() => find(true), autoCheckMs);
    return () => window.clearTimeout(t);
  }, [find, autoCheckMs]);

  const install = async (update: Update) => {
    let total = 0;
    let done = 0;
    setState({ kind: 'downloading', percent: null });
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === 'Started') total = ev.data.contentLength ?? 0;
        if (ev.event === 'Progress') {
          done += ev.data.chunkLength;
          setState({ kind: 'downloading', percent: total ? Math.round((done / total) * 100) : null });
        }
      });
      await relaunch();
    } catch (e) {
      setState({ kind: 'error', message: String(e) });
    }
  };

  return { state, find, install };
}
