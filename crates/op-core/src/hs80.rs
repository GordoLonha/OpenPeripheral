//! HS80 RGB Wireless via its USB receiver ("Bragi" protocol). See protocol/hs80_bragi.md.
//!
//! Request layout after the report ID: [target, command, property, 0x00, value...].
//! target 0x08 = the receiver itself, 0x09 = the paired headset.
//! Only read commands (GET_PROPERTY) are exposed until writes are confirmed by capture.

use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::{descriptor, registry::FoundDevice, Error, Result, Rgb};

pub const TARGET_RECEIVER: u8 = 0x08;
pub const TARGET_HEADSET: u8 = 0x09;

const CMD_SET_PROPERTY: u8 = 0x01;
const CMD_GET_PROPERTY: u8 = 0x02;
const CMD_WRITE_DATA: u8 = 0x06;
const CMD_OPEN_ENDPOINT: u8 = 0x0D;

const PROP_MODE: u8 = 0x03;
const MODE_HARDWARE: u16 = 0x01;
const MODE_SOFTWARE: u16 = 0x02;
const RESOURCE_LIGHTING: u8 = 0x01;
/// 3 LEDs x 3 bytes, planar: RRR GGG BBB. LED 1 is the status LED next to the
/// volume wheel (confirmed 2026-10-17; iCUE blinks it green 80/00 while charging).
pub const LED_BYTES: usize = 9;
pub const LED_COUNT: usize = 3;
pub const LED_STATUS: usize = 1;
/// Mic LED (the boom's tip). OpenLinkHub paints it red while muted.
pub const LED_MIC: usize = 2;

pub const PROP_BATTERY_LEVEL: u8 = 0x0F; // 0..1000 (tenths of a percent)
pub const PROP_BATTERY_STATUS: u8 = 0x10; // 1 charging, 2 discharging (confirmed), 3 full (assumed)
                                          // From OpenLinkHub (hs80rgbW.go / headsetdongle.go, GPL-3.0):
pub const PROP_SLEEP_ENABLED: u8 = 0x0D; // 1 / 0
pub const PROP_SLEEP_MS: u8 = 0x0E; // u32 LE, idle time before the headset powers off
pub const PROP_FIRMWARE: u8 = 0x13; // reply value = 3 bytes, major.minor.patch
pub const PROP_MIC_MUTE: u8 = 0xA6; // 1 = muted (boom flipped up)
/// Mute notifications carry 0x8E or 0xA6 (OpenLinkHub accepts both).
pub const PROP_MIC_MUTE_EVENT: u8 = 0x8E;
/// Sleep timer choices, in minutes (0 = never), as iCUE / OpenLinkHub offer.
pub const SLEEP_MINUTES: [u16; 7] = [0, 1, 5, 10, 15, 30, 60];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    Unknown,
}

const BRAGI_OUT_REPORT: u8 = 0x02;
/// Replies to our requests.
const BRAGI_IN_REPORT: u8 = 0x01;
/// Unsolicited notifications (collection `FF42:0002`).
const BRAGI_EVENT_REPORT: u8 = 0x03;

/// Something the headset reported on its own (report 0x03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Volume wheel pressed / released (`03 01 02 01` / `03 01 02 00`). Needs
    /// software mode. Confirmed on hardware 2026-10-17.
    WheelButton { pressed: bool },
    /// A property changed, e.g. battery (`03 01 01 0f 00 2a 03` = 0x032a).
    Property { prop: u8, value: u16 },
}

/// Parse one input report (with its report ID) into an event.
pub fn parse_event(r: &[u8]) -> Option<Event> {
    match r {
        [BRAGI_EVENT_REPORT, _, 0x02, state, ..] => Some(Event::WheelButton { pressed: *state != 0 }),
        [BRAGI_EVENT_REPORT, _, 0x01, prop, _, lo, hi, ..] => {
            Some(Event::Property { prop: *prop, value: u16::from_le_bytes([*lo, *hi]) })
        }
        _ => None,
    }
}

/// Planar frame from per-LED colours.
pub fn led_frame(leds: &[Rgb; LED_COUNT]) -> [u8; LED_BYTES] {
    let mut f = [0u8; LED_BYTES];
    for (i, c) in leds.iter().enumerate() {
        f[i] = c.r;
        f[LED_COUNT + i] = c.g;
        f[2 * LED_COUNT + i] = c.b;
    }
    f
}

/// PID of the headset itself on its USB cable (vs 0x0A6B receiver).
pub const PID_WIRED: u16 = 0x0A69;

pub struct Hs80 {
    dev: HidDevice,
    /// Separate handle for notifications. On Windows that is its own collection;
    /// on Linux the same hidraw node opened twice (each reader gets every report),
    /// so request/reply traffic never swallows an event.
    events: Option<HidDevice>,
    /// Who answers headset properties: 0x09 through the receiver, 0x08 when the
    /// headset is cabled (it is then the device itself). Confirmed on hardware.
    headset: u8,
    pub wired: bool,
    report_id: u8,
    out_len: usize,
}

