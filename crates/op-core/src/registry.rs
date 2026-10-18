//! Device database (devices/corsair.toml) and discovery of the vendor-defined
//! control interfaces of the devices it lists.

use std::{collections::BTreeMap, ffi::CString, fmt, path::PathBuf, sync::OnceLock};

use hidapi::HidApi;
use serde::Deserialize;

pub const CORSAIR_VID: u16 = 0x1B1C;

/// The database shipped with the binary.
pub const EMBEDDED_DB: &str = include_str!("../../../devices/corsair.toml");

/// Which driver handles a device. Derived from the database entry's family and kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// K70 RGB MK.2 family: legacy 64-byte protocol (0x07 / 0x0E / 0x7F commands).
    LegacyKeyboard,
    /// Wireless dongle speaking the "Bragi" protocol (HS80 receiver).
    BragiDongle,
    /// Mouse on the legacy 64-byte protocol (M65 RGB Elite).
    LegacyMouse,
    /// Known device without a driver (yet): listed, never opened.
    Unsupported,
}

/// Wire protocol family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    /// K70 MK.2 legacy protocol (protocol/k70_mk2_lp.md).
    Nxp,
    /// HS80 and newer (protocol/hs80_bragi.md).
    Bragi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Tested on real hardware with OpenPeripheral.
    Verified,
    /// Reported working by a user.
    Community,
    /// Only known to exist: never written to.
    Untested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Keyboard,
    Mouse,
    Mousepad,
    Headset,
    HeadsetDongle,
    Dongle,
    Cooler,
    Controller,
    Psu,
    Other,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Family::Nxp => "nxp",
            Family::Bragi => "bragi",
        }
    }
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Verified => "verified",
            Status::Community => "community",
            Status::Untested => "untested",
        }
    }
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Keyboard => "keyboard",
            Kind::Mouse => "mouse",
            Kind::Mousepad => "mousepad",
            Kind::Headset => "headset",
            Kind::HeadsetDongle => "headset-dongle",
            Kind::Dongle => "dongle",
            Kind::Cooler => "cooler",
            Kind::Controller => "controller",
            Kind::Psu => "psu",
            Kind::Other => "other",
        }
    }
}

/// A HID collection as "usage_page:usage" in hex, e.g. "FFC2:0004".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Usage {
    pub page: u16,
    pub usage: u16,
}

impl TryFrom<String> for Usage {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        let (p, u) = s.split_once(':').ok_or_else(|| format!("usage {s:?}: expected PAGE:USAGE"))?;
        let hex = |x: &str| u16::from_str_radix(x, 16).map_err(|e| format!("usage {s:?}: {e}"));
        Ok(Usage { page: hex(p)?, usage: hex(u)? })
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}:{:04X}", self.page, self.usage)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEntry {
    pub pid: u16,
    pub name: String,
    pub kind: Kind,
    #[serde(default)]
    pub family: Option<Family>,
    pub status: Status,
    /// Role (command, reply, keys, events) -> collection.
    #[serde(default)]
    pub control: BTreeMap<String, Usage>,
    #[serde(default)]
    pub leds: Option<u32>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    /// Other PIDs of the same physical device.
    #[serde(default)]
    pub pairs: Vec<u16>,
    #[serde(default)]
    pub sources: Vec<String>,
}

