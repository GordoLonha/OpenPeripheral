//! M65 RGB Elite (1B1C:1B5A): same 64-byte "nxp" protocol as the K70 MK.2
//! (07 write / 0E read, commands on MI_01 FFC2:0004, replies on MI_00 FFC2:0003).
//! See protocol/m65.md.
//!
//! Command layouts ported from OpenLinkHub's m65rgbelite driver
//! (https://github.com/jurkovic-nikola/OpenLinkHub, GPL-3.0-or-later).

use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::{registry::FoundDevice, Error, Result, Rgb};

const REPORT_LEN: usize = 64;
const CMD_WRITE: u8 = 0x07;
const CMD_READ: u8 = 0x0E;
const PROP_FIRMWARE: u8 = 0x01;
const PROP_SPECIAL_FUNCTION: u8 = 0x04;
const PROP_LIGHTING_CONTROL: u8 = 0x05;
const PROP_POLLING_RATE: u8 = 0x0A;
const PROP_DPI: u8 = 0x13;
const PROP_COLOR: u8 = 0x22;
const PROP_BUTTON_OPTIMIZATION: u8 = 0x30;
const MODE_HARDWARE: u8 = 0x01;
const MODE_SOFTWARE: u8 = 0x02;

/// LED zones, in the order of the colour packet.
pub const LED_LOGO: u8 = 0x02;
pub const LED_DPI: u8 = 0x03;
pub const LED_WHEEL: u8 = 0x04;

/// Five DPI stages plus the sniper stage.
pub const DPI_STAGES: usize = 5;
pub const DPI_MIN: u16 = 100;
pub const DPI_MAX: u16 = 18_000;

/// Button bits in report 0x03 byte 1 (confirmed 2026-10-17).
pub const BUTTON_DPI_UP: u8 = 0x20;
pub const BUTTON_DPI_DOWN: u8 = 0x40;
/// Held while pressed; the firmware lowers the DPI itself (confirmed).
pub const BUTTON_SNIPER: u8 = 0x80;

/// Buttons: (bit in report 0x03, index for key modes, name used in bindings).
/// Indices 1..8 follow the bits in order, as in OpenLinkHub's key assignment.
pub const BUTTONS: [(u8, u8, &str); 8] = [
    (0x01, 1, "left"),
    (0x02, 2, "right"),
    (0x04, 3, "middle"),
    (0x08, 4, "back"),
    (0x10, 5, "forward"),
    (0x20, 6, "dpi_up"),
    (0x40, 7, "dpi_down"),
    (0x80, 8, "sniper"),
];

const PROP_KEY_MODES: u8 = 0x40;
const MODE_NORMAL: u8 = 0xC0;
const MODE_HOST_ONLY: u8 = 0x40;

pub struct M65 {
    dev: HidDevice,
    /// hidraw path of the command interface, to read the USB report interval.
    node: std::ffi::CString,
    replies: HidDevice,
    /// MI_00 `FFC1:0002`: button bitmap (report 0x03).
    events: Option<HidDevice>,
}

#[derive(Debug, Clone, Copy)]
pub struct FirmwareInfo {
    pub version: u16,
    pub bootloader: u16,
    pub pid: u16,
}

impl M65 {
    pub fn open(api: &HidApi, found: &FoundDevice) -> Result<Self> {
        let cmd = found.collection(0xFFC2, 0x0004).ok_or(Error::NotFound("M65 command collection"))?;
        let rep = found.collection(0xFFC2, 0x0003).ok_or(Error::NotFound("M65 reply collection"))?;
        let events = found.collection(0xFFC1, 0x0002).and_then(|c| api.open_path(&c.path).ok());
        Ok(Self { dev: api.open_path(&cmd.path)?, node: cmd.path.clone(), replies: api.open_path(&rep.path)?, events })
    }

    /// Next button bitmap (report 0x03 byte 1), waiting up to `timeout_ms`.
    /// DPI buttons are still handled by the firmware in software mode; this
    /// only lets the host follow them.
    pub fn read_buttons(&self, timeout_ms: i32) -> Result<Option<u8>> {
        let Some(ev) = &self.events else { return Ok(None) };
        let mut r = [0u8; REPORT_LEN];
        loop {
            let n = ev.read_timeout(&mut r, timeout_ms)?;
            if n == 0 {
                return Ok(None);
            }
            if r[0] == 0x03 && n > 1 {
                return Ok(Some(r[1]));
            }
        }
    }

