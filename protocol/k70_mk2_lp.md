# K70 RGB MK.2 Low Profile — `1B1C:1B55`

Legacy Corsair keyboard protocol. ✅ = seen in iCUE 5.52 captures
(`captures/k70_icue_restart.pcap`, `k70_idle_icue.pcap`) and/or replayed by `opctl`.
Firmware on this unit: **3.24**, bootloader 3.03.
Database family `nxp` (`devices/corsair.toml`): other keyboards marked `nxp` there are
expected to speak this protocol but stay read-only until verified.

## Transport ✅
| Path | Collection | Endpoint | Use |
|---|---|---|---|
| Commands | MI_01 `FFC2:0004`, no report IDs, 64 B | interrupt OUT `0x02` | every host→device command |
| Replies | MI_00 `FFC2:0003`, report ID `0x0E`, 63 B | interrupt IN `0x81` | replies to `0E xx` reads |
| Key state | MI_00 report `0x01` (21 B NKRO bitmap), `0x03` (Corsair key bitmap) | IN `0x81` | key presses |

Replies come back on a **different interface** than the request was sent on.
Linux: both hidraw nodes need access (`linux/70-openperipheral.rules` covers all of the device).

**Pitfall:** if a host program dies while the keyboard is in software mode, the
OUT endpoint can stall and every write fails with `ERROR_INVALID_FUNCTION`
until the keyboard is replugged / PC rebooted. Always send `07 04 01` on exit.
Seen on Linux too (2026-10-17, app killed with SIGTERM): the keyboard stopped
sending any key report, OS or host, while its lighting kept running; replug fixed it.
The app now quits cleanly on SIGTERM/SIGINT/SIGHUP.

## Lifecycle seen from iCUE ✅
| When | Bytes | Meaning |
|---|---|---|
| iCUE start | `0E 01` → reply `0E 01 …` | firmware info |
| | `0E 00` → reply `0E 00 …` | same layout, byte 2 differs (device state?) |
| | `07 04 02` | special function = **software mode** |
| | `07 05 02 00 03` | lighting control = software, `03` = keyboard |
| | `0E 17 …` / `07 17 …` / `FF 01..05 …` | onboard file system (`"PROFILE.I"`): see below |
| | `0E 48` then `07 40 1E 00 [key, mode]…` ×4 | per-key input mode (see Macros) |
| every ~14 ms | `7F 01 3C 00` + 60 B, `7F 02 3C 00` + 60 B, `7F 03 30 00` + 48 B, `07 28 ch 03 fin` | one colour plane; ch 1=R 2=G 3=B, fin `01`, last plane `02` |
| iCUE quit | `07 04 01` | back to **hardware mode** → onboard rainbow returns |

### Firmware reply (`0E 01`) ✅
`0e 01 00 00 01 01 00 01 | 24 03 | 03 03 | 1c 1b | 55 1b | 01 01 00 01 c0 ff 40 02 …`
| off | len | meaning |
|---|---|---|
| 8 | 2 LE | firmware `0x0324` |
| 10 | 2 LE | bootloader `0x0303` |
| 12 | 2 LE | VID `1B1C` |
| 14 | 2 LE | PID `1B55` |

### Colour frame ✅
168 slots per plane = 14 rows × 12; each row has unused slots that iCUE leaves `00`.
Slot → key: **LED slot = (key % 12) × 12 + key / 12**, the key index matrix transposed on a
12-wide grid (F1 = key 1 → LED 12, F8 = key 8 → LED 96, Win Lock = key 96 → LED 8, `1` = key 13
→ LED 13). Rule read off OpenLinkHub's `database/keyboard/k70mk2.json` (`keyData[1]` vs
`packetIndex`); ✅ 2026-10-17 with `opctl k70 echo` (F1–F12, lock) and by painting F8, 5, G and
Profile in the app. `layout/k70_mk2_lp_BR.json` holds key indices; `k70::led_for_key` converts.
Logo LEDs (no key): slots 47 and 59 (OpenLinkHub, not checked).

