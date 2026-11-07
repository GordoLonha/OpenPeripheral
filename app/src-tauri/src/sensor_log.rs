//! Sensor log: chosen readings written to a CSV file, one row every few sensor reads,
//! until stopped or for a set number of minutes.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::sensors::{Reading, Unit};

/// One column, as the page names it (friendly name, chosen unit).
#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    pub columns: Vec<Column>,
    /// Write a row every this many sensor reads (1 = every read).
    pub every: u32,
    /// Stop by itself after this many minutes; None = until stopped.
    pub minutes: Option<u32>,
    /// Temperatures in °F instead of °C.
    pub fahrenheit: bool,
}

/// What the page shows.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct State {
    pub running: bool,
    /// The file being written, or the last one written.
    pub path: Option<String>,
    pub rows: u32,
    /// Why the last log stopped, when it was not by hand or by time.
    pub error: Option<String>,
}

pub struct SensorLog {
    out: BufWriter<File>,
    path: PathBuf,
    columns: Vec<Column>,
    fahrenheit: bool,
    every: u32,
    reads: u32,
    started: Instant,
    until: Option<Instant>,
    rows: u32,
}

impl SensorLog {
    /// Creates `dir/sensores-<date>_<time>.csv` and writes the header.
    pub fn start(dir: &Path, req: Request, stamp: &str, now: Instant) -> io::Result<Self> {
        if req.columns.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "nenhum sensor para gravar"));
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("sensores-{stamp}.csv"));
        let mut out = BufWriter::new(File::create(&path)?);
        writeln!(out, "{}", header(&req.columns))?;
        out.flush()?;
        Ok(Self {
            out,
            path,
            columns: req.columns,
            fahrenheit: req.fahrenheit,
            every: req.every.max(1),
            reads: 0,
            started: now,
            until: req.minutes.map(|m| now + Duration::from_secs(u64::from(m) * 60)),
            rows: 0,
        })
    }

    /// Called on every sensor read. Ok(false) once the set time is over.
    pub fn take(&mut self, readings: &[Reading], now: Instant, clock: &str) -> io::Result<bool> {
        if self.until.is_some_and(|u| now >= u) {
            return Ok(false);
        }
        self.reads += 1;
        if !(self.reads - 1).is_multiple_of(self.every) {
            return Ok(true);
        }
        let secs = (now - self.started).as_secs_f32();
        writeln!(self.out, "{}", row(&self.columns, readings, clock, secs, self.fahrenheit))?;
        // Flushed every row: the file is complete even if the app is closed mid-log.
        self.out.flush()?;
        self.rows += 1;
        Ok(true)
    }

    pub fn state(&self) -> State {
        State { running: true, path: Some(self.path.display().to_string()), rows: self.rows, error: None }
    }
}

/// CSV field: always quoted, quotes doubled, line breaks turned into spaces.
fn quote(s: &str) -> String {
    let clean: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    format!("\"{}\"", clean.replace('"', "\"\""))
}

fn header(columns: &[Column]) -> String {
    let mut fields = vec![quote("Hora"), quote("Segundos")];
    fields.extend(columns.iter().map(|c| quote(&c.label)));
    fields.join(",")
}

/// One row; a sensor that is gone (e.g. a drive unplugged) leaves its field empty.
fn row(columns: &[Column], readings: &[Reading], clock: &str, secs: f32, fahrenheit: bool) -> String {
    let mut fields = vec![quote(clock), format!("{secs:.1}")];
    fields.extend(columns.iter().map(|c| {
        readings.iter().find(|r| r.id == c.id).map_or_else(String::new, |r| {
            let v = if fahrenheit && r.unit == Unit::Celsius { r.value * 9.0 / 5.0 + 32.0 } else { r.value };
            match r.unit {
                Unit::Celsius => format!("{v:.1}"),
                _ => format!("{v:.0}"),
            }
        })
    }));
    fields.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::Group;

    fn reading(id: &str, unit: Unit, value: f32) -> Reading {
        Reading { id: id.into(), group: Group::Cpu, source: "k10temp".into(), name: "Tctl".into(), unit, value }
    }

    fn col(id: &str, label: &str) -> Column {
        Column { id: id.into(), label: label.into() }
    }

    #[test]
    fn header_quotes_names() {
        let h = header(&[col("a", "Processador \"Tctl\" (°C)"), col("b", "Uso\n(%)")]);
        assert_eq!(h, r#""Hora","Segundos","Processador ""Tctl"" (°C)","Uso (%)""#);
    }

    #[test]
    fn row_values_and_missing_sensor() {
        let cols = [col("t", "T"), col("load", "Uso"), col("gone", "SSD")];
        let rs = [reading("t", Unit::Celsius, 62.26), reading("load", Unit::Percent, 13.6)];
        assert_eq!(row(&cols, &rs, "14:03:05", 1.5, false), r#""14:03:05",1.5,62.3,14,"#);
        // Fahrenheit converts temperatures only.
        assert_eq!(row(&cols, &rs, "14:03:05", 3.0, true), r#""14:03:05",3.0,144.1,14,"#);
    }

    #[test]
    fn writes_every_nth_read_and_stops_on_time() {
        let dir = std::env::temp_dir().join(format!("op-sensor-log-{}", std::process::id()));
        let t0 = Instant::now();
        let req = Request { columns: vec![col("t", "T")], every: 2, minutes: Some(1), fahrenheit: false };
        let mut log = SensorLog::start(&dir, req, "test", t0).unwrap();
        let rs = [reading("t", Unit::Celsius, 50.0)];
        for i in 0..5 {
            assert!(log.take(&rs, t0 + Duration::from_millis(1500 * i), "x").unwrap());
        }
        assert_eq!(log.rows, 3); // reads 1, 3, 5
        assert!(!log.take(&rs, t0 + Duration::from_secs(60), "x").unwrap());
        let text = std::fs::read_to_string(&log.path).unwrap();
        assert_eq!(text.lines().count(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_a_column() {
        let req = Request { columns: vec![], every: 1, minutes: None, fahrenheit: false };
        assert!(SensorLog::start(&std::env::temp_dir(), req, "x", Instant::now()).is_err());
    }
}
