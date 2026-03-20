use crate::flags::StatusFlags;
use std::borrow::Cow;
use std::time::Instant;

/// Represents a parsed measurement value.
#[derive(Debug, Clone)]
pub enum MeasuredValue {
    /// A normal numeric reading.
    Normal(f64),
    /// The meter is showing OL (overload).
    Overload,
    /// NCV (non-contact voltage) detection level (0-4 typically).
    NcvLevel(u8),
}

/// A fully parsed measurement from the meter.
///
/// This is the unified measurement type used by all protocol implementations.
/// String-based mode/unit fields allow each protocol to produce human-readable
/// values without sharing a common mode enum.
///
/// The `mode`, `unit`, and `range_label` fields use `Cow<'static, str>` to
/// avoid per-measurement heap allocations. Most values come from static lookup
/// tables (`Cow::Borrowed`); only fallback paths like `format!("Unknown(0x{:02x})")`
/// produce owned strings (`Cow::Owned`).
#[derive(Debug, Clone)]
pub struct Measurement {
    pub timestamp: Instant,
    /// Human-readable mode string (e.g. "DC V", "AC mV", "Unknown(0x05)").
    pub mode: Cow<'static, str>,
    /// Raw protocol-level mode value (for debugging and spec lookup).
    pub mode_raw: u16,
    /// Raw protocol-level range byte (for spec lookup).
    pub range_raw: u8,
    pub value: MeasuredValue,
    /// Unit string (e.g. "V", "mV", "kΩ", "nS").
    pub unit: Cow<'static, str>,
    /// Range label (e.g. "22V", "220mV", "" if not applicable).
    pub range_label: Cow<'static, str>,
    /// Bar graph progress value, None if the protocol doesn't provide it.
    pub progress: Option<u16>,
    /// Raw ASCII display value as received, None for float-based meters.
    pub display_raw: Option<String>,
    pub flags: StatusFlags,
    /// Raw payload bytes as received (for protocol debugging).
    pub raw_payload: Vec<u8>,
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value_str = match &self.value {
            MeasuredValue::Normal(_) => self
                .display_raw
                .as_deref()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| match &self.value {
                    MeasuredValue::Normal(v) => format!("{v}"),
                    _ => unreachable!(),
                }),
            MeasuredValue::Overload => "OL".to_string(),
            MeasuredValue::NcvLevel(level) => format!("NCV:{level}"),
        };
        write!(f, "{value_str} {}", self.unit)?;
        let flags_str = self.flags.to_string();
        if !flags_str.is_empty() {
            write!(f, " [{flags_str}]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_measurement(
        value: MeasuredValue,
        unit: &'static str,
        flags: StatusFlags,
    ) -> Measurement {
        Measurement {
            timestamp: Instant::now(),
            mode: "DC V".into(),
            mode_raw: 0x02,
            range_raw: 1,
            value,
            unit: unit.into(),
            range_label: "22V".into(),
            progress: Some(0),
            display_raw: Some("  5.678".to_string()),
            flags,
            raw_payload: vec![],
        }
    }

    #[test]
    fn display_normal() {
        let m = make_measurement(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
    }

    #[test]
    fn display_overload() {
        let m = make_measurement(MeasuredValue::Overload, "Ω", StatusFlags::default());
        assert!(m.to_string().contains("OL"));
    }

    #[test]
    fn display_ncv() {
        let m = make_measurement(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        assert!(m.to_string().contains("NCV:3"));
    }

    #[test]
    fn display_with_flags() {
        let flags = StatusFlags {
            hold: true,
            auto_range: true,
            ..Default::default()
        };
        let m = make_measurement(MeasuredValue::Normal(1.0), "V", flags);
        let s = m.to_string();
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }
}
