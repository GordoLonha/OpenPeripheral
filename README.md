# OpenPeripheral

Open, lightweight replacement for Corsair iCUE on **Linux** and **Windows**: lighting,
key remapping and macros, profiles, a headset equalizer and battery in the tray.
Userspace HID drivers in Rust (no kernel driver), desktop app in Tauri 2 + React.
The app's interface is in Brazilian Portuguese.

> Not affiliated with Corsair. See [CREDITS.md](CREDITS.md).

| Device | USB id | What works |
|---|---|---|
| K70 RGB MK.2 Low Profile | `1B1C:1B55` | per-key lighting and effects, ABNT2 layout, key remap and macros, profiles, Profile / Brightness / Win Lock keys, firmware Win Lock (also blocks Alt+Tab, Alt+F4, Shift+Tab), polling rate |
| HS80 RGB Wireless | `1B1C:0A6B` receiver, `0A69` cable | battery and charging in the tray, 10-band EQ (PipeWire on Linux, Equalizer APO on Windows), volume-wheel click cycles EQ presets, auto power-off, mic mute, firmware |
| M65 RGB Elite | `1B1C:1B5A` | lighting following the keyboard, DPI stage indicator, firmware |

About 200 other Corsair devices are listed in the [device database](#device-database) as
untested (read-only) until someone confirms them.

## Status
- **Linux**: tested on Fedora 44, KDE Plasma (Wayland), with all three devices.
- **Windows: beta.** It builds and was used on Windows up to 0.2.0, but this version has
  not been tested on Windows hardware yet. Reports welcome.

## Install
Download from the Releases page:
- **Fedora / openSUSE**: `.rpm`; **Ubuntu / Debian**: `.deb`. Both install the udev rule
  that gives your user access to the devices. Replug the devices (or reboot) once.
- **Other Linux**: AppImage; install the udev rule by hand (see [Linux](#linux-fedora)).
- **Windows**: `*-setup.exe`. For the headset EQ, install
  [Equalizer APO](https://sourceforge.net/projects/equalizerapo/) and select the HS80.

Close iCUE first: both programs fight over the devices.

## License
GPL-3.0-or-later, see [LICENSE](LICENSE).

## Device database
`devices/corsair.toml` lists every Corsair USB PID we know (~200): name, kind, protocol
family (`nxp` = K70 MK.2 legacy, `bragi` = HS80 and newer), the HID control collections,
LED count, layout, features, where the PID is known from, and a status:
`verified` (tested here), `community` (reported working) or `untested`. **Untested
devices are read-only**: they are listed but OpenPeripheral never writes to them.
A new device of a family we already drive works by adding or promoting its entry
(family + `control` + status). The file is embedded at build time; entries in
`~/.config/dev.openperipheral.app/devices.toml` (Windows: `%APPDATA%\dev.openperipheral.app\`)
are merged on top, same PID = replaced.
```bash
cargo run -p opctl -- db                      # whole database
cargo run -p opctl -- db --status verified
cargo run -p opctl -- udev-rules > linux/70-openperipheral.rules   # after editing the DB
```
The untested entries were seeded from OpenLinkHub, OpenRGB and liquidctl device lists
(facts only: PID, name, type) by `tools/seed_devices.py`; not from ckb-next (GPL-2.0 only).

## Layout
- `crates/op-core` — device database/registry, HID descriptor parser, drivers (`k70`, `hs80`, `m65`)
- `devices/corsair.toml` — device database
- `crates/opctl` — CLI for testing on hardware
- `protocol/` — protocol specs; every command has a status (VERIFY / ✅)
- `linux/70-openperipheral.rules` — udev permissions, generated from the database

## Try it (CLI)
```bash
cargo run -p opctl -- list
cargo run -p opctl -- probe
cargo run -p opctl -- hs80 battery
cargo run -p opctl -- listen 5
cargo run -p opctl -- raw 0a6b ff42 02 08 12 --len 64
```

## Linux (Fedora)
```bash
sudo dnf install rust cargo nodejs npm gcc-c++ webkit2gtk4.1-devel openssl-devel \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel systemd-devel rpm-build
sudo cp linux/70-openperipheral.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
cd app && npm ci
npm run tauri dev     # development
# Release builds also sign the update bundles (in-app updater): point the build at
# the private key, or use `npm run tauri build -- --no-bundle` to skip packaging.
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/openperipheral-updater.key)"
npm run tauri build   # .deb, .rpm (both install the udev rule) and AppImage
```
The udev rule must sort before `73-seat-late.rules` (hence `70-`), or the `uaccess`
tag is never turned into an ACL. It also opens `/dev/uinput`.

- Remapped keys and macros are injected through a uinput virtual device, so they work
  on Wayland; characters are mapped for the ABNT2 (br) layout.
- EQ: a PipeWire filter-chain sink in front of the HS80 (our own `pipewire -c` child),
  made the default output while enabled; gains change live via `pw-cli set-param`.
- Not yet on Linux: auto-switching profiles by the focused app (KDE Wayland needs a
  KWin script).
