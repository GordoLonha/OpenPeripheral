//! PC sensors for the Painel page, the Temperature effect and the CSV log.
//!
//! Linux: every `/sys/class/hwmon` chip (temperatures, fans, power), CPU load from
//! `/proc/stat`, memory from `/proc/meminfo`, GPU load and VRAM from the amdgpu sysfs files.
//! Windows: CPU load and memory (temperatures need a vendor driver there).

use serde::Serialize;
#[cfg(target_os = "linux")]
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Celsius,
    Rpm,
    Percent,
    Watt,
}

/// Where a sensor belongs on the Painel page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Board,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reading {
    /// Stable across reads, e.g. "k10temp/temp1".
    pub id: String,
    pub group: Group,
    /// The chip or device it comes from, e.g. "k10temp", "Samsung SSD 990 PRO".
    pub source: String,
    /// The sensor's own label, e.g. "Tctl", or "Temp 1".
    pub name: String,
    pub unit: Unit,
    pub value: f32,
}

/// CPU temperature for the Temperature effect: the package / control sensor of the CPU
/// chip if there is one, else its hottest sensor.
pub fn cpu_temp(readings: &[Reading]) -> Option<f32> {
    let cpu: Vec<&Reading> = readings.iter().filter(|r| r.group == Group::Cpu && r.unit == Unit::Celsius).collect();
    const MAIN: [&str; 5] = ["Tctl", "Tdie", "Package id 0", "CPU", "cpu"];
    cpu.iter()
        .find(|r| MAIN.iter().any(|m| r.name.starts_with(m)))
        .or_else(|| cpu.iter().max_by(|a, b| a.value.total_cmp(&b.value)))
        .map(|r| r.value)
}

/// Keeps the previous CPU counters, so load is measured between two reads.
#[derive(Default)]
pub struct Sensors {
    cpu_prev: Option<(u64, u64)>,
}

impl Sensors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self) -> Vec<Reading> {
        let mut out = Vec::new();
        if let Some((busy, total)) = cpu_times() {
            if let Some((pb, pt)) = self.cpu_prev {
                if total > pt {
                    let load = (busy.saturating_sub(pb)) as f32 / (total - pt) as f32 * 100.0;
                    out.push(reading("cpu/load", Group::Cpu, "Processador", "Uso", Unit::Percent, load.clamp(0.0, 100.0)));
                }
            }
            self.cpu_prev = Some((busy, total));
        }
        if let Some(used) = memory_used_percent() {
            out.push(reading("mem/used", Group::Memory, "Memória RAM", "Em uso", Unit::Percent, used));
        }
        #[cfg(target_os = "linux")]
        {
            out.extend(read_hwmon(Path::new("/sys/class/hwmon")));
            out.extend(read_gpu_load(Path::new("/sys/class/drm")));
        }
        out.sort_by(|a, b| a.group.cmp(&b.group).then_with(|| a.source.cmp(&b.source)).then_with(|| a.id.cmp(&b.id)));
        out
    }
}

fn reading(id: &str, group: Group, source: &str, name: &str, unit: Unit, value: f32) -> Reading {
    Reading { id: id.into(), group, source: source.into(), name: name.into(), unit, value }
}

/// (busy, total) jiffies from the first line of /proc/stat.
pub fn parse_proc_stat(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().next()?;
    let mut fields = line.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let v: Vec<u64> = fields.filter_map(|f| f.parse().ok()).collect();
    if v.len() < 4 {
        return None;
    }
    let total: u64 = v.iter().sum();
    // idle + iowait are not busy.
    let idle = v[3] + v.get(4).copied().unwrap_or(0);
    Some((total - idle, total))
}

/// Percent of memory in use from /proc/meminfo (MemTotal - MemAvailable).
pub fn parse_meminfo(text: &str) -> Option<f32> {
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
    };
    let (total, avail) = (field("MemTotal:")?, field("MemAvailable:")?);
    (total > 0.0).then(|| ((total - avail) / total * 100.0) as f32)
}

#[cfg(target_os = "linux")]
fn cpu_times() -> Option<(u64, u64)> {
    parse_proc_stat(&std::fs::read_to_string("/proc/stat").ok()?)
}

#[cfg(target_os = "linux")]
fn memory_used_percent() -> Option<f32> {
    parse_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(windows)]
fn cpu_times() -> Option<(u64, u64)> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: three valid out-pointers to FILETIME.
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let n = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    // Kernel time includes idle time.
    let total = n(kernel) + n(user);
    Some((total.saturating_sub(n(idle)), total))
}