## Key modes / remapping ✅ (`captures/k70_remap.pcap`)
`07 40 count 00` followed by `count` pairs `[key index, mode]`, sent in 4 packets
(30+30+30+26 = 116 keys). Key index = the report `03` bit, **not** the LED slot (see Colour frame).

| Mode | Meaning | Evidence |
|---|---|---|
| `C0` | normal: HID to the OS **and** report `0x03` to the host | default after iCUE start; typing kept working in `opctl k70 echo` |
| `40` | **host only**: no HID report, only report `0x03` | iCUE "F12 → B": `48 C0` became `48 40`; F12 presses then produced only report `03` |
| `C1` | WinLock (`0x47`) only — firmware-handled | sent by iCUE every time |

So bit 7 (`0x80`) = send to the OS, bit 6 (`0x40`) = report to the host.
Remapping = set the key to `40`, watch report `03`, and inject the replacement from the host
(iCUE uses its VHID kernel driver; we use the OS input API: SendInput / uinput).

## Top-left keys and media keys ✅ 2026-10-17
In software mode these reach the host only (report `03`), so the app implements them.
The lock LED is slot 8 (= key 96 transposed, see Colour frame); Profile is LED 125, Brightness 137.

| Key | Report `03` bit | OS sees | App action |
|---|---|---|---|
| Profile | 70 | — | next profile |
| Brightness | 71 (`0x47`, mode `C1` = ACT_LIGHT) | — | 100 → 66 → 33 → 0 → 100 % |
| Win Lock | 96 | — | firmware lock `07 48` (below); lock LED 8 lit white |
| Mute | 97 | consumer `E2` | (OS) |
| Stop / Prev / Play / Next | 98–101 | `B7` / `B6` / `CD` / `B5` | (OS) |
| Volume wheel | 130 / 131 | `E9` / `EA` | (OS) |

Media keys send the same consumer reports (report `02`) in hardware and software mode.
Mode bits follow ckb-next's naming: `0x80` IN_HID, `0x40` IN_CORSAIR, low bits = firmware
action (1 = light, 8 = lock).

## Firmware lock and polling rate ✅ 2026-10-17
Commands from [OpenLinkHub](https://github.com/jurkovic-nikola/OpenLinkHub)
(`src/devices/k70mk2/k70mk2.go`, GPL-3.0), checked on this keyboard (fw 3.24).

| Bytes | Meaning | Checked |
|---|---|---|
| `07 48 xx` | lock: `xx` = `A0` + 1 Windows keys + 2 Alt+Tab + 4 Alt+F4 + 8 Shift+Tab; `00` = off | ✅ all four blocked (hardware and software mode) |
| `07 40 01 00 60 C0` | sent after `07 48` (as OpenLinkHub does): Win Lock key back to `C0` | ✅ key 96 still reaches the host in software mode |
| `0E 48` | reply byte 2 = lock byte; reads `81` for `A1` while active, `01` once inactive | ✅ |
| `07 0A 00 00 ms` | polling interval, `ms` = 1/2/4/8 (1000/500/250/125 Hz) | ✅ keyboard re-enumerates; EP `0x81` bInterval 1 → 8 |
| `0E 0A` | — | no reply: read the rate from sysfs (`<usb>:1.0/ep_81/interval`) |

- Leaving software mode (`07 04 01`) **clears** the lock (`0E 48` → `01`, Windows key
  works again). Sending `07 48` again while in hardware mode makes it hold with no host,
  so the app sends it after `07 04 01` on exit. ✅
- The lock survives the polling re-enumeration. ✅
- In hardware mode the Win Lock key does nothing with mode `C0` or `C8`: the firmware
  does not toggle the lock itself. Only the host changes it. ✅
- Changing the polling rate drops every handle (new USB descriptors): the app sends it
  only in hardware mode (on connect, before taking the keyboard, or after `07 04 01`)
  and takes the keyboard again on the next scan.

## Onboard storage — **do not replay**
`0E 17 xx` / `07 17 xx` with ASCII names (`PROFILE.I`) and `FF nn len` data packets
write the hardware profile to flash. Writing flash can corrupt the onboard profile;
only implement after a full read-back/restore path exists.
