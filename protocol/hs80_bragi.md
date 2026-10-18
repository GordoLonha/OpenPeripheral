# HS80 RGB Wireless — receiver `1B1C:0A6B`

Corsair "Bragi" protocol through the wireless receiver. The audio path is a
standard USB Audio Class device (works natively on Windows and Linux); this
spec covers only the vendor control channel.
Database family `bragi` (`devices/corsair.toml`); `0A6B`, `0A69` and `0A6A` are linked
there with `pairs`.

## Transport
- USB interface `MI_03`, several HID collections (`COL01..COL07`); the vendor
  ones (usage page ≥ `0xFF00`) carry control traffic, **with report IDs**.
- Report ID and lengths come from the descriptor at runtime (`opctl probe`).

## Confirmed on hardware (2026-10-13, receiver fw unknown, iCUE closed)
Collection `FF42:0001`: output report ID `0x02` (63 B), input report ID `0x01` (63 B).
Collection `FF42:0002`: input-only report `0x03` (likely unsolicited events).

| Request (out, after `02`) | Reply (in, after `01`) | Meaning |
|---|---|---|
| `09 02 0F 00` | `01 02 00 34 03` | headset battery = 0x0334 = 820 → **82.0 %** ✅ |
| `08 02 0F 00` | `00 02 05 …` | receiver has no battery: status `05` = unsupported ✅ |
| `08 12` | `00 12 00 …` | ping OK ✅ |

So the **reply** header is `[index, command, status]`, where index = target & 1
(`00` receiver, `01` headset) — not the raw target byte.
iCUE polls every ~0.5 s; seen reply `01 06 00 04 00 00 00 1b 69 0a 10 "<serial>…"`
(command `06` from the headset — request not yet captured).

## Notifications (report `0x03`) ✅ 2026-10-17
Unsolicited input reports, `[03, index, kind, …]`. On Linux they arrive on the same
hidraw node as replies, so request/reply code must only accept report `0x01`.

| Bytes | Meaning | Mode |
|---|---|---|
| `03 01 02 01` / `03 01 02 00` | volume-wheel **press / release** | software only (nothing in hardware mode) |
| `03 01 01 0f 00 lo hi` | battery level changed (tenths of %) | seen with iCUE; ✅ software mode (`1c 02` = 54.0 %) |
| `03 01 01 8e/a6 00 mute` | mic mute changed (OpenLinkHub) | VERIFY — this unit's mic is faulty, nothing seen |
| `0e 01` `0e 02` … `0e 00` | volume wheel turned (report `0x0E`) | both |

Corsair's quick start guide: a short press (0.1–0.5 s) on the wheel toggles EQ; iCUE
cycles presets. Mic flip-to-mute produced no notification here.

## Settings from OpenLinkHub (2026-10-17)
From [OpenLinkHub](https://github.com/jurkovic-nikola/OpenLinkHub)
(`src/devices/hs80rgbW/hs80rgbW.go`, `headsetdongle/headsetdongle.go`, GPL-3.0),
checked on this headset (fw 5.8.48) through the receiver (target `09`).

| Request | Reply | Meaning | Status |
|---|---|---|---|
| `09 02 13 00` | `01 02 00 05 08 30` | firmware 5.8.48 | ✅ |
| `09 02 0D 00` | `01 02 00 00` | sleep timer enabled (0/1); was 0 out of the box here | ✅ read |
| `09 02 0E 00` | `01 02 00 c0 27 09 00` | sleep time, u32 ms (600000 = 10 min) | ✅ read |
| `09 01 0D 00 01` | `01 01 00` | enable sleep timer | ✅ |
| `09 01 0E 00 <u32 ms>` | `01 01 00` | sleep time | ✅ read back |
| `09 02 A6 00` | `01 02 00 00` | mic muted (1) | VERIFY — always 0 here (faulty mic) |

- **SET only works in software mode**: in hardware mode both writes answered status `09`.
  Values stay after going back to hardware mode.
- The power-off itself was not observed: with a 1-min timer and the PipeWire sink idle
  (TeamSpeak stream moved away) the headset stayed on for 5 min. Unknown whether the
  receiver still counts as activity. TODO.
- OpenLinkHub paints the mic LED (planar LED 2: bytes 2, 5, 8) red while muted.

## LEDs ✅ 2026-10-17
`WRITE_DATA` to the lighting endpoint, 9 bytes, **planar** `RRR GGG BBB` for 3 LEDs.
LED 1 is the status LED next to the wheel (pattern `ff0000 00ff00 0000ff` lit it green;
iCUE blinks it green `80`/`00` while charging). In software mode the headset keeps the
last frame, so the host must draw the status LED (battery colour, blink while charging)
and put both receiver and headset back in hardware mode (`03` = `01`) on exit.

## On the USB cable (PID `0A69`) ✅ 2026-10-13
The headset enumerates as its own composite device (USB audio + HID), same serial
as the wireless headset. Same `FF42:0001` / `FF42:0002` collections and report IDs.
It is now the device itself, so properties are read with **target `08`**:

| Request | Reply | Meaning |
|---|---|---|
| `08 02 0F 00` | `00 02 00 DA 02` | battery 0x02DA = 73.0 % |
| `08 02 10 00` | `00 02 00 01` | status 1 = charging (cable powers it while playing) |
| `09 02 0F 00` | `01 02 06 …` | status `06`: no sub-device on the cable |

PID `0A6A` (same serial, HID only, no audio) = cable plugged while the headset is off.

Battery status (`0x10`): `1` charging, `2` on battery (both confirmed), `3` full (assumed).

**USB bandwidth:** on this PC the wired headset failed with "USB controller resources
exceeded" on the chipset controller that already hosts 3 USB audio devices; a port on
the CPU controller worked. Not a protocol issue.

## Request layout (after report ID)
| off | meaning |
|---|---|
| 0 | target: `08` receiver, `09` paired headset |
| 1 | command |
| 2 | property / endpoint |
| 3.. | arguments (little endian) |

Reply: `[target, command, status(00 = ok), value…]`.

## Commands
| Command | Byte | Status |
|---|---|---|
| SET_PROPERTY | `01` | VERIFY — not used until captured |
| GET_PROPERTY | `02` | VERIFY |
| CLOSE_ENDPOINT | `05` | VERIFY |
| WRITE_DATA | `06` | VERIFY |
| READ_DATA | `08` | VERIFY |
| OPEN_ENDPOINT | `0D` | VERIFY |
| PING / keep-alive | `12` | VERIFY |

## Properties
| Property | Id | Notes | Status |
|---|---|---|---|
| Mode (hardware/software) | `03` | `01` hw, `02` sw (read back with `08 02 03`) | ✅ |
| Brightness | `02` | 0..1000 | VERIFY |
| Battery level | `0F` | 0..1000 → % ×10 | VERIFY |
| Battery status | `10` | charging / discharging / full | VERIFY |
| Sidetone level | ? | capture iCUE slider min→max | TODO |
| Mic mute | `A6` (events `8E`/`A6`) | 1 = muted (OpenLinkHub) | VERIFY |
| EQ preset / bands | ? | possibly applied host-side by the Corsair APO (`CorsairGamingAPO`) rather than in the headset — capture to find out | TODO |
| Sleep timer | `0D` / `0E` | enable / u32 ms, see above | ✅ write+read, power-off TODO |
| Firmware | `13` | 3 bytes major.minor.patch | ✅ |

> EQ note: iCUE installs an *Audio Processing Object*. If EQ is host-side DSP,
> our version applies EQ with an OS audio filter (Windows APO / PipeWire
> filter-chain) instead of a device command.