#[cfg(windows)]
fn memory_used_percent() -> Option<f32> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: MEMORYSTATUSEX is plain data; dwLength is set before the call.
    let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 || m.ullTotalPhys == 0 {
        return None;
    }
    Some(((m.ullTotalPhys - m.ullAvailPhys) as f64 / m.ullTotalPhys as f64 * 100.0) as f32)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn cpu_times() -> Option<(u64, u64)> {
    None
}

#[cfg(not(any(target_os = "linux", windows)))]
fn memory_used_percent() -> Option<f32> {
    None
}

/// Group of a hwmon chip, by its driver name.
pub fn group_of(chip: &str) -> Group {
    match chip {
        "k10temp" | "zenpower" | "coretemp" | "cpu_thermal" | "fam15h_power" => Group::Cpu,
        "amdgpu" | "radeon" | "nouveau" | "i915" | "xe" => Group::Gpu,
        "spd5118" | "jc42" | "ee1004" => Group::Memory,
        "nvme" | "drivetemp" => Group::Storage,
        c if c.starts_with("nct") || c.starts_with("it8") || c.starts_with("asus") || c.starts_with("w83") || c == "acpitz" => Group::Board,
        _ => Group::Other,
    }
}

#[cfg(target_os = "linux")]
fn read_trimmed(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Every temperature, fan and power input under a hwmon root (normally /sys/class/hwmon).
#[cfg(target_os = "linux")]
pub fn read_hwmon(root: &Path) -> Vec<Reading> {
    let mut out = Vec::new();
    let Ok(dirs) = std::fs::read_dir(root) else { return out };
    let mut dirs: Vec<_> = dirs.flatten().map(|e| e.path()).collect();
    dirs.sort();
    // Same chip twice (three SSDs, two RAM sticks): number them.
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for dir in dirs {
        let Some(chip) = read_trimmed(&dir.join("name")) else { continue };
        let group = group_of(&chip);
        let n = seen.entry(chip.clone()).or_insert(0);
        *n += 1;
        let model = read_trimmed(&dir.join("device/model"));
        let source = match (&model, group) {
            (Some(m), _) => m.clone(),
            (None, Group::Memory) => format!("Pente {}", n),
            (None, Group::Storage) => format!("Disco {}", n),
            _ if *n > 1 => format!("{} {}", chip, n),
            _ => chip.clone(),
        };
        let Ok(files) = std::fs::read_dir(&dir) else { continue };
        let mut inputs: Vec<String> = files.flatten().filter_map(|e| e.file_name().into_string().ok()).collect();
        inputs.sort();
        for file in inputs {
            let (unit, kind, scale) = if file.starts_with("temp") && file.ends_with("_input") {
                (Unit::Celsius, "Temp", 1000.0)
            } else if file.starts_with("fan") && file.ends_with("_input") {
                (Unit::Rpm, "Ventoinha", 1.0)
            } else if file.starts_with("power") && (file.ends_with("_average") || file.ends_with("_input")) {
                (Unit::Watt, "Consumo", 1_000_000.0)
            } else {
                continue;
            };
            let Some(raw) = read_trimmed(&dir.join(&file)).and_then(|v| v.parse::<f64>().ok()) else { continue };
            let base = file.split('_').next().unwrap_or(&file).to_string();
            let index = base.trim_start_matches(|c: char| c.is_ascii_alphabetic()).to_string();
            let name = read_trimmed(&dir.join(format!("{base}_label"))).unwrap_or_else(|| format!("{kind} {index}"));
            let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("hwmon");
            out.push(Reading {
                id: format!("{chip}/{dir_name}/{base}"),
                group,
                source: source.clone(),
                name,
                unit,
                value: (raw / scale) as f32,
            });
        }
    }
    out
}

/// GPU load and VRAM use from amdgpu's sysfs files (one entry per card that has them).
#[cfg(target_os = "linux")]
pub fn read_gpu_load(drm: &Path) -> Vec<Reading> {
    let mut out = Vec::new();
    let Ok(cards) = std::fs::read_dir(drm) else { return out };
    let mut cards: Vec<_> = cards
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("card") && !n.contains('-')))
        .collect();
    cards.sort();
    for card in cards {
        let dev = card.join("device");
        let name = card.file_name().and_then(|n| n.to_str()).unwrap_or("card").to_string();
        if !dev.join("gpu_busy_percent").exists() {
            continue;
        }
        let source = "amdgpu";
        if let Some(busy) = read_trimmed(&dev.join("gpu_busy_percent")).and_then(|v| v.parse::<f32>().ok()) {
            out.push(reading(&format!("{name}/busy"), Group::Gpu, source, "Uso", Unit::Percent, busy));
        }
        let vram = |f: &str| read_trimmed(&dev.join(f)).and_then(|v| v.parse::<f64>().ok());
        if let (Some(used), Some(total)) = (vram("mem_info_vram_used"), vram("mem_info_vram_total")) {
            if total > 0.0 {
                out.push(reading(&format!("{name}/vram"), Group::Gpu, source, "Memória de vídeo", Unit::Percent, (used / total * 100.0) as f32));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_busy_and_total() {
        let text = "cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 1 2 3 4\n";
        assert_eq!(parse_proc_stat(text), Some((150, 1000)));
        assert_eq!(parse_proc_stat("intr 1 2"), None);
    }

    #[test]
    fn meminfo_percent() {
        let text = "MemTotal:       32000000 kB\nMemFree:  1000 kB\nMemAvailable:   24000000 kB\n";
        assert_eq!(parse_meminfo(text), Some(25.0));
        assert_eq!(parse_meminfo("MemTotal: 0 kB\nMemAvailable: 0 kB"), None);
    }

    #[test]
    fn chips_are_grouped() {
        assert_eq!(group_of("k10temp"), Group::Cpu);
        assert_eq!(group_of("amdgpu"), Group::Gpu);
        assert_eq!(group_of("spd5118"), Group::Memory);
        assert_eq!(group_of("nvme"), Group::Storage);
        assert_eq!(group_of("nct6799"), Group::Board);
        assert_eq!(group_of("mt7925_phy0"), Group::Other);
    }

    #[test]
    fn cpu_temp_prefers_the_control_sensor() {
        let r = |name: &str, v: f32| reading(name, Group::Cpu, "k10temp", name, Unit::Celsius, v);
        assert_eq!(cpu_temp(&[r("Tccd1", 70.0), r("Tctl", 55.0)]), Some(55.0));
        assert_eq!(cpu_temp(&[r("Tccd1", 44.0), r("Tccd2", 56.0)]), Some(56.0));
        assert_eq!(cpu_temp(&[]), None);
    }

    /// `cargo test -p openperipheral-app sensors::tests::show -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn show() {
        let mut s = Sensors::new();
        s.read();
        std::thread::sleep(std::time::Duration::from_millis(500));
        for r in s.read() {
            println!("{:?} {} · {} = {:.1} {:?}", r.group, r.source, r.name, r.value, r.unit);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reads_a_fake_hwmon_tree() {
        let root = std::env::temp_dir().join(format!("op-hwmon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |dir: &str, files: &[(&str, &str)]| {
            let d = root.join(dir);
            std::fs::create_dir_all(&d).unwrap();
            for (f, v) in files {
                std::fs::write(d.join(f), v).unwrap();
            }
        };
        mk("hwmon0", &[("name", "k10temp\n"), ("temp1_input", "63125\n"), ("temp1_label", "Tctl\n"), ("temp3_input", "44125\n")]);
        mk("hwmon1", &[("name", "amdgpu\n"), ("temp1_input", "39000\n"), ("temp1_label", "edge\n"), ("fan1_input", "1200\n"), ("power1_average", "45000000\n")]);
        mk("hwmon2", &[("name", "nvme\n"), ("temp1_input", "31850\n")]);
        mk("hwmon3", &[("name", "nvme\n"), ("temp1_input", "41850\n")]);
        mk("hwmon4", &[("name", "asus\n")]);
        let r = read_hwmon(&root);
        let _ = std::fs::remove_dir_all(&root);

        let find = |id: &str| r.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("{id} in {r:?}"));
        let tctl = find("k10temp/hwmon0/temp1");
        assert_eq!((tctl.name.as_str(), tctl.value, tctl.group), ("Tctl", 63.125, Group::Cpu));
        assert_eq!(find("k10temp/hwmon0/temp3").name, "Temp 3");
        assert_eq!(find("amdgpu/hwmon1/fan1").unit, Unit::Rpm);
        assert_eq!(find("amdgpu/hwmon1/power1").value, 45.0);
        assert_eq!(find("nvme/hwmon2/temp1").source, "Disco 1");
        assert_eq!(find("nvme/hwmon3/temp1").source, "Disco 2");
        assert_eq!(r.len(), 7, "chips without inputs add nothing");
        assert_eq!(cpu_temp(&r), Some(63.125));
    }
}
