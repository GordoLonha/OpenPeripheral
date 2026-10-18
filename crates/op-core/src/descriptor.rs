//! Minimal HID report-descriptor parser: enough to learn which report IDs a
//! collection uses and how long its input/output/feature reports are.

use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReportSizes {
    /// Payload length in bytes, excluding the report-ID byte.
    pub input: usize,
    pub output: usize,
    pub feature: usize,
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorInfo {
    /// First usage page seen (the top-level collection's page).
    pub usage_page: u16,
    /// Report ID (0 = descriptor uses no report IDs) -> sizes.
    pub reports: BTreeMap<u8, ReportSizes>,
}

pub fn parse(desc: &[u8]) -> DescriptorInfo {
    let mut info = DescriptorInfo::default();
    let mut bits: BTreeMap<u8, (usize, usize, usize)> = BTreeMap::new();
    let (mut report_id, mut report_size, mut report_count) = (0u8, 0usize, 0usize);
    let mut stack: Vec<(u8, usize, usize)> = Vec::new();
    let mut i = 0;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // long item: skip
            let len = *desc.get(i + 1).unwrap_or(&0) as usize;
            i += 3 + len;
            continue;
        }
        let size = match prefix & 3 {
            3 => 4,
            n => n as usize,
        };
        let mut val = 0u32;
        for k in 0..size {
            val |= (*desc.get(i + 1 + k).unwrap_or(&0) as u32) << (8 * k);
        }
        match prefix & 0xFC {
            0x04 if info.usage_page == 0 => info.usage_page = val as u16, // Usage Page
            0x74 => report_size = val as usize,                           // Report Size
            0x94 => report_count = val as usize,                          // Report Count
            0x84 => report_id = val as u8,                                // Report ID
            0xA4 => stack.push((report_id, report_size, report_count)),   // Push
            0xB4 => {
                if let Some(s) = stack.pop() {
                    (report_id, report_size, report_count) = s;
                }
            }
            0x80 | 0x90 | 0xB0 => {
                let e = bits.entry(report_id).or_default();
                let n = report_size * report_count;
                match prefix & 0xFC {
                    0x80 => e.0 += n,
                    0x90 => e.1 += n,
                    _ => e.2 += n,
                }
            }
            _ => {}
        }
        i += 1 + size;
    }
    info.reports = bits
        .into_iter()
        .map(|(id, (a, b, c))| (id, ReportSizes { input: a.div_ceil(8), output: b.div_ceil(8), feature: c.div_ceil(8) }))
        .collect();
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_collection_without_ids() {
        // Usage Page 0xFFC2, Usage 4, Collection, 64 in + 64 out bytes, End
        let d = [
            0x06, 0xC2, 0xFF, 0x09, 0x04, 0xA1, 0x01, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x75, 0x08, 0x95, 0x40, 0x09,
            0x01, 0x81, 0x02, 0x95, 0x40, 0x09, 0x02, 0x91, 0x02, 0xC0,
        ];
        let info = parse(&d);
        assert_eq!(info.usage_page, 0xFFC2);
        assert_eq!(info.reports[&0], ReportSizes { input: 64, output: 64, feature: 0 });
    }

    #[test]
    fn report_ids_are_split() {
        let d = [0x06, 0x42, 0xFF, 0xA1, 0x01, 0x85, 0x02, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x02, 0x85, 0x03, 0x95, 0x20,
            0x81, 0x02, 0xC0];
        let info = parse(&d);
        assert_eq!(info.reports[&2].output, 63);
        assert_eq!(info.reports[&3].input, 32);
    }
}