impl Hs80 {
    /// Opens the Bragi command collection (`FF42:0001`: out 0x02, in 0x01).
    pub fn open(api: &HidApi, found: &FoundDevice) -> Result<Self> {
        let iface = found.collection(0xFF42, 0x0001).ok_or(Error::NotFound("HS80 Bragi collection"))?;
        let dev = api.open_path(&iface.path)?;
        let mut raw = [0u8; 4096];
        let n = dev.get_report_descriptor(&mut raw)?;
        let info = descriptor::parse(&raw[..n]);
        // Bragi requests go out on report 0x02. On Linux one hidraw node carries
        // the whole interface (reports 01 02 03 0e 0f 11 21 58), so pick 0x02
        // explicitly rather than "the first report with an output".
        let (&id, sizes) = info
            .reports
            .get_key_value(&BRAGI_OUT_REPORT)
            .or_else(|| info.reports.iter().find(|(_, s)| s.output > 0))
            .ok_or(Error::NotFound("HS80 output report"))?;
        let wired = found.pid == PID_WIRED;
        let events = found.collection(0xFF42, 0x0002).and_then(|c| api.open_path(&c.path).ok());
        Ok(Self { dev, events, headset: if wired { TARGET_RECEIVER } else { TARGET_HEADSET }, wired, report_id: id, out_len: sizes.output })
    }

    pub fn report_id(&self) -> u8 {
        self.report_id
    }

    /// Send a raw request (bytes after the report ID) and return the first reply.
    pub fn transact(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; self.out_len + 1];
        buf[0] = self.report_id;
        buf[1..1 + payload.len()].copy_from_slice(payload);
        self.dev.write(&buf)?;
        let mut r = vec![0u8; 256];
        for _ in 0..8 {
            let n = self.dev.read_timeout(&mut r, Duration::from_millis(500).as_millis() as i32)?;
            if n == 0 {
                return Err(Error::Timeout);
            }
            // One hidraw node carries replies and notifications alike: only report
            // 0x01 is a reply (an event `03 01 02 01` would otherwise look like one).
            if self.report_id != 0 && r[0] != BRAGI_IN_REPORT {
                continue;
            }
            let body = strip_report_id(&r[..n], self.report_id);
            // Reply echoes [target?, command]; accept the first one carrying our command.
            if body.len() >= 2 && (body[1] == payload[1] || body[0] == payload[1]) {
                return Ok(body.to_vec());
            }
        }
        Err(Error::BadReply("no matching reply".into()))
    }

    pub fn get_property(&self, target: u8, prop: u8) -> Result<Vec<u8>> {
        self.transact(&[target, CMD_GET_PROPERTY, prop, 0x00])
    }

    /// SET_PROPERTY: `[target, 01, prop, 00, value LE (u16)]`.
    pub fn set_property(&self, target: u8, prop: u8, value: u16) -> Result<()> {
        let [lo, hi] = value.to_le_bytes();
        let r = self.transact(&[target, CMD_SET_PROPERTY, prop, 0x00, lo, hi])?;
        check_status(&r)
    }

    /// SET_PROPERTY with a 32-bit value (the sleep timer).
    pub fn set_property_u32(&self, target: u8, prop: u8, value: u32) -> Result<()> {
        let mut p = vec![target, CMD_SET_PROPERTY, prop, 0x00];
        p.extend_from_slice(&value.to_le_bytes());
        check_status(&self.transact(&p)?)
    }

    /// Value bytes of a headset property (after `[index, cmd, status]`).
    fn headset_value(&self, prop: u8) -> Result<Vec<u8>> {
        let r = self.get_property(self.headset, prop)?;
        check_status(&r)?;
        Ok(r[3..].to_vec())
    }

    /// Auto power-off after `minutes` idle; 0 disables it.
    pub fn set_sleep(&self, minutes: u16) -> Result<()> {
        self.set_property(self.headset, PROP_SLEEP_ENABLED, (minutes > 0) as u16)?;
        if minutes > 0 {
            self.set_property_u32(self.headset, PROP_SLEEP_MS, minutes as u32 * 60_000)?;
        }
        Ok(())
    }

    /// Firmware version of the headset, e.g. "5.13.123".
    pub fn firmware(&self) -> Result<String> {
        let v = self.headset_value(PROP_FIRMWARE)?;
        match v[..] {
            [a, b, c, ..] => Ok(format!("{a}.{b}.{c}")),
            _ => Err(Error::BadReply(hex(&v))),
        }
    }

    /// Mic muted (boom flipped up).
    pub fn mic_muted(&self) -> Result<bool> {
        let v = self.headset_value(PROP_MIC_MUTE)?;
        v.first().map(|&m| m == 1).ok_or_else(|| Error::BadReply("empty mute reply".into()))
    }

    /// Software mode on receiver + headset (as iCUE does on start); `false` returns
    /// both to their onboard lighting (rainbow).
    pub fn set_software_mode(&self, on: bool) -> Result<()> {
        let mode = if on { MODE_SOFTWARE } else { MODE_HARDWARE };
        self.set_property(TARGET_RECEIVER, PROP_MODE, mode)?;
        self.set_property(self.headset, PROP_MODE, mode)
    }

    /// Next notification, waiting up to `timeout_ms` (0 = just poll).
    pub fn read_event(&self, timeout_ms: i32) -> Result<Option<Event>> {
        let Some(ev) = &self.events else { return Ok(None) };
        let mut r = [0u8; 64];
        loop {
            let n = ev.read_timeout(&mut r, timeout_ms)?;
            if n == 0 {
                return Ok(None);
            }
            if let Some(e) = parse_event(&r[..n]) {
                return Ok(Some(e));
            }
            // Replies and volume-wheel reports (0x0e) also arrive here on Linux.
        }
    }

    /// Open handle 0 on the lighting resource (`0D 00 01`). Needed once after
    /// entering software mode, before `write_leds`.
    pub fn open_lighting(&self) -> Result<()> {
        let r = self.transact(&[self.headset, CMD_OPEN_ENDPOINT, 0x00, RESOURCE_LIGHTING])?;
        // status 03 = already open (e.g. second call) is fine.
        if r.get(2).is_some_and(|&s| s != 0 && s != 3) {
            return Err(Error::BadReply(hex(&r[..r.len().min(16)])));
        }
        Ok(())
    }

    /// Raw lighting frame: 9 bytes for the headset's 3 LEDs. The headset drops
    /// back to hardware lighting if frames stop, so callers resend about every 0.5 s.
    pub fn write_leds(&self, data: &[u8; LED_BYTES]) -> Result<()> {
        let mut p = vec![self.headset, CMD_WRITE_DATA, 0x00];
        p.extend_from_slice(&(LED_BYTES as u32).to_le_bytes());
        p.extend_from_slice(data);
        let r = self.transact(&p)?;
        check_status(&r)
    }

    /// Charging state (property 0x10). Confirmed: 1 while the USB cable is
    /// plugged, 2 on battery; 3 is assumed to be "full" (not yet observed).
    pub fn battery_status(&self) -> Result<BatteryStatus> {
        let r = self.get_property(self.headset, PROP_BATTERY_STATUS)?;
        if r.len() < 4 || r[2] != 0 {
            return Err(Error::BadReply(hex(&r[..r.len().min(16)])));
        }
        Ok(match r[3] {
            1 => BatteryStatus::Charging,
            2 => BatteryStatus::Discharging,
            3 => BatteryStatus::Full,
            _ => BatteryStatus::Unknown,
        })
    }

    /// Battery percentage (0.0..100.0) of the headset.
    ///
    /// Replies don't name the property, so if another program (iCUE, a second
    /// instance) queries the headset at the same moment we may pick up its reply
    /// (seen: status `1` read as 0.1 %). Require two agreeing reads.
    pub fn battery_percent(&self) -> Result<f32> {
        let mut last = self.battery_percent_once()?;
        for _ in 0..3 {
            let next = self.battery_percent_once()?;
            if (next - last).abs() <= 2.0 {
                return Ok(next);
            }
            last = next;
        }
        Err(Error::BadReply("battery readings disagree (concurrent access?)".into()))
    }

    fn battery_percent_once(&self) -> Result<f32> {
        let r = self.get_property(self.headset, PROP_BATTERY_LEVEL)?;
        // Reply: [target, cmd, status, lo, hi]; status 0 = ok.
        if r.len() < 5 || r[2] != 0 {
            return Err(Error::BadReply(hex(&r[..r.len().min(16)])));
        }
        Ok(u16::from_le_bytes([r[3], r[4]]) as f32 / 10.0)
    }
}

