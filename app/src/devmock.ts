// Browser preview for UI work: `npm run dev` outside Tauri answers every backend
// command with sample data, so pages can be viewed and screenshotted without the
// devices. Only loaded in dev builds when the Tauri runtime is absent.
import { emit } from '@tauri-apps/api/event';
import { mockIPC } from '@tauri-apps/api/mocks';
import keymapBR from '../../layout/k70_mk2_lp_BR.json';
import type { AppSettings, Bindings, Eq, Lighting, Profiles, Status } from './api';

const keymap = keymapBR as Record<string, number | null>;
// Same rule as op_core::k70::led_for_key: the key matrix transposed on 12 columns.
const ledmap = Object.fromEntries(
  Object.entries(keymap).map(([name, k]) => [name, k == null || k >= 144 ? null : (k % 12) * 12 + Math.floor(k / 12)]),
);

const lighting: Lighting = { effect: 'wave', color: [0, 190, 255], color2: [40, 0, 120], speed: 1, brightness: 100, painted: {} };
const zone = (color: [number, number, number]): Lighting => ({ ...lighting, effect: 'static', color });

let settings: AppSettings = {
  battery_tray: true,
  win_lock: false,
  lock_alt_tab: false,
  lock_alt_f4: false,
  lock_shift_tab: false,
  polling_ms: 1,
  headset_sleep_min: 15,
  mic_sync: false,
  mouse_stage: 2,
  mouse: {
    stages: [
      { x: 800, y: 800, color: [255, 0, 0] },
      { x: 1500, y: 1500, color: [255, 255, 255] },
      { x: 3000, y: 3000, color: [0, 255, 0] },
      { x: 6000, y: 6000, color: [128, 0, 128] },
      { x: 9000, y: 9000, color: [0, 191, 255] },
    ],
    sniper: { x: 400, y: 400, color: [255, 255, 0] },
    polling_ms: 1,
    angle_snapping: false,
    button_optimization: true,
    lighting: { follow_keyboard: true, logo: zone([0, 190, 255]), wheel: zone([255, 60, 0]) },
  },
};

const status: Status = {
  keyboard: { name: 'K70 RGB MK.2 Low Profile', firmware: '3.24' },
  headset: { name: 'HS80 RGB Wireless', battery: 64, charging: false, firmware: '5.13', mic_muted: false },
  icue_running: false,
  win_lock: false,
  mouse: { name: 'M65 RGB Elite', firmware: '3.41', dpi_stage: 2 },
};

const profiles: Profiles = {
  selected: 'default',
  active: 'default',
  auto_app: null,
  profiles: [
    { id: 'default', name: 'Padrão', apps: [] },
    { id: 'fps', name: 'FPS', apps: ['cs2'] },
  ],
};

let bindings: Bindings = {
  F12: { kind: 'media', action: 'playpause' },
  CapsLock: { kind: 'disabled' },
};
let mouseBindings: Bindings = {
  back: {
    kind: 'macro',
    steps: [
      { key: 'a', down: true, delay_ms: 0 },
      { key: 'a', down: false, delay_ms: 77 },
      { key: 'Mouse:left', down: true, delay_ms: 120 },
      { key: 'Mouse:left', down: false, delay_ms: 60 },
    ],
  },
};
let eq: Eq = { enabled: true, preset: 'fps', gains: [-3, -2, -1, 0, 0, 1.5, 3.5, 4.5, 3, 0] };

// ?devices=mouse (or keyboard,headset,...) shows only those, like a PC with just the mouse.
const only = new URLSearchParams(location.search).get('devices');
if (only) {
  const want = only.split(',');
  if (!want.includes('keyboard')) status.keyboard = null;
  if (!want.includes('headset')) status.headset = null;
  if (!want.includes('mouse')) status.mouse = null;
}

// Like the backend: every profile change is announced with a 'profile' event.
function changed() {
  void emit('profile', { ...profiles, profiles: [...profiles.profiles] });
  return null;
}

export function installDevMock() {
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case 'get_status': return status;
        case 'get_lighting': return lighting;
        case 'get_keymap': return keymap;
        case 'get_ledmap': return ledmap;
        case 'get_profiles': return profiles;
        case 'get_bindings': return bindings;
        case 'set_bindings': bindings = a.bindings as Bindings; return null;
        case 'get_mouse_bindings': return mouseBindings;
        case 'set_mouse_bindings': mouseBindings = a.bindings as Bindings; return null;
        case 'get_app_settings': return settings;
        case 'set_app_settings': settings = a.value as AppSettings; return null;
        case 'set_mouse_stage': status.mouse!.dpi_stage = a.stage as number; void emit('status', status); return null;
        case 'get_eq': return eq;
        case 'set_eq': eq = a.value as Eq; return null;
        case 'eq_backend': return { kind: 'pipewire', available: true, detail: 'alsa_output.usb-Corsair_HS80' };
        case 'get_autostart': return true;
        case 'foreground_app': return 'cs2';
        case 'select_profile':
          profiles.selected = profiles.active = a.id as string;
          profiles.auto_app = null;
          return changed();
        case 'create_profile': {
          const id = (a.name as string).toLowerCase().replace(/[^a-z0-9]+/g, '-') || 'perfil';
          profiles.profiles = [...profiles.profiles, { id, name: a.name as string, apps: [] }];
          void changed();
          return id;
        }
        case 'update_profile':
          profiles.profiles = profiles.profiles.map((p) => (p.id === a.id ? { ...p, name: a.name as string, apps: a.apps as string[] } : p));
          return changed();
        case 'delete_profile':
          if (profiles.profiles.length > 1) profiles.profiles = profiles.profiles.filter((p) => p.id !== a.id);
          if (!profiles.profiles.some((p) => p.id === profiles.selected)) profiles.selected = profiles.profiles[0].id;
          if (!profiles.profiles.some((p) => p.id === profiles.active)) profiles.active = profiles.selected;
          return changed();
        case 'install_kind': return 'package';
        // ?update=1 pretends a newer release is out (Settings → Atualizações).
        case 'plugin:updater|check':
          return new URLSearchParams(location.search).has('update')
            ? { rid: 1, currentVersion: __APP_VERSION__, version: '0.4.1', date: null, body: '- Exemplo de novidade\n- Outra novidade', rawJson: {} }
            : null;
        default: return null; // set_lighting, set_geometry, select_profile, ...
      }
    },
    { shouldMockEvents: true },
  );
}
