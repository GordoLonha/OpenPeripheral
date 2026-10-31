//! Headset equaliser. The HS80 has no on-device EQ (iCUE's EQ is host-side DSP,
//! confirmed by capture), so we drive a system-wide parametric EQ:
//! - Windows: Equalizer APO, via an included config file limited to the HS80.
//! - Linux: a PipeWire filter-chain sink in front of the HS80, run as our own
//!   `pipewire` child process and made the default output while enabled.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

pub const FREQS: [f32; 10] = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
/// Q for one-octave-spaced peaking bands.
const Q: f32 = 1.41;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Eq {
    pub enabled: bool,
    pub preset: String,
    /// Gain in dB per band in `FREQS`, -12..=12.
    pub gains: [f32; 10],
}

impl Default for Eq {
    fn default() -> Self {
        Self { enabled: false, preset: "flat".into(), gains: [0.0; 10] }
    }
}

/// Same list and order as the UI (Equalizer.tsx); the headset wheel cycles it.
pub const PRESETS: [(&str, [f32; 10]); 6] = [
    ("flat", [0.0; 10]),
    ("bass", [6.0, 5.0, 3.5, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("fps", [-3.0, -2.0, -1.0, 0.0, 0.0, 1.5, 3.5, 4.5, 3.0, 0.0]),
    ("movie", [4.0, 3.0, 1.5, 0.0, -1.0, 0.0, 1.5, 2.5, 2.0, 1.0]),
    ("music", [4.5, 3.5, 1.5, 0.0, -1.5, -1.5, 0.0, 1.5, 3.0, 3.5]),
    ("voice", [-4.0, -3.0, -1.5, 0.0, 1.5, 3.0, 3.0, 1.5, 0.0, -1.5]),
];

impl Eq {
    /// Next preset after the current one (a custom curve goes to the first),
    /// switching the EQ on.
    pub fn next_preset(&self) -> Eq {
        let i = PRESETS.iter().position(|(id, _)| *id == self.preset).map_or(0, |i| (i + 1) % PRESETS.len());
        let (id, gains) = PRESETS[i];
        Eq { enabled: true, preset: id.into(), gains }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Backend {
    /// "equalizer-apo", "pipewire" or "none".
    pub kind: &'static str,
    pub available: bool,
    pub detail: String,
}

#[cfg(windows)]
mod apo {
    use super::*;

    const MANAGED: &str = "openperipheral.txt";

    pub fn config_dir() -> Option<PathBuf> {
        let dir = PathBuf::from(r"C:\Program Files\EqualizerAPO\config");
        dir.join("config.txt").exists().then_some(dir)
    }

    pub fn render(eq: &Eq) -> String {
        let mut s = String::from("# Managed by OpenPeripheral - edits here are overwritten.\nDevice: HS80\n");
        if eq.enabled {
            // Pre-attenuate by the largest boost so boosted bands cannot clip.
            let peak = eq.gains.iter().cloned().fold(0.0f32, f32::max);
            s += &format!("Preamp: {:.1} dB\n", -peak);
            for (f, g) in FREQS.iter().zip(eq.gains) {
                s += &format!("Filter: ON PK Fc {f} Hz Gain {g:.1} dB Q {Q}\n");
            }
        }
        s
    }

    pub fn apply(eq: &Eq) -> Result<(), String> {
        let dir = config_dir().ok_or("Equalizer APO não encontrado")?;
        fs::write(dir.join(MANAGED), render(eq)).map_err(|e| e.to_string())?;
        // Make sure the main config includes our file (once). The first time,
        // keep a backup of the user's config and drop APO's demo include.
        let main = dir.join("config.txt");
        let cfg = fs::read_to_string(&main).unwrap_or_default();
        if !cfg.contains(MANAGED) {
            let _ = fs::write(dir.join("config.before-openperipheral.txt"), &cfg);
            let kept: String = cfg
                .lines()
                .filter(|l| !l.trim_start().starts_with("Include: example.txt"))
                .map(|l| format!("{l}\n"))
                .collect();
            fs::write(&main, format!("{kept}Include: {MANAGED}\n")).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}


#[cfg(target_os = "linux")]
mod pw {
    use super::*;
    use std::{
        os::unix::process::CommandExt,
        process::{Child, Command, Stdio},
        sync::Mutex,
        thread,
        time::{Duration, Instant},
    };

    use serde_json::Value;

    /// node.name of the EQ sink apps play into.
    pub const NODE: &str = "openperipheral_eq";

    struct Running {
        child: Child,
        /// node.name of the HS80 sink the EQ plays into.
        target: String,
    }

    static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

    fn dump() -> Vec<Value> {
        Command::new("pw-dump")
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| serde_json::from_slice(&o.stdout).ok())
            .unwrap_or_default()
    }

    fn props(o: &Value) -> &Value {
        &o["info"]["props"]
    }

    /// (id, node.name) of the first audio sink whose name matches.
    fn find_sink(objs: &[Value], pred: impl Fn(&str) -> bool) -> Option<(u64, String)> {
        objs.iter().find_map(|o| {
            let p = props(o);
            let name = p["node.name"].as_str()?;
            (p["media.class"] == "Audio/Sink" && pred(name)).then(|| (o["id"].as_u64().unwrap_or(0), name.to_string()))
        })
    }

    pub fn hs80_sink(objs: &[Value]) -> Option<(u64, String)> {
        find_sink(objs, |n| n.contains("HS80") && n != NODE)
    }

    fn default_sink(objs: &[Value]) -> Option<String> {
        objs.iter()
            .filter(|o| o["props"]["metadata.name"] == "default")
            .flat_map(|o| o["metadata"].as_array().cloned().unwrap_or_default())
            .find(|m| m["key"] == "default.audio.sink")
            .and_then(|m| m["value"]["name"].as_str().map(str::to_string))
    }

    fn set_default(id: u64) {
        let _ = Command::new("wpctl").args(["set-default", &id.to_string()]).status();
    }

    pub fn preamp(eq: &Eq) -> f32 {
        // Pre-attenuate by the largest boost so boosted bands cannot clip.
        let peak = eq.gains.iter().cloned().fold(0.0f32, f32::max);
        10f32.powf(-peak / 20.0)
    }

    pub fn render(eq: &Eq, target: &str) -> String {
        let mut nodes = format!(
            "          {{ type = builtin name = pre label = mixer control = {{ \"Gain 1\" = {:.4} }} }}\n",
            preamp(eq)
        );
        let mut links = String::from("          { output = \"pre:Out\" input = \"band1:In\" }\n");
        for (i, (f, g)) in FREQS.iter().zip(eq.gains).enumerate() {
            let n = i + 1;
            nodes += &format!(
                "          {{ type = builtin name = band{n} label = bq_peaking control = {{ \"Freq\" = {f} \"Q\" = {Q} \"Gain\" = {g:.1} }} }}\n"
            );
            if n < FREQS.len() {
                links += &format!("          {{ output = \"band{n}:Out\" input = \"band{}:In\" }}\n", n + 1);
            }
        }
        let target = target.replace('"', "");
        format!(
            r#"# Generated by OpenPeripheral; rewritten on every start.
context.properties = {{ log.level = 0 }}
context.spa-libs = {{
    audio.convert.* = audioconvert/libspa-audioconvert
    support.*       = support/libspa-support
}}
context.modules = [
    {{ name = libpipewire-module-rt flags = [ ifexists nofail ] }}
    {{ name = libpipewire-module-protocol-native }}
    {{ name = libpipewire-module-client-node }}
    {{ name = libpipewire-module-adapter }}
    {{ name = libpipewire-module-filter-chain
      args = {{
        node.description = "HS80 + Equalizador (OpenPeripheral)"
        media.name       = "HS80 + Equalizador (OpenPeripheral)"
        filter.graph = {{
          nodes = [
{nodes}          ]
          links = [
{links}          ]
          inputs  = [ "pre:In 1" ]
          outputs = [ "band{last}:Out" ]
        }}
        audio.channels = 2
        audio.position = [ FL FR ]
        capture.props = {{
          node.name   = "{NODE}"
          media.class = Audio/Sink
        }}
        playback.props = {{
          node.name        = "{NODE}.output"
          node.passive     = true
          target.object    = "{target}"
          node.dont-reconnect = true
          stream.dont-remix   = true
        }}
      }}
    }}
]
"#,
            last = FREQS.len()
        )
    }

    /// Change gains on the running filter without restarting it (no dropout).
    fn set_controls(eq: &Eq) -> Result<(), String> {
        let id = find_sink(&dump(), |n| n == NODE).ok_or("filtro do EQ não encontrado")?.0;
        let mut params = format!("\"pre:Gain 1\" {:.4}", preamp(eq));
        for (i, g) in eq.gains.iter().enumerate() {
            params += &format!(" \"band{}:Gain\" {g:.1}", i + 1);
        }
        let ok = Command::new("pw-cli")
            .args(["set-param", &id.to_string(), "Props", &format!("{{ params = [ {params} ] }}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok { Ok(()) } else { Err("pw-cli set-param falhou".into()) }
    }

    fn spawn(eq: &Eq, target: &str) -> Result<Child, String> {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let conf = dir.join("openperipheral-eq.conf");
        fs::write(&conf, render(eq, target)).map_err(|e| e.to_string())?;
        let mut cmd = Command::new("pipewire");
        cmd.arg("-c").arg(&conf).stdout(Stdio::null()).stderr(Stdio::null());
        // Die with the app, even if it crashes, so the EQ sink never lingers.
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
        cmd.spawn().map_err(|e| format!("não foi possível iniciar o pipewire: {e}"))
    }

    fn stop_locked(slot: &mut Option<Running>) {
        let Some(mut r) = slot.take() else { return };
        // Hand the default output back to the headset before the EQ sink goes.
        let objs = dump();
        if default_sink(&objs).as_deref() == Some(NODE) {
            if let Some((id, _)) = find_sink(&objs, |n| n == r.target) {
                set_default(id);
            }
        }
        let _ = r.child.kill();
        let _ = r.child.wait();
    }

    pub fn stop() {
        if let Ok(mut slot) = RUNNING.lock() {
            stop_locked(&mut slot);
        }
    }

    pub fn apply(eq: &Eq) -> Result<(), String> {
        let mut slot = RUNNING.lock().map_err(|e| e.to_string())?;
        if !eq.enabled {
            stop_locked(&mut slot);
            return Ok(());
        }
        let objs = dump();
        let (hs_id, target) = hs80_sink(&objs).ok_or("HS80 não encontrado no PipeWire (receptor conectado?)")?;
        if let Some(r) = slot.as_mut() {
            let alive = matches!(r.child.try_wait(), Ok(None));
            if alive && r.target == target && set_controls(eq).is_ok() {
                return Ok(());
            }
        }
        stop_locked(&mut slot);
        let child = spawn(eq, &target)?;
        *slot = Some(Running { child, target });

        // Route sound through the EQ: take over as default while the HS80 is it.
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            let objs = dump();
            if let Some((eq_id, _)) = find_sink(&objs, |n| n == NODE) {
                if default_sink(&objs).is_none_or(|d| d.contains("HS80")) {
                    set_default(eq_id);
                }
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        let _ = hs_id;
        Err("o filtro do EQ não apareceu no PipeWire".into())
    }

    pub fn available() -> Result<String, String> {
        if Command::new("pw-cli").arg("--version").output().is_err() {
            return Err("PipeWire não encontrado.".into());
        }
        match hs80_sink(&dump()) {
            Some((_, name)) => Ok(name),
            None => Err("HS80 não encontrado no PipeWire (receptor conectado?).".into()),
        }
    }
}

/// Remove the EQ sink (Linux) on exit.
pub fn shutdown() {
    #[cfg(target_os = "linux")]
    pw::stop();
}

pub fn backend() -> Backend {
    #[cfg(windows)]
    {
        match apo::config_dir() {
            Some(d) => Backend { kind: "equalizer-apo", available: true, detail: d.display().to_string() },
            None => Backend {
                kind: "equalizer-apo",
                available: false,
                detail: "Instale o Equalizer APO e selecione o HS80 no instalador.".into(),
            },
        }
    }
    #[cfg(target_os = "linux")]
    {
        match pw::available() {
            Ok(sink) => Backend { kind: "pipewire", available: true, detail: sink },
            Err(detail) => Backend { kind: "pipewire", available: false, detail },
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Backend { kind: "none", available: false, detail: "Sem EQ neste sistema.".into() }
    }
}

pub fn apply(eq: &Eq) -> Result<(), String> {
    #[cfg(windows)]
    {
        apo::apply(eq)
    }
    #[cfg(target_os = "linux")]
    {
        pw::apply(eq)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = eq;
        Err("Sem EQ neste sistema".into())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn apo_config_has_preamp_and_bands() {
        let mut eq = Eq { enabled: true, ..Default::default() };
        eq.gains[0] = 6.0;
        eq.gains[9] = -3.0;
        let s = apo::render(&eq);
        assert!(s.contains("Device: HS80"));
        assert!(s.contains("Preamp: -6.0 dB"));
        assert!(s.contains("Filter: ON PK Fc 32 Hz Gain 6.0 dB Q 1.41"));
        assert_eq!(s.matches("Filter:").count(), 10);
        let off = apo::render(&Eq::default());
        assert!(!off.contains("Filter:"));
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn wheel_cycles_presets() {
        let mut eq = Eq::default();
        let mut seen = vec![];
        for _ in 0..PRESETS.len() {
            eq = eq.next_preset();
            assert!(eq.enabled);
            seen.push(eq.preset.clone());
        }
        assert_eq!(seen, ["bass", "fps", "movie", "music", "voice", "flat"]);
        let custom = Eq { enabled: true, preset: "custom".into(), gains: [1.0; 10] };
        assert_eq!(custom.next_preset().preset, "flat");
    }
}

#[cfg(all(test, target_os = "linux"))]
mod pw_tests {
    use super::*;

    #[test]
    fn filter_chain_config() {
        let mut eq = Eq { enabled: true, ..Default::default() };
        eq.gains[0] = 6.0;
        let s = pw::render(&eq, "alsa_output.hs80");
        assert_eq!(s.matches("label = bq_peaking").count(), 10);
        assert!(s.contains("\"Freq\" = 32 \"Q\" = 1.41 \"Gain\" = 6.0"));
        assert!(s.contains("target.object    = \"alsa_output.hs80\""));
        assert!(s.contains("outputs = [ \"band10:Out\" ]"));
        assert!((pw::preamp(&eq) - 0.5012).abs() < 1e-3);
    }
}
