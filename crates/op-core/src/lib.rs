//! OpenPeripheral core: userspace HID drivers for Corsair peripherals.
//!
//! No kernel driver is needed: every device here is HID, reached through
//! hidapi on Windows (HID class driver) and Linux (hidraw).

pub mod color;
pub mod descriptor;
pub mod error;
pub mod hs80;
pub mod k70;
pub mod m65;
pub mod registry;

pub use color::Rgb;
pub use error::{Error, Result};
pub use registry::{enumerate, DeviceKind, FoundDevice, CORSAIR_VID};
