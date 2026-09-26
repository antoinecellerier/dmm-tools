//! The 121GW's mode and range tables
//! (`docs/research/121gw/reverse-engineered-protocol.md` §6.1, §6.2).
//!
//! The packet carries a mode code and a range code; the decimal point and
//! the unit prefix of the main display follow from the two (spec §6.2).
//! Each range here is the EEVblog LCD column of §6.2, labelled with the
//! manual's range name, `""` where the manual names none. The VA modes
//! repeat labels (ranges 1 and 2 are different current × voltage pairs of
//! one size, spec §6.2), so anything keyed on a range keys on the range
//! byte, not the label.

/// One range of a mode: digits after the decimal point, unit, label.
pub(super) struct Range {
    pub(super) decimals: u8,
    pub(super) unit: &'static str,
    pub(super) label: &'static str,
}

/// One mode code: the reading's name and its ranges by range code.
pub(super) struct Mode {
    pub(super) name: &'static str,
    pub(super) ranges: &'static [Range],
}

const fn r(decimals: u8, unit: &'static str, label: &'static str) -> Range {
    Range {
        decimals,
        unit,
        label,
    }
}

const LOZ: &[Range] = &[r(1, "V", "600V")];
const VOLTS: &[Range] = &[
    r(4, "V", "5V"),
    r(3, "V", "50V"),
    r(2, "V", "500V"),
    r(1, "V", "600V"),
];
const MILLIVOLTS: &[Range] = &[r(3, "mV", "50mV"), r(2, "mV", "500mV")];
/// °C here; the unit bits of byte 6 pick °C or °F, and the mode is named
/// for the unit (spec §6.4).
const TEMP: &[Range] = &[r(1, "°C", "")];
const HZ: &[Range] = &[
    r(3, "Hz", "99.999Hz"),
    r(2, "Hz", "999.99Hz"),
    r(4, "kHz", "9.9999kHz"),
    r(3, "kHz", "99.999kHz"),
    r(2, "kHz", "999.99kHz"),
];
const PULSE_WIDTH: &[Range] = &[r(4, "ms", ""), r(3, "ms", ""), r(2, "ms", "")];
const DUTY: &[Range] = &[r(1, "%", "")];
const OHMS: &[Range] = &[
    r(3, "Ω", "50Ω"),
    r(2, "Ω", "500Ω"),
    r(4, "kΩ", "5kΩ"),
    r(3, "kΩ", "50kΩ"),
    r(2, "kΩ", "500kΩ"),
    r(4, "MΩ", "5MΩ"),
    r(3, "MΩ", "50MΩ"),
];
const CONTINUITY: &[Range] = &[r(2, "Ω", "500Ω")];
/// 0.1 mV at 3 V, as both apps; the manual gives 1 mV (spec §6.2, §14.6).
const DIODE: &[Range] = &[r(4, "V", "3V"), r(3, "V", "15V")];
/// The top range as both apps and manual p.19; p.71 says 10.00 mF (spec
/// §6.2, §14.7).
const CAPACITANCE: &[Range] = &[
    r(2, "nF", "10nF"),
    r(1, "nF", "100nF"),
    r(3, "µF", "1µF"),
    r(2, "µF", "10µF"),
    r(1, "µF", "100µF"),
    r(0, "µF", "9999µF"),
];
const MICRO_VA: &[Range] = &[
    r(2, "µVA", "250µVA"),
    r(1, "µVA", "2500µVA"),
    r(1, "µVA", "2500µVA"),
    r(0, "µVA", "25000µVA"),
];
const MILLI_VA: &[Range] = &[
    r(3, "mVA", "25mVA"),
    r(2, "mVA", "250mVA"),
    r(2, "mVA", "250mVA"),
    r(1, "mVA", "2500mVA"),
];
const VA: &[Range] = &[
    r(1, "mVA", "2500mVA"),
    r(0, "mVA", "25000mVA"),
    r(3, "VA", "50VA"),
    r(2, "VA", "500VA"),
];
const MICRO_AMPS: &[Range] = &[r(3, "µA", "50µA"), r(2, "µA", "500µA")];
const MILLI_AMPS: &[Range] = &[r(4, "mA", "5mA"), r(3, "mA", "50mA")];
const AMPS: &[Range] = &[r(2, "mA", "500mA"), r(4, "A", "5A"), r(3, "A", "10A")];

