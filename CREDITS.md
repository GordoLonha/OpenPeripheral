# Credits

OpenPeripheral is licensed under the GNU General Public License, version 3 or (at your
option) any later version. See [LICENSE](LICENSE).

It is not affiliated with, endorsed by or sponsored by Corsair. "Corsair", "iCUE" and
product names are trademarks of their owners and are used only to say which devices
work with this software.

## Code and protocol knowledge from other projects

| Project | License | What we use |
|---|---|---|
| [OpenLinkHub](https://github.com/jurkovic-nikola/OpenLinkHub) by Nikola Jurković | GPL-3.0-or-later | Command layouts ported to `crates/op-core/src/m65.rs` (M65 RGB Elite), the K70 MK.2 performance lock and polling rate in `k70.rs`, the HS80 sleep timer, mic mute and firmware properties in `hs80.rs`; device names and types in `devices/corsair.toml` |
| [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB) | GPL-2.0-or-later | Device names and PIDs in `devices/corsair.toml` |
| [liquidctl](https://github.com/liquidctl/liquidctl) | GPL-3.0 | Device names and PIDs in `devices/corsair.toml`; the idea of generating udev rules from the device table |
| [ckb-next](https://github.com/ckb-next/ckb-next) | GPL-2.0 | Facts only (meaning of the K70 key-mode bits); no code |

Everything else was reverse-engineered on our own hardware with USB captures; see
`protocol/` for every command and how it was confirmed.

## Libraries

The Rust crates and npm packages in `Cargo.lock` and `app/package-lock.json` keep their
own licenses (mostly MIT / Apache-2.0), all compatible with GPL-3.0.
