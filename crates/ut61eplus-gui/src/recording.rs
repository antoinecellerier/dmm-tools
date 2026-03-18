use chrono::{DateTime, Local};
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

/// A single recorded sample.
#[derive(Debug, Clone)]
pub struct Sample {
    pub wall_time: DateTime<Local>,
    pub mode: String,
    pub value_str: String,
    pub value_f64: Option<f64>,
    pub unit: String,
    pub range_label: String,
    pub flags: String,
}

impl Sample {
    pub fn from_measurement(m: &Measurement) -> Self {
        let (value_str, value_f64) = match &m.value {
            MeasuredValue::Normal(v) => (format!("{v}"), Some(*v)),
            MeasuredValue::Overload => ("OL".to_string(), None),
            MeasuredValue::NcvLevel(l) => (format!("NCV:{l}"), None),
        };
        Self {
            wall_time: Local::now(),
            mode: m.mode.to_string(),
            value_str,
            value_f64,
            unit: m.unit.clone(),
            range_label: m.range_label.clone(),
            flags: m.flags.to_string(),
        }
    }
}

/// In-memory recording buffer.
#[derive(Debug)]
pub struct Recording {
    pub active: bool,
    pub samples: Vec<Sample>,
    pub start_time: Option<DateTime<Local>>,
}

impl Recording {
    pub fn new() -> Self {
        Self {
            active: false,
            samples: Vec::new(),
            start_time: None,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if self.active {
            self.samples.clear();
            self.start_time = Some(Local::now());
        }
    }

    pub fn push(&mut self, m: &Measurement) {
        if self.active {
            self.samples.push(Sample::from_measurement(m));
        }
    }

    pub fn duration_secs(&self) -> f64 {
        self.start_time
            .map(|start| (Local::now() - start).num_milliseconds() as f64 / 1000.0)
            .unwrap_or(0.0)
    }

    pub fn export_csv(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_path(path)?;
        wtr.write_record(["timestamp", "mode", "value", "unit", "range", "flags"])?;
        for s in &self.samples {
            wtr.write_record([
                &s.wall_time.to_rfc3339(),
                &s.mode,
                &s.value_str,
                &s.unit,
                &s.range_label,
                &s.flags,
            ])?;
        }
        wtr.flush()?;
        Ok(())
    }
}

impl Default for Recording {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ut61eplus_lib::measurement::Measurement;
    use ut61eplus_lib::tables::ut61e_plus::Ut61ePlusTable;

    fn make_measurement(display: &[u8; 7]) -> Measurement {
        let payload: Vec<u8> = vec![
            0x02,           // mode: DcV (raw, no 0x30)
            0x31,           // range: 1 (with 0x30 prefix)
            display[0], display[1], display[2], display[3],
            display[4], display[5], display[6],
            0x00, 0x00,     // progress (raw)
            0x30, 0x30, 0x30, // flags (with 0x30 prefix, all zero = AUTO on)
        ];
        let table = Ut61ePlusTable::new();
        Measurement::parse(&payload, &table).unwrap()
    }

    #[test]
    fn recording_inactive_by_default() {
        let r = Recording::new();
        assert!(!r.active);
        assert!(r.samples.is_empty());
    }

    #[test]
    fn recording_toggle_starts_and_stops() {
        let mut r = Recording::new();
        r.toggle();
        assert!(r.active);
        assert!(r.start_time.is_some());
        r.toggle();
        assert!(!r.active);
    }

    #[test]
    fn recording_only_captures_when_active() {
        let mut r = Recording::new();
        let m = make_measurement(b"  1.234");
        r.push(&m);
        assert!(r.samples.is_empty());

        r.toggle(); // start
        r.push(&m);
        assert_eq!(r.samples.len(), 1);
    }

    #[test]
    fn recording_toggle_clears_previous() {
        let mut r = Recording::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        r.push(&m);
        r.push(&m);
        assert_eq!(r.samples.len(), 2);

        r.toggle(); // stop
        r.toggle(); // start again — should clear
        assert!(r.samples.is_empty());
    }

    #[test]
    fn sample_from_measurement() {
        let m = make_measurement(b"  5.678");
        let s = Sample::from_measurement(&m);
        assert_eq!(s.mode, "DC V");
        assert_eq!(s.value_str, "5.678");
        assert_eq!(s.value_f64, Some(5.678));
        assert_eq!(s.unit, "V");
    }

    #[test]
    fn export_csv_roundtrip() {
        let mut r = Recording::new();
        r.toggle();
        let m = make_measurement(b"  5.678");
        r.push(&m);
        r.push(&m);

        let dir = std::env::temp_dir();
        let path = dir.join("ut61eplus_test_export.csv");
        r.export_csv(&path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 samples
        assert!(lines[0].starts_with("timestamp,"));
        assert!(lines[1].contains("5.678"));

        let _ = std::fs::remove_file(&path);
    }
}