const fn m(name: &'static str, ranges: &'static [Range]) -> Mode {
    Mode { name, ranges }
}

/// Byte 5 bits 4-0, codes 0-24 (spec §6.1).
pub(super) static MODES: [Mode; 25] = [
    m("LoZ V", LOZ),
    m("DC V", VOLTS),
    m("AC V", VOLTS),
    m("DC mV", MILLIVOLTS),
    m("AC mV", MILLIVOLTS),
    m("°C", TEMP),
    m("Hz", HZ),
    m("Pulse Width", PULSE_WIDTH),
    m("Duty %", DUTY),
    m("Ω", OHMS),
    m("Continuity", CONTINUITY),
    m("Diode", DIODE),
    m("Capacitance", CAPACITANCE),
    m("AC µVA", MICRO_VA),
    m("AC mVA", MILLI_VA),
    m("AC VA", VA),
    m("AC µA", MICRO_AMPS),
    m("DC µA", MICRO_AMPS),
    m("AC mA", MILLI_AMPS),
    m("DC mA", MILLI_AMPS),
    m("AC A", AMPS),
    m("DC A", AMPS),
    m("DC µVA", MICRO_VA),
    m("DC mVA", MILLI_VA),
    m("DC VA", VA),
];

/// The main display's mode code, byte 5 bits 4-0 (spec §5).
pub(super) fn mode_code(p: &[u8]) -> u8 {
    p[5] & 0x1F
}

/// The main display's range code, byte 6 bits 3-0 (spec §5).
pub(super) fn range_code(p: &[u8]) -> u8 {
    p[6] & 0x0F
}

/// The mode and range a packet names, `None` for a code outside the tables.
pub(super) fn lookup(p: &[u8]) -> Option<(&'static Mode, &'static Range)> {
    let mode = MODES.get(usize::from(mode_code(p)))?;
    let range = mode.ranges.get(usize::from(range_code(p)))?;
    Some((mode, range))
}

/// Whether the packet's mode and range are in the tables.
pub(super) fn decodable(p: &[u8]) -> bool {
    lookup(p).is_some()
}

/// The VA modes, whose secondary display shows the voltage and the current
/// the power is the product of (spec §6.1, §7.1).
pub(super) fn is_va(mode: u8) -> bool {
    matches!(mode, 13..=15 | 22..=24)
}

/// The name a mode takes with the 1 kHz low-pass filter on, byte 15 bit 6
/// (spec §9): the AC modes that REL held selects it in (manual p.33).
pub(super) fn lpf_name(mode: u8) -> Option<&'static str> {
    match mode {
        2 => Some("LPF V"),
        4 => Some("LPF mV"),
        16 => Some("LPF µA"),
        18 => Some("LPF mA"),
        20 => Some("LPF A"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §6.2's range rows per mode.
    #[test]
    fn every_mode_has_the_specs_range_count() {
        let counts: Vec<usize> = MODES.iter().map(|m| m.ranges.len()).collect();
        assert_eq!(
            counts,
            [
                1, 4, 4, 2, 2, 1, 5, 3, 1, 7, 1, 2, 6, 4, 4, 4, 2, 2, 2, 2, 3, 3, 4, 4, 4
            ]
        );
    }

    #[test]
    fn mode_and_range_codes_take_their_bits_only() {
        let mut p = [0u8; 19];
        p[5] = 0xC0 | 0x20 | 24;
        p[6] = 0xF0 | 3;
        assert_eq!((mode_code(&p), range_code(&p)), (24, 3));
        assert!(decodable(&p));
        p[6] = 4;
        assert!(!decodable(&p), "DC VA has four ranges");
        p[5] = 25;
        p[6] = 0;
        assert!(!decodable(&p), "no mode 25");
    }

    #[test]
    fn only_the_ac_modes_the_manual_names_take_the_filter() {
        let named: Vec<u8> = (0..=24).filter(|&m| lpf_name(m).is_some()).collect();
        assert_eq!(named, [2, 4, 16, 18, 20]);
        for mode in named {
            assert!(MODES[usize::from(mode)].name.starts_with("AC "));
        }
    }
}
