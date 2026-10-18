//! K70 RGB MK.2 (legacy Corsair keyboard protocol). See protocol/k70_mk2_lp.md.
//!
//! Every report is 64 bytes on the vendor interface (no report IDs, so hidapi
//! needs a leading 0x00). Status: commands marked VERIFY in the spec still need
//! a capture from iCUE to confirm before they are trusted.

use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::{registry::FoundDevice, Error, Result, Rgb};

const REPORT_LEN: usize = 64;
/// Colour slots per plane: 14 rows x 12 (unused slots stay 0). Confirmed by capture.
pub const LED_COUNT: usize = 168;

const CMD_WRITE: u8 = 0x07;
const CMD_READ: u8 = 0x0E;
const CMD_STREAM: u8 = 0x7F;

const PROP_FIRMWARE: u8 = 0x01;
const PROP_SPECIAL_FUNCTION: u8 = 0x04;
const PROP_LIGHTING_CONTROL: u8 = 0x05;
const PROP_SUBMIT_COLOR_24: u8 = 0x28;
/// `07 0A 00 00 ms`: report interval in ms (1/2/4/8 = 1000/500/250/125 Hz).
/// From OpenLinkHub (k70mk2.go, GPL-3.0).
const PROP_POLLING_RATE: u8 = 0x0A;
/// `07 48 flags`: firmware "performance" lock, from OpenLinkHub (k70mk2.go).
const PROP_PERFORMANCE: u8 = 0x48;

const MODE_HARDWARE: u8 = 0x01;
const MODE_SOFTWARE: u8 = 0x02;

#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    pub version: u16,
    pub bootloader: u16,
    pub vid: u16,
    pub pid: u16,
    pub raw: [u8; REPORT_LEN],
}

const PROP_KEY_MODES: u8 = 0x40;
/// Key mode bits (from captures): 0x80 = send to the OS as HID, 0x40 = report to
/// the host (report 0x03). 0xC0 = normal key that the app also sees; 0x40 = key
/// swallowed for the app only (used for remapping).
const KEY_MODE_SOFTWARE: u8 = 0xC0;
pub const KEY_MODE_HOST_ONLY: u8 = 0x40;
/// Top-left keys, as key indices in the report 0x03 bitmap (confirmed on hardware
/// 2026-10-17). For these the key index is not the LED slot.
pub const KEY_PROFILE: u8 = 70;
pub const KEY_BRIGHTNESS: u8 = 0x47; // 71
pub const KEY_WINLOCK: u8 = 96;
/// LED under the Win Lock key (the lock symbol) = `led_for_key(KEY_WINLOCK)`.
pub const LED_WINLOCK: usize = 8;

/// LED slot of a key from its key index (report 0x03 bit, `07 40` index). The
/// colour planes are the key matrix transposed on a 12-wide grid: key 8 (F8) is
/// LED 96 and key 96 (Win Lock) is LED 8. Rule read off OpenLinkHub's
/// `database/keyboard/k70mk2.json` (keyData vs packetIndex), confirmed here.
pub fn led_for_key(key: u8) -> Option<usize> {
    let k = key as usize;
    (k < 144).then_some((k % 12) * 12 + k / 12)
}

/// Key indices iCUE switches to software mode on start (captured; 116 keys).
/// Brightness (0x47) is sent with mode 0xC1 instead of 0xC0: bit 0 = ACT_LIGHT in
/// ckb-next's naming (firmware brightness action); it is still reported to the host.
const KEY_INDICES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11,
    0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23,
    0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
    0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x40, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a,
    0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x51, 0x52, 0x53, 0x54, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e,
    0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x70, 0x71,
    0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x82, 0x83,
];

/// Polling intervals the firmware accepts, in ms.
pub const POLLING_MS: [u8; 4] = [1, 2, 4, 8];

/// What the firmware lock blocks (`07 48`). `windows` is the Win Lock key itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lock {
    pub windows: bool,
    pub alt_tab: bool,
    pub alt_f4: bool,
    pub shift_tab: bool,
}

impl Lock {
    /// `0xA0` = lock on, plus one bit per blocked key / combination; 0 = off.
    pub fn byte(&self) -> u8 {
        let bits =
            self.windows as u8 | (self.alt_tab as u8) << 1 | (self.alt_f4 as u8) << 2 | (self.shift_tab as u8) << 3;
        if bits == 0 {
            0
        } else {
            0xA0 | bits
        }
    }
}

pub struct K70 {
    /// MI_01 `FFC2:0004`: commands go out here (interrupt OUT 0x02).
    dev: HidDevice,
    /// MI_00 `FFC2:0003`: replies arrive here as report ID 0x0E (interrupt IN 0x81).
    replies: HidDevice,
    /// MI_00 `FFC0:0002`: key bitmap (report ID 0x03) while keys are in software mode.
    keys: Option<HidDevice>,
    /// hidraw node of the command collection, to find the USB device in sysfs.
    node: std::ffi::CString,
}

