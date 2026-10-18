import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type RGB = [number, number, number];
export type Effect = 'static' | 'rainbow' | 'wave' | 'breathing' | 'reactive';

export type Lighting = {
  effect: Effect;
  color: RGB;
  color2: RGB;
  speed: number;
  brightness: number;
  painted: Record<string, RGB>; // LED slot -> colour
};

export type Status = {
  keyboard: { name: string; firmware: string } | null;
  headset: {
    name: string;
    battery: number | null;
    charging: boolean;
    firmware: string | null;
    /** Boom flipped up; null until the headset answers. */
    mic_muted: boolean | null;
  } | null;
  icue_running: boolean;
  win_lock: boolean;
  mouse: { name: string; firmware: string; dpi_stage: number } | null;
};

export type Eq = { enabled: boolean; preset: string; gains: number[] };
export type EqBackend = { kind: string; available: boolean; detail: string };

export type Action =
  | { kind: 'disabled' }
  | { kind: 'key'; key: string }
  | { kind: 'shortcut'; keys: string[] }
  | { kind: 'text'; text: string }
  | { kind: 'launch'; target: string }
  | { kind: 'media'; action: string }
  | { kind: 'mouse'; button: string }
  | { kind: 'profile'; target: string }
  | { kind: 'macro'; steps: { key: string; down: boolean; delay_ms: number }[] };
export type Bindings = Record<string, Action>;

export type ProfileInfo = { id: string; name: string; apps: string[] };
export type Profiles = { selected: string; active: string; auto_app: string | null; profiles: ProfileInfo[] };

export type AppSettings = {
  battery_tray: boolean;
  win_lock: boolean;
  lock_alt_tab: boolean;
  lock_alt_f4: boolean;
  lock_shift_tab: boolean;
  /** K70 report interval: 1, 2, 4 or 8 ms. */
  polling_ms: number;
  /** HS80 auto power-off in minutes (0 = never); null = headset's own setting. */
  headset_sleep_min: number | null;
  mic_sync: boolean;
  /** M65 DPI stage (0-based) restored on connect. */
  mouse_stage: number;
  mouse: MouseOptions;
};

export type DpiStage = { x: number; y: number; color: RGB };
export type MouseOptions = {
  stages: DpiStage[];
  sniper: DpiStage;
  /** 1, 2, 4 or 8 ms. */
  polling_ms: number;
  angle_snapping: boolean;
  button_optimization: boolean;
  lighting: { follow_keyboard: boolean; logo: Lighting; wheel: Lighting };
};

export const api = {
  appSettings: () => invoke<AppSettings>('get_app_settings'),
  /** Make a DPI stage active now (0-based). */
  setMouseStage: (stage: number) => invoke<void>('set_mouse_stage', { stage }),
  setAppSettings: (value: AppSettings) => invoke<void>('set_app_settings', { value }),
  /** Change some settings, starting from the saved ones (Win Lock may have been toggled on the keyboard). */
  patchAppSettings: async (patch: Partial<AppSettings>) => {
    const next = { ...(await invoke<AppSettings>('get_app_settings')), ...patch };
    await invoke<void>('set_app_settings', { value: next });
    return next;
  },
  onNavigate: (cb: (page: string) => void) => listen<string>('navigate', (e) => cb(e.payload)),
  profiles: () => invoke<Profiles>('get_profiles'),
  selectProfile: (id: string) => invoke<void>('select_profile', { id }),
  createProfile: (name: string) => invoke<string>('create_profile', { name }),
  updateProfile: (id: string, name: string, apps: string[]) => invoke<void>('update_profile', { id, name, apps }),
  deleteProfile: (id: string) => invoke<void>('delete_profile', { id }),
  foregroundApp: () => invoke<string | null>('foreground_app'),
  onProfile: (cb: (p: Profiles) => void) => listen<Profiles>('profile', (e) => cb(e.payload)),
  bindings: () => invoke<Bindings>('get_bindings'),
  setBindings: (bindings: Bindings) => invoke<void>('set_bindings', { bindings }),
  eq: () => invoke<Eq>('get_eq'),
  setEq: (value: Eq) => invoke<void>('set_eq', { value }),
  eqBackend: () => invoke<EqBackend>('eq_backend'),
  status: () => invoke<Status>('get_status'),
  lighting: () => invoke<Lighting>('get_lighting'),
  setLighting: (lighting: Lighting) => invoke<void>('set_lighting', { lighting }),
  /** Key name -> key index (what the keyboard reports; used for remapping). */
  keymap: () => invoke<Record<string, number | null>>('get_keymap'),
  /** Key name -> LED slot (painting, effects). */
  ledmap: () => invoke<Record<string, number | null>>('get_ledmap'),
  setGeometry: (geometry: Record<number, [number, number]>) => invoke<void>('set_geometry', { geometry }),
  autostart: () => invoke<boolean>('get_autostart'),
  setAutostart: (enabled: boolean) => invoke<void>('set_autostart', { enabled }),
  onStatus: (cb: (s: Status) => void) => listen<Status>('status', (e) => cb(e.payload)),
  /** Lighting changed outside the page (brightness key). */
  onLighting: (cb: (l: Lighting) => void) => listen<Lighting>('lighting', (e) => cb(e.payload)),
  /** EQ changed outside the page (headset wheel click). */
  onEq: (cb: (eq: Eq) => void) => listen<Eq>('eq', (e) => cb(e.payload)),
};

export const toHex = ([r, g, b]: RGB) => '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('');
export const fromHex = (h: string): RGB => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as RGB;