impl DeviceEntry {
    pub fn driver(&self) -> DeviceKind {
        if self.control.is_empty() {
            return DeviceKind::Unsupported;
        }
        match (self.family, self.kind) {
            (Some(Family::Nxp), Kind::Keyboard) => DeviceKind::LegacyKeyboard,
            (Some(Family::Nxp), Kind::Mouse) => DeviceKind::LegacyMouse,
            (Some(Family::Bragi), Kind::Headset | Kind::HeadsetDongle) => DeviceKind::BragiDongle,
            _ => DeviceKind::Unsupported,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Db {
    pub devices: Vec<DeviceEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DbFile {
    #[serde(default)]
    device: Vec<DeviceEntry>,
}

impl Db {
    /// Parse a database file; duplicate PIDs are an error.
    pub fn parse(text: &str) -> Result<Db, String> {
        let file: DbFile = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut seen = std::collections::HashSet::new();
        for d in &file.device {
            if !seen.insert(d.pid) {
                return Err(format!("duplicate pid {:04x}", d.pid));
            }
        }
        Ok(Db { devices: file.device })
    }

    pub fn embedded() -> Db {
        Db::parse(EMBEDDED_DB).expect("devices/corsair.toml is valid (checked by tests)")
    }

    /// Entries of `other` replace ours with the same PID; new ones are added.
    pub fn merge(&mut self, other: Db) {
        for d in other.devices {
            match self.devices.iter_mut().find(|e| e.pid == d.pid) {
                Some(e) => *e = d,
                None => self.devices.push(d),
            }
        }
    }

    pub fn get(&self, pid: u16) -> Option<&DeviceEntry> {
        self.devices.iter().find(|d| d.pid == pid)
    }
}

/// `<config dir>/dev.openperipheral.app/devices.toml`, the app's config dir.
pub fn user_db_path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    Some(base?.join("dev.openperipheral.app").join("devices.toml"))
}

/// The embedded database merged with the user's file, loaded once.
pub fn db() -> &'static Db {
    static DB: OnceLock<Db> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = Db::embedded();
        if let Some(path) = user_db_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                match Db::parse(&text) {
                    Ok(user) => db.merge(user),
                    Err(e) => eprintln!("ignoring {}: {e}", path.display()),
                }
            }
        }
        db
    })
}

/// udev rules giving the logged-in user hidraw access to every device that is not untested.
pub fn udev_rules(db: &Db) -> String {
    let mut s = String::from(
        "# OpenPeripheral: let the logged-in user access supported Corsair HID devices.\n\
         # Generated from devices/corsair.toml by `cargo run -p opctl -- udev-rules`; do not edit.\n\
         # Must sort before 73-seat-late.rules, which turns the uaccess tag into ACLs.\n\
         # Install: sudo cp 70-openperipheral.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger\n",
    );
    for d in db.devices.iter().filter(|d| d.status != Status::Untested) {
        s += &format!(
            "# {}\nKERNEL==\"hidraw*\", ATTRS{{idVendor}}==\"{:04x}\", ATTRS{{idProduct}}==\"{:04x}\", TAG+=\"uaccess\"\n",
            d.name, CORSAIR_VID, d.pid
        );
    }
    s += "# Virtual keyboard/mouse for remapped keys and macros (works on Wayland and X11)\n\
          KERNEL==\"uinput\", SUBSYSTEM==\"misc\", TAG+=\"uaccess\", OPTIONS+=\"static_node=uinput\"\n";
    s
}

#[derive(Debug, Clone)]
pub struct HidInterface {
    pub path: CString,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
}

#[derive(Debug, Clone)]
pub struct FoundDevice {
    pub name: &'static str,
    pub kind: DeviceKind,
    pub pid: u16,
    pub serial: String,
    pub status: Status,
    pub family: Option<Family>,
    /// The database entry it was matched with.
    pub entry: &'static DeviceEntry,
    /// Every vendor-defined (usage page >= 0xFF00) collection, on any interface.
    /// Drivers pick theirs by usage: commands and replies may live on different
    /// interfaces (K70: commands on MI_01, replies on MI_00).
    pub control: Vec<HidInterface>,
}

impl FoundDevice {
    pub fn collection(&self, usage_page: u16, usage: u16) -> Option<&HidInterface> {
        self.control.iter().find(|c| c.usage_page == usage_page && c.usage == usage)
    }

    /// Untested devices are read-only: no lighting, modes or key modes.
    pub fn writable(&self) -> bool {
        self.status != Status::Untested
    }
}

