# Changelog

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