    fn send(&self, payload: &[u8]) -> Result<()> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[1..1 + payload.len()].copy_from_slice(payload);
        self.dev.write(&buf)?;
        Ok(())
    }

    fn read_property(&self, prop: u8) -> Result<[u8; REPORT_LEN]> {
        self.send(&[CMD_READ, prop])?;
        let mut r = [0u8; REPORT_LEN];
        for _ in 0..8 {
            let n = self.replies.read_timeout(&mut r, Duration::from_millis(500).as_millis() as i32)?;
            if n == 0 {
                return Err(Error::Timeout);
            }
            if r[0] == CMD_READ && r[1] == prop {
                return Ok(r);
            }
        }
        Err(Error::BadReply(format!("no reply to 0e {prop:02x}")))
    }

    /// Read-only.
    pub fn firmware_info(&self) -> Result<FirmwareInfo> {
        let r = self.read_property(PROP_FIRMWARE)?;
        let le = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
        Ok(FirmwareInfo { version: le(8), bootloader: le(10), pid: le(14) })
    }

    /// Software mode: the host drives the LEDs. `false` (or unplugging) returns
    /// the mouse to its onboard lighting and DPI handling.
    pub fn set_software_mode(&self, on: bool) -> Result<()> {
        let mode = if on { MODE_SOFTWARE } else { MODE_HARDWARE };
        self.send(&[CMD_WRITE, PROP_SPECIAL_FUNCTION, mode])?;
        if !on {
            // Undo the input/lighting selection set by `set_button_modes`.
            self.send(&[CMD_WRITE, PROP_LIGHTING_CONTROL, MODE_HARDWARE, 0x00, 0x01])?;
        }
        Ok(())
    }

    /// One packet for the three zones: `07 22 03 01 [02 rgb] [03 rgb] [04 rgb]`.
    pub fn set_leds(&self, logo: Rgb, dpi: Rgb, wheel: Rgb) -> Result<()> {
        let mut p = vec![CMD_WRITE, PROP_COLOR, 0x03, 0x01];
        for (zone, c) in [(LED_LOGO, logo), (LED_DPI, dpi), (LED_WHEEL, wheel)] {
            p.extend_from_slice(&[zone, c.r, c.g, c.b]);
        }
        self.send(&p)
    }

    /// Experiment: colour only some zones (count = number of pairs).
    pub fn set_zones(&self, zones: &[(u8, Rgb)]) -> Result<()> {
        let mut p = vec![CMD_WRITE, PROP_COLOR, zones.len() as u8, 0x01];
        for (zone, c) in zones {
            p.extend_from_slice(&[*zone, c.r, c.g, c.b]);
        }
        self.send(&p)
    }

    /// Define a DPI stage (0..5) or the sniper stage (`None`) with its indicator
    /// colour: `07 13 (D1+stage | D0) 00 00 x_lo x_hi y_lo y_hi r g b` (confirmed).
    pub fn set_dpi_stage_xy(&self, stage: Option<usize>, x: u16, y: u16, color: Rgb) -> Result<()> {
        let index = match stage {
            Some(s) if s < DPI_STAGES => 0xD1 + s as u8,
            Some(_) => return Err(Error::BadReply("DPI stage out of range".into())),
            None => 0xD0,
        };
        let [xl, xh] = x.clamp(DPI_MIN, DPI_MAX).to_le_bytes();
        let [yl, yh] = y.clamp(DPI_MIN, DPI_MAX).to_le_bytes();
        self.send(&[CMD_WRITE, PROP_DPI, index, 0x00, 0x00, xl, xh, yl, yh, color.r, color.g, color.b])
    }

    /// Same DPI on both axes.
    pub fn set_dpi_stage(&self, stage: Option<usize>, dpi: u16, color: Rgb) -> Result<()> {
        self.set_dpi_stage_xy(stage, dpi, dpi, color)
    }

    /// Make a stage active: `07 13 02 00 n`, n = stage + 1, 0 = sniper.
    pub fn select_dpi_stage(&self, stage: Option<usize>) -> Result<()> {
        let n = stage.map_or(0, |s| s.min(DPI_STAGES - 1) as u8 + 1);
        self.send(&[CMD_WRITE, PROP_DPI, 0x02, 0x00, n])
    }

    /// Current USB report interval in ms (Linux: from sysfs).
    pub fn polling_ms(&self) -> Option<u8> {
        crate::k70::polling_from_sysfs(self.node.to_str().ok()?)
    }

    /// Angle snapping (iCUE "Correção de Ângulo"): `07 13 04 00 v` and `07 13 04 01 v`.
    pub fn set_angle_snapping(&self, on: bool) -> Result<()> {
        for i in 0..=1 {
            self.send(&[CMD_WRITE, PROP_DPI, 0x04, i, on as u8])?;
        }
        Ok(())
    }

    /// Button response optimization: `07 30 00 00 v`, 1 = off, 2 = on.
    pub fn set_button_optimization(&self, on: bool) -> Result<()> {
        self.send(&[CMD_WRITE, PROP_BUTTON_OPTIMIZATION, 0x00, 0x00, on as u8 + 1])
    }

    /// Buttons in `host_only` (bits) stop reaching the OS and are only reported to
    /// the host (remapping); the others act normally:
    /// `07 40 08 00 [index mode] x8`, mode C0 normal / 40 host only.
    pub fn set_button_modes(&self, host_only: u8) -> Result<()> {
        let mut p = vec![CMD_WRITE, PROP_KEY_MODES, BUTTONS.len() as u8, 0x00];
        for (bit, index, _) in BUTTONS {
            p.extend_from_slice(&[index, if host_only & bit != 0 { MODE_HOST_ONLY } else { MODE_NORMAL }]);
        }
        self.send(&p)?;
        // Commit the input selection (lighting control, 01 = mouse): without
        // it the modes do not take (confirmed).
        self.send(&[CMD_WRITE, PROP_LIGHTING_CONTROL, MODE_SOFTWARE, 0x00, 0x01])
    }

    /// Report interval in ms: 1, 2, 4 or 8 (1000/500/250/125 Hz).
    pub fn set_polling_ms(&self, ms: u8) -> Result<()> {
        if ![1, 2, 4, 8].contains(&ms) {
            return Err(Error::BadReply(format!("unsupported polling interval {ms} ms")));
        }
        self.send(&[CMD_WRITE, PROP_POLLING_RATE, 0x00, 0x00, ms])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dpi_index() {
        assert_eq!(0xD1 + 4u8, 0xD5);
        assert_eq!(0xD0u8, 208); // sniper, as in OpenLinkHub (209 - 1)
    }
}
