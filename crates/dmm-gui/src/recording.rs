use chrono::{DateTime, Local};
use dmm_lib::measurement::{MeasuredValue, Measurement};

/// Maximum recording samples (~14 hours at 10Hz, ~22MB memory).
const MAX_RECORDING_SAMPLES: usize = 500_000;

/// A single recorded sample.
#[derive(Debug, Clone)]
pub struct Sample {
    pub wall_time: DateTime<Local>,
    pub mode: String,
    pub value_str: String,
    pub unit: String,
    pub range_label: String,
    pub flags: String,
}

impl Sample {
    pub fn from_measurement(m: &Measurement) -> Self {
        let value_str = if let Some(raw) = &m.display_raw {
            raw.trim().to_string()
        } else {
            match &m.value {
                MeasuredValue::Normal(v) => format!("{v}"),
                MeasuredValue::Overload => "OL".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
            }
        };
        Self {
            wall_time: Local::now(),
            mode: m.mode.to_string(),
            value_str,
            unit: m.unit.to_string(),
            range_label: m.range_label.to_string(),
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

    /// Push a sample. Returns `true` if the buffer just became full (auto-stops recording).
    pub fn push(&mut self, m: &Measurement) -> bool {
        if self.active && self.samples.len() < MAX_RECORDING_SAMPLES {
            self.samples.push(Sample::from_measurement(m));
            if self.samples.len() >= MAX_RECORDING_SAMPLES {
                self.active = false;
                return true;
            }
        }
        false
    }

    pub fn is_full(&self) -> bool {
        self.samples.len() >= MAX_RECORDING_SAMPLES
    }

    pub fn duration_secs(&self) -> f64 {
        self.start_time
            .map(|start| (Local::now() - start).num_milliseconds() as f64 / 1000.0)
            .unwrap_or(0.0)
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
    use dmm_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;

    fn make_measurement(display: &[u8; 7]) -> Measurement {
        let payload: Vec<u8> = vec![
            0x02, // mode: DcV (raw, no 0x30)
            0x31, // range: 1 (with 0x30 prefix)
            display[0], display[1], display[2], display[3], display[4], display[5], display[6],
            0x00, 0x00, // progress (raw)
            0x30, 0x30, 0x30, // flags (with 0x30 prefix, all zero = AUTO on)
        ];
        let table = Ut61ePlusTable::new();
        dmm_lib::protocol::ut61eplus::parse_measurement(&payload, &table).unwrap()
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
    fn recording_auto_stops_when_full() {
        let mut r = Recording::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        // Fill to one below capacity
        for _ in 0..MAX_RECORDING_SAMPLES - 1 {
            assert!(!r.push(&m));
            assert!(r.active);
        }
        // The push that hits capacity should auto-stop and return true
        assert!(r.push(&m));
        assert!(!r.active);
        assert_eq!(r.samples.len(), MAX_RECORDING_SAMPLES);
        assert!(r.is_full());
    }

    #[test]
    fn recording_push_after_auto_stop_is_noop() {
        let mut r = Recording::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        for _ in 0..MAX_RECORDING_SAMPLES {
            r.push(&m);
        }
        assert!(!r.active);
        // Further pushes should be no-ops
        assert!(!r.push(&m));
        assert_eq!(r.samples.len(), MAX_RECORDING_SAMPLES);
    }

    #[test]
    fn sample_from_measurement() {
        let m = make_measurement(b"  5.678");
        let s = Sample::from_measurement(&m);
        assert_eq!(s.mode, "DC V");
        assert_eq!(s.value_str, "5.678");
        assert_eq!(s.unit, "V");
    }
}