/// Every plugged-in Corsair device the database knows, one per PID + serial.
pub fn enumerate(api: &HidApi) -> Vec<FoundDevice> {
    let db = db();
    let mut out: Vec<FoundDevice> = Vec::new();
    for info in api.device_list().filter(|d| d.vendor_id() == CORSAIR_VID) {
        let Some(known) = db.get(info.product_id()) else { continue };
        let serial = info.serial_number().unwrap_or_default().to_string();
        let idx = match out.iter().position(|d| d.pid == known.pid && d.serial == serial) {
            Some(i) => i,
            None => {
                out.push(FoundDevice {
                    name: &known.name,
                    kind: known.driver(),
                    pid: known.pid,
                    serial,
                    status: known.status,
                    family: known.family,
                    entry: known,
                    control: vec![],
                });
                out.len() - 1
            }
        };
        if info.usage_page() >= 0xFF00 {
            out[idx].control.push(HidInterface {
                path: info.path().to_owned(),
                interface: info.interface_number(),
                usage_page: info.usage_page(),
                usage: info.usage(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_db_parses_without_duplicates() {
        let db = Db::parse(EMBEDDED_DB).unwrap();
        assert!(db.devices.len() > 200, "seeded list present");
        assert!(Db::parse("[[device]]\npid = 1\nname = \"a\"\nkind = \"other\"\nstatus = \"untested\"\n\n[[device]]\npid = 1\nname = \"b\"\nkind = \"other\"\nstatus = \"untested\"\n").is_err());
    }

    #[test]
    fn verified_entries_are_complete() {
        for d in Db::embedded().devices.iter().filter(|d| d.status == Status::Verified) {
            assert!(d.family.is_some(), "{:04x} has no family", d.pid);
            // Detect-only entries (HS80 cable with the headset off) declare no driver features.
            if d.features != ["presence"] {
                assert!(d.control.contains_key("command"), "{:04x} has no command collection", d.pid);
                assert_ne!(d.driver(), DeviceKind::Unsupported, "{:04x}", d.pid);
            }
        }
    }

    #[test]
    fn known_devices_map_to_drivers() {
        let db = Db::embedded();
        let k70 = db.get(0x1B55).unwrap();
        assert_eq!(k70.driver(), DeviceKind::LegacyKeyboard);
        assert_eq!(k70.control["command"], Usage { page: 0xFFC2, usage: 0x0004 });
        assert_eq!(k70.control["reply"], Usage { page: 0xFFC2, usage: 0x0003 });
        assert_eq!(k70.control["keys"], Usage { page: 0xFFC0, usage: 0x0002 });
        for pid in [0x0A6B, 0x0A69] {
            let hs = db.get(pid).unwrap();
            assert_eq!(hs.driver(), DeviceKind::BragiDongle);
            assert_eq!(hs.control["command"], Usage { page: 0xFF42, usage: 0x0001 });
            assert_eq!(hs.control["events"], Usage { page: 0xFF42, usage: 0x0002 });
        }
        assert_eq!(db.get(0x0A6A).unwrap().driver(), DeviceKind::Unsupported);
        assert!(db.get(0x0A6B).unwrap().pairs.contains(&0x0A69));
    }

    #[test]
    fn pairs_point_at_known_pids() {
        let db = Db::embedded();
        for d in &db.devices {
            for p in &d.pairs {
                assert!(db.get(*p).is_some(), "{:04x} pairs with unknown {p:04x}", d.pid);
            }
        }
    }

    #[test]
    fn merge_replaces_by_pid() {
        let mut db = Db::embedded();
        let n = db.devices.len();
        db.merge(Db::parse("[[device]]\npid = 0x1B55\nname = \"mine\"\nkind = \"keyboard\"\nstatus = \"untested\"\n").unwrap());
        assert_eq!(db.devices.len(), n);
        assert_eq!(db.get(0x1B55).unwrap().name, "mine");
    }

    #[test]
    fn udev_rules_cover_verified_devices_only() {
        let rules = udev_rules(&Db::embedded());
        for pid in ["1b55", "0a6b", "0a69", "0a6a"] {
            assert!(rules.contains(&format!("ATTRS{{idProduct}}==\"{pid}\"")), "{pid} missing");
        }
        assert!(!rules.contains("idProduct}}==\"1b49\""), "untested K70 RGB MK.2 must not get access");
        assert!(rules.contains("73-seat-late.rules"));
        assert!(rules.contains("KERNEL==\"uinput\""));
    }

    #[test]
    fn committed_udev_rules_are_up_to_date() {
        let committed = include_str!("../../../linux/70-openperipheral.rules");
        assert_eq!(committed, udev_rules(&Db::embedded()), "run `cargo run -p opctl -- udev-rules > linux/70-openperipheral.rules`");
    }
}