impl K70 {
    pub fn open(api: &HidApi, found: &FoundDevice) -> Result<Self> {
        let cmd = found.collection(0xFFC2, 0x0004).ok_or(Error::NotFound("K70 command collection"))?;
        let rep = found.collection(0xFFC2, 0x0003).ok_or(Error::NotFound("K70 reply collection"))?;
        let keys = found.collection(0xFFC0, 0x0002).and_then(|k| api.open_path(&k.path).ok());
        Ok(Self { dev: api.open_path(&cmd.path)?, replies: api.open_path(&rep.path)?, keys, node: cmd.path.clone() })
    }

    /// Route every key to the host only: the OS stops receiving typing from this
    /// keyboard until `set_software_mode(false)` (or replug). Used by remapping and
    /// by the key mapper.
    pub fn capture_keys(&self) -> Result<()> {
        self.set_key_modes(&[])
    }

    /// Every key reports to the host; keys in `swallowed` (LED slot = key index)
    /// stop reaching the OS so the app can replace what they do (remapping).
    pub fn set_key_modes(&self, swallowed: &[u8]) -> Result<()> {
        for chunk in KEY_INDICES.chunks(30) {
            let mut p = vec![CMD_WRITE, PROP_KEY_MODES, chunk.len() as u8, 0x00];
            for &k in chunk {
                let mode = if k == KEY_BRIGHTNESS {
                    KEY_MODE_SOFTWARE | 1
                } else if swallowed.contains(&k) {
                    KEY_MODE_HOST_ONLY
                } else {
                    KEY_MODE_SOFTWARE
                };
                p.extend_from_slice(&[k, mode]);
            }
            self.send(&p)?;
        }
        Ok(())
    }

    /// One key's mode (`07 40 01 00 key mode`).
    pub fn set_key_mode(&self, key: u8, mode: u8) -> Result<()> {
        self.send(&[CMD_WRITE, PROP_KEY_MODES, 0x01, 0x00, key, mode])
    }

    /// Next key bitmap (bit n set = key n held), or None on timeout.
    pub fn read_keys(&self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let dev = self.keys.as_ref().ok_or(Error::NotFound("K70 key collection"))?;
        let mut buf = [0u8; REPORT_LEN];
        let n = dev.read_timeout(&mut buf, timeout.as_millis() as i32)?;
        // Strip report ID 0x03.
        Ok((n > 1 && buf[0] == 0x03).then(|| buf[1..n].to_vec()))
    }