/// Reply header is `[index, command, status, …]`; status 0 = ok.
fn check_status(r: &[u8]) -> Result<()> {
    match r.get(2) {
        Some(0) => Ok(()),
        _ => Err(Error::BadReply(hex(&r[..r.len().min(16)]))),
    }
}

/// Replies come on a different report ID than requests (out 0x02, in 0x01), so
/// when the collection uses report IDs the first byte is always the ID.
fn strip_report_id(r: &[u8], id: u8) -> &[u8] {
    if id != 0 && !r.is_empty() {
        &r[1..]
    } else {
        r
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events() {
        assert_eq!(parse_event(&[0x03, 0x01, 0x02, 0x01, 0x00]), Some(Event::WheelButton { pressed: true }));
        assert_eq!(parse_event(&[0x03, 0x01, 0x02, 0x00, 0x00]), Some(Event::WheelButton { pressed: false }));
        assert_eq!(
            parse_event(&[0x03, 0x01, 0x01, 0x0f, 0x00, 0x2a, 0x03]),
            Some(Event::Property { prop: 0x0f, value: 0x032a })
        );
        assert_eq!(parse_event(&[0x01, 0x01, 0x02, 0x00, 0x76, 0x02]), None); // a reply
        assert_eq!(parse_event(&[0x0e, 0x01]), None); // volume wheel turn
    }

    #[test]
    fn planar_leds() {
        let f = led_frame(&[Rgb::new(1, 2, 3), Rgb::new(4, 5, 6), Rgb::new(7, 8, 9)]);
        assert_eq!(f, [1, 4, 7, 2, 5, 8, 3, 6, 9]);
    }
}
