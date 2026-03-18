use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::mode::Mode;
use crate::protocol::MEASUREMENT_PAYLOAD_LEN;
use crate::tables::DeviceTable;
use log::debug;
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
#[derive(Debug, Clone)]
pub struct Measurement {
    pub timestamp: Instant,
    pub mode: Mode,
    pub range: u8,
    /// The raw 7-character ASCII display value as received.
    pub display_raw: String,
    pub value: MeasuredValue,
    /// Unit string (e.g., "V", "mV", "kΩ").
    pub unit: &'static str,
    /// Range label (e.g., "2.2V", "220mV").
    pub range_label: &'static str,
    /// Bar graph progress value (0-100).
    pub progress: u16,
    pub flags: StatusFlags,
    /// Raw 14-byte payload as received (for protocol debugging).
    pub raw_payload: Vec<u8>,
}

impl Measurement {
    /// Parse a 14-byte measurement payload.
    ///
    /// Layout (verified against real device captures):
    /// - byte 0:    mode   (raw, no masking — does not have 0x30 prefix)
    /// - byte 1:    range  (& 0x0F — has 0x30 prefix)
    /// - bytes 2-8: display value (7 ASCII chars, no masking needed)
    /// - byte 9:    progress high (raw, no 0x30 prefix)
    /// - byte 10:   progress low  (raw, no 0x30 prefix)
    /// - byte 11:   flag1  (& 0x0F — has 0x30 prefix)
    /// - byte 12:   flag2  (& 0x0F — has 0x30 prefix)
    /// - byte 13:   flag3  (& 0x0F — has 0x30 prefix)
    pub fn parse(payload: &[u8], table: &dyn DeviceTable) -> Result<Self> {
        if payload.len() < MEASUREMENT_PAYLOAD_LEN {
            return Err(Error::InvalidResponse(format!(
                "payload too short: {} bytes, expected {}",
                payload.len(),
                MEASUREMENT_PAYLOAD_LEN
            )));
        }

        // Mode byte is raw (no 0x30 prefix), range byte has 0x30 prefix
        let mode_byte = payload[0];
        let range_byte = payload[1] & 0x0F;
        let display_bytes = &payload[2..9];
        // Progress bytes are raw (no 0x30 prefix observed on real device)
        let progress_hi = payload[9] as u16;
        let progress_lo = payload[10] as u16;
        let flag1 = payload[11] & 0x0F;
        let flag2 = payload[12] & 0x0F;
        let flag3 = payload[13] & 0x0F;

        let mode = Mode::from_byte(mode_byte)?;
        let display_raw = String::from_utf8_lossy(display_bytes).to_string();
        let progress = (progress_hi << 4) | progress_lo;
        let flags = StatusFlags::parse(flag1, flag2, flag3);

        // Look up range info from device table
        let range_info = table.range_info(mode, range_byte);
        let unit = range_info.map(|r| r.unit).unwrap_or("");
        let range_label = range_info.map(|r| r.label).unwrap_or("");

        // Parse display value.
        // The meter pads with spaces for alignment, e.g. "- 55.79" for -55.79.
        // Strip all spaces before parsing to handle this.
        let display_trimmed = display_raw.trim();
        let display_compact: String = display_trimmed.chars().filter(|c| *c != ' ').collect();
        let value = if mode == Mode::Ncv {
            // NCV mode: display is a level indicator
            let level = display_compact.parse::<u8>().unwrap_or(0);
            MeasuredValue::NcvLevel(level)
        } else if display_compact == "OL" || display_compact.contains("OL") {
            MeasuredValue::Overload
        } else {
            match display_compact.parse::<f64>() {
                Ok(v) => MeasuredValue::Normal(v),
                Err(_) => {
                    debug!(
                        "measurement: could not parse display value: {:?} (compact: {:?})",
                        display_trimmed, display_compact
                    );
                    MeasuredValue::Overload
                }
            }
        };

        Ok(Measurement {
            timestamp: Instant::now(),
            mode,
            range: range_byte,
            display_raw,
            value,
            unit,
            range_label,
            progress,
            flags,
            raw_payload: payload[..MEASUREMENT_PAYLOAD_LEN].to_vec(),
        })
    }
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value_str = match &self.value {
            MeasuredValue::Normal(v) => format!("{v}"),
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
    use crate::tables::ut61e_plus::Ut61ePlusTable;

    fn make_payload(
        mode: u8,
        range: u8,
        display: &[u8; 7],
        progress: (u8, u8),
        flags: (u8, u8, u8),
    ) -> Vec<u8> {
        // Mode byte is raw (no 0x30 prefix).
        // Range and flag bytes have 0x30 prefix.
        // Progress bytes are raw (no 0x30 prefix).
        vec![
            mode,
            range | 0x30,
            display[0],
            display[1],
            display[2],
            display[3],
            display[4],
            display[5],
            display[6],
            progress.0,
            progress.1,
            flags.0 | 0x30,
            flags.1 | 0x30,
            flags.2 | 0x30,
        ]
    }

    #[test]
    fn parse_dc_voltage() {
        let table = Ut61ePlusTable::new();
        // Mode 0x02 (DCV), Range 0x01 (22V), display " 12.345"
        let payload = make_payload(0x02, 0x01, b" 12.345", (0x05, 0x0A), (0x00, 0x00, 0x00));
        let m = Measurement::parse(&payload, &table).unwrap();
        assert_eq!(m.mode, Mode::DcV);
        assert_eq!(m.range, 0x01);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "22V");
        assert!(m.flags.auto_range);
    }

    #[test]
    fn parse_overload() {
        let table = Ut61ePlusTable::new();
        // Mode 0x06 (Ohm), Range 0x00 (220Ω), display "    OL "
        let payload = make_payload(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = Measurement::parse(&payload, &table).unwrap();
        assert_eq!(m.mode, Mode::Ohm);
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.unit, "Ω");
    }

    #[test]
    fn parse_with_hold_flag() {
        let table = Ut61ePlusTable::new();
        // Mode 0x02 (DCV), flag1=0x02 (HOLD), flag2=0x00 (AUTO on)
        let payload = make_payload(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = Measurement::parse(&payload, &table).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range);
        assert!(!m.flags.rel);
    }

    #[test]
    fn parse_negative_with_space() {
        // The meter sends "- 55.79" (space between sign and digits) for -55.79 mV
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x03, 0x00, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = Measurement::parse(&payload, &table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-55.79)).abs() < 1e-6));
    }

    #[test]
    fn parse_payload_too_short() {
        let table = Ut61ePlusTable::new();
        let payload = vec![0x30; 10]; // too short
        assert!(Measurement::parse(&payload, &table).is_err());
    }

    #[test]
    fn display_format() {
        let table = Ut61ePlusTable::new();
        // Mode 0x02 (DCV), flag1=0x02 (HOLD), flag2=0x00 (AUTO on)
        let payload = make_payload(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = Measurement::parse(&payload, &table).unwrap();
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }
}