    fn send(&self, payload: &[u8]) -> Result<()> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[1..1 + payload.len()].copy_from_slice(payload);
        self.dev.write(&buf)?;
        Ok(())
    }

    fn recv(&self, timeout: Duration) -> Result<[u8; REPORT_LEN]> {
        let mut buf = [0u8; REPORT_LEN];
        // The report ID (0x0E) is the first byte and doubles as the command echo.
        let n = self.replies.read_timeout(&mut buf, timeout.as_millis() as i32)?;
        if n == 0 {
            return Err(Error::Timeout);
        }
        Ok(buf)
    }

    /// `0E prop`: read a property; the reply echoes `0E prop`.
    pub fn read_property(&self, prop: u8) -> Result<[u8; REPORT_LEN]> {
        self.send(&[CMD_READ, prop])?;
        // Skip unrelated input reports until the matching reply arrives.
        for _ in 0..8 {
            let r = self.recv(Duration::from_millis(500))?;
            if r[0] == CMD_READ && r[1] == prop {
                return Ok(r);
            }
        }
        Err(Error::BadReply(format!("no reply to 0e {prop:02x}")))
    }

    /// Read-only: safe to call while iCUE is running.
    pub fn firmware_info(&self) -> Result<FirmwareInfo> {
        let r = self.read_property(PROP_FIRMWARE)?;
        let le = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
        Ok(FirmwareInfo { version: le(8), bootloader: le(10), vid: le(12), pid: le(14), raw: r })
    }

    /// Software mode: the host drives the LEDs. Unplugging (or `false`) returns
    /// the keyboard to its onboard hardware lighting.
    pub fn set_software_mode(&self, on: bool) -> Result<()> {
        let mode = if on { MODE_SOFTWARE } else { MODE_HARDWARE };
        self.send(&[CMD_WRITE, PROP_SPECIAL_FUNCTION, mode])?;
        // byte 4 = 0x03 selects keyboard lighting (mice use 0x01).
        self.send(&[CMD_WRITE, PROP_LIGHTING_CONTROL, mode, 0x00, 0x03])
    }

    /// Push a full frame: one colour per LED slot (0..168).
    pub fn set_leds(&self, leds: &[Rgb; LED_COUNT]) -> Result<()> {
        let planes: [Vec<u8>; 3] = [
            leds.iter().map(|c| c.r).collect(),
            leds.iter().map(|c| c.g).collect(),
            leds.iter().map(|c| c.b).collect(),
        ];
        for (ch, plane) in planes.iter().enumerate() {
            // 168 bytes -> stream packets of 60, 60, 48 (as iCUE sends them).
            let chunks: Vec<&[u8]> = plane.chunks(60).collect();
            for (i, chunk) in chunks.iter().enumerate() {
                let mut p = vec![CMD_STREAM, i as u8 + 1, chunk.len() as u8, 0x00];
                p.extend_from_slice(chunk);
                self.send(&p)?;
            }
            let finish = if ch == 2 { 0x02 } else { 0x01 };
            self.send(&[CMD_WRITE, PROP_SUBMIT_COLOR_24, ch as u8 + 1, chunks.len() as u8, finish])?;
        }
        Ok(())
    }

    /// Firmware key lock (performance mode): the keyboard itself drops the
    /// blocked keys, also once back in hardware mode. OpenLinkHub then sets the
    /// Win Lock key back to `C0` (`07 40 01 00 60 C0`), as the lock command
    /// changes its mode; we do the same so it keeps reporting to the host.
    pub fn set_lock(&self, lock: Lock) -> Result<()> {
        self.send(&[CMD_WRITE, PROP_PERFORMANCE, lock.byte()])?;
        self.set_key_mode(KEY_WINLOCK, KEY_MODE_SOFTWARE)
    }

    /// Current polling interval in ms. The firmware does not answer `0E 0A`, but
    /// the keyboard re-enumerates with the new bInterval on its key endpoint
    /// (EP 0x81, interface 0), which Linux shows in sysfs. None elsewhere.
    pub fn polling_ms(&self) -> Option<u8> {
        polling_from_sysfs(self.node.to_str().ok()?)
    }

    /// Report interval in ms, one of [`POLLING_MS`]. The keyboard drops off the
    /// bus and comes back (new USB descriptors), so every handle is lost.
    pub fn set_polling_ms(&self, ms: u8) -> Result<()> {
        if !POLLING_MS.contains(&ms) {
            return Err(Error::BadReply(format!("unsupported polling interval {ms} ms")));
        }
        self.send(&[CMD_WRITE, PROP_POLLING_RATE, 0x00, 0x00, ms])
    }

    pub fn set_all(&self, c: Rgb) -> Result<()> {
        self.set_leds(&[c; LED_COUNT])
    }
}

#[cfg(target_os = "linux")]
fn polling_from_sysfs(node: &str) -> Option<u8> {
    // /dev/hidrawN -> /sys/class/hidraw/hidrawN/device = <usb-dev>:1.1/<hid-dev>
    let name = std::path::Path::new(node).file_name()?;
    let hid = std::fs::canonicalize(std::path::Path::new("/sys/class/hidraw").join(name).join("device")).ok()?;
    let iface = hid.parent()?; // e.g. 5-2.1.3:1.1
    let usb = iface.file_name()?.to_str()?.split(':').next()?.to_string();
    let interval = std::fs::read_to_string(iface.parent()?.join(format!("{usb}:1.0/ep_81/interval"))).ok()?;
    parse_interval(&interval)
}

#[cfg(not(target_os = "linux"))]
fn polling_from_sysfs(_node: &str) -> Option<u8> {
    None
}

/// sysfs `interval`, e.g. "8ms".
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_interval(s: &str) -> Option<u8> {
    s.trim().strip_suffix("ms")?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_byte() {
        assert_eq!(Lock::default().byte(), 0);
        assert_eq!(Lock { windows: true, ..Default::default() }.byte(), 0xA1);
        assert_eq!(Lock { windows: true, alt_tab: true, alt_f4: true, shift_tab: true }.byte(), 0xAF);
        assert_eq!(Lock { alt_f4: true, ..Default::default() }.byte(), 0xA4);
    }

    #[test]
    fn key_to_led() {
        assert_eq!(led_for_key(KEY_WINLOCK), Some(LED_WINLOCK));
        assert_eq!(led_for_key(8), Some(96)); // F8
        assert_eq!(led_for_key(13), Some(13)); // 1: on the diagonal
        assert_eq!(led_for_key(KEY_BRIGHTNESS), Some(137));
        assert_eq!(led_for_key(119), Some(141)); // keypad .
        assert!((0..144).all(|k| led_for_key(led_for_key(k).unwrap() as u8) == Some(k as usize)));
    }

    #[test]
    fn interval() {
        assert_eq!(parse_interval("8ms\n"), Some(8));
        assert_eq!(parse_interval("125us"), None);
    }
}
