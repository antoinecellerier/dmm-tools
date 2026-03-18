use super::{DeviceTable, RangeInfo};
use crate::mode::Mode;

/// Device table for the UNI-T UT61E+.
pub struct Ut61ePlusTable {
    // Tables indexed by range byte (0x00..0x07 typically)
    dc_v: [RangeInfo; 5],
    ac_v: [RangeInfo; 5],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 7],
    capacitance: [RangeInfo; 8],
    hz: [RangeInfo; 5],
    duty_cycle: [RangeInfo; 1],
    temp_c: [RangeInfo; 1],
    temp_f: [RangeInfo; 1],
    diode: [RangeInfo; 1],
    continuity: [RangeInfo; 1],
    dc_ua: [RangeInfo; 2],
    ac_ua: [RangeInfo; 2],
    dc_ma: [RangeInfo; 2],
    ac_ma: [RangeInfo; 2],
    dc_a: [RangeInfo; 2],
    ac_a: [RangeInfo; 2],
}

impl Ut61ePlusTable {
    pub fn new() -> Self {
        Self {
            dc_v: [
                RangeInfo {
                    label: "2.2V",
                    unit: "V",
                    overload_pos: 2.2,
                    overload_neg: -2.2,
                },
                RangeInfo {
                    label: "22V",
                    unit: "V",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220V",
                    unit: "V",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "1000V",
                    unit: "V",
                    overload_pos: 1000.0,
                    overload_neg: -1000.0,
                },
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            ac_v: [
                RangeInfo {
                    label: "2.2V",
                    unit: "V",
                    overload_pos: 2.2,
                    overload_neg: -2.2,
                },
                RangeInfo {
                    label: "22V",
                    unit: "V",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220V",
                    unit: "V",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "750V",
                    unit: "V",
                    overload_pos: 750.0,
                    overload_neg: -750.0,
                },
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            dc_mv: [
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2.2V",
                    unit: "mV",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ac_mv: [
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2.2V",
                    unit: "mV",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ohm: [
                RangeInfo {
                    label: "220Ω",
                    unit: "Ω",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2kΩ",
                    unit: "kΩ",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22kΩ",
                    unit: "kΩ",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220kΩ",
                    unit: "kΩ",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2MΩ",
                    unit: "MΩ",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22MΩ",
                    unit: "MΩ",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220MΩ",
                    unit: "MΩ",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            capacitance: [
                RangeInfo {
                    label: "22nF",
                    unit: "nF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220nF",
                    unit: "nF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2µF",
                    unit: "µF",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22µF",
                    unit: "µF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220µF",
                    unit: "µF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2mF",
                    unit: "mF",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22mF",
                    unit: "mF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220mF",
                    unit: "mF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            hz: [
                RangeInfo {
                    label: "22Hz",
                    unit: "Hz",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220Hz",
                    unit: "Hz",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2kHz",
                    unit: "kHz",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22kHz",
                    unit: "kHz",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220kHz",
                    unit: "kHz",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            duty_cycle: [RangeInfo {
                label: "Duty",
                unit: "%",
                overload_pos: 100.0,
                overload_neg: 0.0,
            }],
            temp_c: [RangeInfo {
                label: "Temp",
                unit: "°C",
                overload_pos: 1200.0,
                overload_neg: -40.0,
            }],
            temp_f: [RangeInfo {
                label: "Temp",
                unit: "°F",
                overload_pos: 2192.0,
                overload_neg: -40.0,
            }],
            diode: [RangeInfo {
                label: "Diode",
                unit: "V",
                overload_pos: 2.2,
                overload_neg: 0.0,
            }],
            continuity: [RangeInfo {
                label: "Cont",
                unit: "Ω",
                overload_pos: 220.0,
                overload_neg: f64::NEG_INFINITY,
            }],
            dc_ua: [
                RangeInfo {
                    label: "220µA",
                    unit: "µA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2200µA",
                    unit: "µA",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ac_ua: [
                RangeInfo {
                    label: "220µA",
                    unit: "µA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2200µA",
                    unit: "µA",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            dc_ma: [
                RangeInfo {
                    label: "22mA",
                    unit: "mA",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220mA",
                    unit: "mA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            ac_ma: [
                RangeInfo {
                    label: "22mA",
                    unit: "mA",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220mA",
                    unit: "mA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            dc_a: [
                // Range 0x00 unknown — may not be used. Placeholder.
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
                // Range 0x01 verified: 20A range (confirmed with bench PSU at 100mA)
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
            ],
            ac_a: [
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
            ],
        }
    }

    fn lookup<'a>(&self, table: &'a [RangeInfo], range: u8) -> Option<&'a RangeInfo> {
        table.get(range as usize)
    }
}

impl Default for Ut61ePlusTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceTable for Ut61ePlusTable {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo> {
        match mode {
            Mode::DcV => self.lookup(&self.dc_v, range),
            Mode::AcV => self.lookup(&self.ac_v, range),
            Mode::DcMv => self.lookup(&self.dc_mv, range),
            Mode::AcMv => self.lookup(&self.ac_mv, range),
            Mode::Ohm => self.lookup(&self.ohm, range),
            Mode::Capacitance => self.lookup(&self.capacitance, range),
            Mode::Hz => self.lookup(&self.hz, range),
            Mode::DutyCycle => self.lookup(&self.duty_cycle, range),
            Mode::TempC => self.lookup(&self.temp_c, range),
            Mode::TempF => self.lookup(&self.temp_f, range),
            Mode::Diode => self.lookup(&self.diode, range),
            Mode::Continuity => self.lookup(&self.continuity, range),
            Mode::DcUa => self.lookup(&self.dc_ua, range),
            Mode::AcUa => self.lookup(&self.ac_ua, range),
            Mode::DcMa => self.lookup(&self.dc_ma, range),
            Mode::AcMa => self.lookup(&self.ac_ma, range),
            Mode::DcA => self.lookup(&self.dc_a, range),
            Mode::AcA => self.lookup(&self.ac_a, range),
            // Derived modes share tables with their base mode
            Mode::AcDcV | Mode::LpfV | Mode::LozV => self.lookup(&self.dc_v, range),
            Mode::AcDcMv | Mode::LpfMv => self.lookup(&self.dc_mv, range),
            Mode::AcDcA | Mode::AcDcDcA | Mode::AcDcA2 | Mode::LpfA => {
                self.lookup(&self.dc_a, range)
            }
            // Modes without range tables
            Mode::Ncv | Mode::Hfe | Mode::Live | Mode::Inrush => None,
        }
    }

    fn model_name(&self) -> &'static str {
        "UNI-T UT61E+"
    }
}
