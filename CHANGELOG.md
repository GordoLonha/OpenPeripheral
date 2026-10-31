# Changelog

## 0.4.2 — 2026-10-31

- **Profiles switch by themselves on Linux**: a profile linked to a program now turns on
  while that program is in front, like on Windows. Works on X11 and, on Wayland, for
  XWayland windows, which covers games under Proton (link them by the `.exe` name, e.g.
  `cs2.exe`) and most native games (`cs2`). "Detectar programa" works there too.
- **Profiles page** in the new look, and deleting a profile asks first.
- **"Abrir app" key action on Linux**: Calculadora, Editor de texto, Gerenciador de
  arquivos and Configurações do sistema open the desktop's own programs; a typed program
  runs with its arguments (`steam steam://rungameid/730`). Before, only links and files
  opened.

### Fixes
- A remapped key or mouse button held during a profile switch (or when the device was
  unplugged) could stay pressed in the system, e.g. Ctrl stuck after alt-tabbing out of
  a game.
- A damaged `profiles.json` (power cut while saving) wiped every profile on the next
  start. Settings files are now saved safely, and a file that cannot be read is kept as
  `profiles.json.broken-…` instead of being overwritten.
- The keyboard and mouse no longer drop off and come back twice at login when a polling
  rate other than 1000 Hz is saved; Win Lock and the DPI stage are set from the start.
- The Profile key works while a game with its own profile is in front; a profile chosen
  by hand stays until you leave the game.
- A key bound to a deleted profile no longer leaves the app stuck on a game's profile.
- Mouse buttons set to keep their own function (macro with "keep original output") no
  longer confuse the DPI stage; the same for the K70's Profile, Brightness and Win Lock keys.
- Windows: the mouse is handed to iCUE when iCUE starts, like the keyboard.

## 0.4.1 — 2026-10-18

- **Updates from the app**: Configurações → Atualizações looks for a new release on GitHub
  (also quietly a few seconds after start, with a note at the top when there is one).
  Windows and the AppImage update themselves and restart; .rpm/.deb open the download
  page. Updates are signed; the app only accepts ones made with the project's key.
- From this version on, updates arrive inside the app; 0.4.0 needs this one installed by hand.

## 0.4.0 — 2026-10-17

- **New look**: sidebar with icons, Keyboard and Mouse pages in tabs, settings grouped in
  lists, consistent colours, spacing and controls.
- **Only the devices you have**: the menu and the home page show the connected devices,
  like iCUE; with none, a short note says what to do.
- **M65 RGB Elite page**: five DPI stages plus sniper, each with its own DPI (X/Y can
  differ) and indicator colour, iCUE's defaults; click a stage to use it; polling rate;
  angle snapping; button response optimization; lighting per zone (logo, wheel) or
  following the keyboard.
- Sniper button slows the pointer and lights the DPI LED in the sniper colour.
- **M65 buttons**: any of the 8 buttons can be remapped per profile (key, shortcut, mouse,
  text, media, app, macro, profile switch, disabled); back to normal when the app closes.
- **Macros like iCUE**, for keys and mouse buttons: record keys, mouse clicks and the
  wheel; edit each delay, reorder and delete steps; trigger on press, on release, while
  held or toggle; repeat once, N times or continuously; run uninterrupted; keep the key's
  own function; play a sound when it starts.

## 0.3.0 — 2026-10-17

First public version, for Linux and Windows (beta).

### Linux (new)
- Runs on Linux: tested on Fedora 44, KDE Plasma (Wayland). `.deb`, `.rpm` and AppImage;
  the packages install the udev rule for device access.
- Key remaps and macros through a uinput virtual keyboard (work on Wayland), ABNT2 layout.
- Headset EQ through a PipeWire filter in front of the HS80, changed live.

### Devices
- **Device database** (`devices/corsair.toml`): about 200 Corsair devices, each with
  protocol family and status; untested devices are never written to. udev rules are
  generated from it.
- **K70 RGB MK.2 LP**: Profile, Brightness and Win Lock keys; Win Lock done by the
  firmware (works with the app closed), optional Alt+Tab / Alt+F4 / Shift+Tab lock;
  polling rate; LEDs follow the real key matrix (painting and effects hit the right keys).
- **HS80 RGB Wireless**: volume-wheel click cycles EQ presets; status LED shows the
  battery colour; auto power-off timer; mic mute; firmware version; battery updates
  from the headset's own notifications.
- **M65 RGB Elite** (new): lighting follows the keyboard, DPI stage indicator, firmware.

### Fixes
- Quitting on logout, shutdown or `kill` returns the devices to hardware mode (before,
  the K70 could stop typing until replugged).
- Battery icon visible in Linux trays (AppIndicator needs a menu).

### Windows (beta)
Builds and packages, but this version was not tested on Windows hardware.

## 0.2.0
Windows only: lighting, ABNT2 key map, remaps and macros, profiles with auto-switch,
HS80 battery in the tray and EQ via Equalizer APO.
