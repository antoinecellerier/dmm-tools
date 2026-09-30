//! The BM78xBT's function, unit, prefix and display-word tables
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §6.3-§6.5, §6.7).
//!
//! A reading names its function by two IDs, main and sub (spec §6.5). The
//! table is r4's, for both models: nothing on the wire tells a BM788BT from
//! a BM787BT (spec §1), and a BM787BT simply never sends the functions it
//! lacks (T2, T1-T2, %4-20mA, spec §9.5).

/// What a function reads as.
pub(super) struct Function {
    /// The reading's mode name.
    pub(super) name: &'static str,
    /// A DC sub-function (spec §6.5): AC, DC and AC+DC are sub IDs, not
    /// flags (spec, Implementation Notes).
    pub(super) dc: bool,
}

const fn f(name: &'static str) -> Function {
    Function { name, dc: false }
}

const fn dc(name: &'static str) -> Function {
    Function { name, dc: true }
}

/// Main 0x02, AutoCheck: AutoV with its low input impedance (spec §6.5,
/// §9.5).
pub(super) const AUTO_CHECK: u8 = 0x02;
/// Main 0x22, EF detection (spec §6.5).
pub(super) const EF: u8 = 0x22;

/// "Hz of …" sub `03`, struck through in r4 and red in r2, which added main
/// 0x23 for it; which the meter sends is open (spec §6.5;
/// `docs/research/bm78xbt/verification.md`), so both read as line
/// frequency.
const STRUCK_LINE_HZ: Function = f("Line Hz");

/// A main function ID and its sub-functions, by sub ID (spec §6.5).
struct Main {
    id: u8,
    subs: &'static [(u8, Function)],
}

/// r4 p.14 (spec §6.5), with two additions that stay silent: the struck
/// `03` subs, and the app's AutoCheck sub `02` (spec §6.5).
static MAINS: [Main; 17] = [
    Main {
        id: AUTO_CHECK,
        subs: &[
            (0x00, f("LoZ AC V")),
            (0x01, dc("LoZ DC V")),
            // The app's OHM; r4 has no sub 02 (spec §6.5).
            (0x02, f("LoZ Ω")),
            // r4's AUTO: AutoV with no input, showing "Auto" (spec §6.7).
            (0x03, f("Auto V")),
        ],
    },
    Main {
        id: 0x03,
        subs: &[
            (0x00, f("AC V")),
            (0x01, dc("DC V")),
            (0x02, f("AC+DC V")),
            (0x03, STRUCK_LINE_HZ),
        ],
    },
    Main {
        id: 0x17,
        subs: &[(0x00, f("VFD Hz")), (0x01, f("VFD AC V"))],
    },
    Main {
        id: 0x04,
        // No sub 03: the app's "HZ" here is in no r4 row, so it is reported
        // (spec §6.5).
        subs: &[
            (0x00, f("AC mV")),
            (0x01, dc("DC mV")),
            (0x02, f("AC+DC mV")),
        ],
    },
    Main {
        id: 0x05,
        subs: &[
            (0x00, f("AC µA")),
            (0x01, dc("DC µA")),
            (0x02, f("AC+DC µA")),
            (0x03, STRUCK_LINE_HZ),
        ],
    },
    Main {
        id: 0x06,
        subs: &[
            (0x00, f("AC mA")),
            (0x01, dc("DC mA")),
            (0x02, f("AC+DC mA")),
            (0x03, STRUCK_LINE_HZ),
            // The UT171's spelling of the same loop-current reading.
            (0x08, f("% 4-20mA")),
        ],
    },
    Main {
        id: 0x07,
        subs: &[
            (0x00, f("AC A")),
            (0x01, dc("DC A")),
            (0x02, f("AC+DC A")),
            (0x03, STRUCK_LINE_HZ),
        ],
    },
    Main {
        id: 0x0C,
        subs: &[(0x00, f("T1")), (0x01, f("T2")), (0x02, f("T1-T2"))],
    },
    Main {
        id: 0x0D,
        subs: &[(0x00, f("Ω"))],
    },
    Main {
        id: 0x0E,
        subs: &[(0x00, f("Capacitance"))],
    },
    Main {
        id: 0x0F,
        subs: &[(0x00, f("Continuity"))],
    },
    Main {
        id: 0x10,
        subs: &[(0x00, f("Diode"))],
    },
    Main {
        id: 0x11,
        subs: &[(0x00, f("nS"))],
    },
    Main {
        id: 0x12,
        subs: &[(0x00, f("Duty %"))],
    },
    Main {
        id: 0x13,
        subs: &[(0x00, f("Logic Hz"))],
    },
    Main {
        id: EF,
        subs: &[(0x00, f("EF-L")), (0x01, f("EF-H"))],
    },
    Main {
        id: 0x23,
        subs: &[(0x00, f("Line Hz"))],
    },
];

/// Why a function did not look up.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unknown {
    /// A main ID r4 does not list.
    Main,
    /// A listed main ID with a sub ID r4 does not list under it.
    Sub,
}

/// The function a reading's main and sub IDs name (spec §6.5).
pub(super) fn function(main: u8, sub: u8) -> Result<&'static Function, Unknown> {
    let main = MAINS.iter().find(|m| m.id == main).ok_or(Unknown::Main)?;
    main.subs
        .iter()
        .find(|(id, _)| *id == sub)
        .map(|(_, function)| function)
        .ok_or(Unknown::Sub)
}

/// Every function name the table holds, for the capture-step test.
#[cfg(test)]
pub(super) fn names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for (_, function) in MAINS.iter().flat_map(|m| m.subs) {
        if !names.contains(&function.name) {
            names.push(function.name);
        }
    }
    names
}

/// A unit with each metric prefix r4 lists, −9 to 9 in steps of 3, in
/// order (spec §6.3).
macro_rules! prefixed {
    ($unit:literal) => {
        [
            concat!("n", $unit),
            concat!("µ", $unit),
            concat!("m", $unit),
            $unit,
            concat!("k", $unit),
            concat!("M", $unit),
            concat!("G", $unit),
        ]
    };
}

/// The index into [`UNITS`]' rows that has no prefix.
const NO_PREFIX: usize = 3;

/// r4's ten function units, [26], each with every prefix (spec §6.4).
/// µ is U+00B5 and Ω U+03A9, the characters `transform::si_prefix` reads.
const UNITS: [(u8, [&str; 7]); 10] = [
    (0x02, prefixed!("V")),
    (0x03, prefixed!("A")),
    (0x04, prefixed!("Ω")),
    (0x05, prefixed!("S")),
    (0x06, prefixed!("F")),
    (0x08, prefixed!("Hz")),
    (0x0A, prefixed!("%")),
    (0x14, prefixed!("°C")),
    (0x15, prefixed!("°F")),
    // r4's "%4~20mA": the reading is a percentage (spec §6.4).
    (0x4F, prefixed!("%")),
];

/// The metric prefix [25] as an index into [`UNITS`]' rows: a signed power
/// of ten, −9 to 9 in steps of 3 (spec §6.3). `None` for anything else.
pub(super) fn prefix_index(byte: u8) -> Option<usize> {
    let power = i8::from_le_bytes([byte]);
    (power % 3 == 0 && (-9..=9).contains(&power)).then(|| (power / 3 + 3) as usize)
}

/// The unit [26] names with the prefix at `prefix` (from [`prefix_index`],
/// or the unprefixed one). `None` for a unit code r4 does not list.
pub(super) fn unit(code: u8, prefix: Option<usize>) -> Option<&'static str> {
    let (_, row) = UNITS.iter().find(|(c, _)| *c == code)?;
    Some(row[prefix.unwrap_or(NO_PREFIX)])
}

/// What an ASCII reading's count shows (spec §6.7), when Flag0 bit 2 is set.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Shown {
    /// `0`: OL, in the app's table only (spec §6.7).
    Overload,
    /// A word r4 prints: "Auto", "InEr", the dash runs.
    Word(&'static str),
    /// `3`-`7` in EF detection: the field strength as dashes, 1 to 5
    /// (manual p.15-16).
    FieldStrength(u8),
    /// `0A`, `0B` in EF detection: EF-H or EF-L, ready and with no field
    /// found.
    FieldReady,
    /// `0A`, `0B` outside EF detection, where no source puts them: the
    /// words r4 gives them.
    OutsideEf(&'static str),
    /// A word only the app's table has (spec §6.7).
    AppWord(&'static str),
    /// A code no source lists.
    Unknown,
}

/// "InEr", the Beep-Jack warning (spec §6.7).
pub(super) const INPUT_ERROR: u32 = 0x02;

/// r4's dash runs for codes `3`-`7`, as r4 prints them (spec §6.7).
const DASHES: [&str; 5] = ["-", "- -", "- - -", "- - - -", "- - - - -"];

/// What code `code` shows; `in_ef` for a reading in EF detection, where the
/// dashes are the field strength.
pub(super) fn shown(code: u32, in_ef: bool) -> Shown {
    match code {
        0x00 => Shown::Overload,
        0x01 => Shown::Word("Auto"),
        INPUT_ERROR => Shown::Word("InEr"),
        0x03..=0x07 if in_ef => Shown::FieldStrength((code - 2) as u8),
        0x03..=0x07 => Shown::Word(DASHES[(code - 3) as usize]),
        0x0A | 0x0B if in_ef => Shown::FieldReady,
        0x0A => Shown::OutsideEf("EF-H"),
        0x0B => Shown::OutsideEf("EF-L"),
        0x08 => Shown::AppWord("diSC"),
        0x09 => Shown::AppWord("CALi"),
        0x0C => Shown::AppWord("rS-3"),
        0x0D => Shown::AppWord("SoC"),
        0x0E => Shown::AppWord("SoH"),
        0x0F => Shown::AppWord("bAd"),
        _ => Shown::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// r4's rows (spec §6.5), transcribed apart from the table.
    #[test]
    fn every_function_the_spec_lists_looks_up() {
        for (main, sub, name) in [
            (0x02, 0x00, "LoZ AC V"),
            (0x02, 0x01, "LoZ DC V"),
            (0x02, 0x02, "LoZ Ω"),
            (0x02, 0x03, "Auto V"),
            (0x03, 0x00, "AC V"),
            (0x03, 0x01, "DC V"),
            (0x03, 0x02, "AC+DC V"),
            (0x03, 0x03, "Line Hz"),
            (0x17, 0x00, "VFD Hz"),
            (0x17, 0x01, "VFD AC V"),
            (0x04, 0x00, "AC mV"),
            (0x04, 0x01, "DC mV"),
            (0x04, 0x02, "AC+DC mV"),
            (0x05, 0x00, "AC µA"),
            (0x05, 0x01, "DC µA"),
            (0x05, 0x02, "AC+DC µA"),
            (0x05, 0x03, "Line Hz"),
            (0x06, 0x00, "AC mA"),
            (0x06, 0x01, "DC mA"),
            (0x06, 0x02, "AC+DC mA"),
            (0x06, 0x03, "Line Hz"),
            (0x06, 0x08, "% 4-20mA"),
            (0x07, 0x00, "AC A"),
            (0x07, 0x01, "DC A"),
            (0x07, 0x02, "AC+DC A"),
            (0x07, 0x03, "Line Hz"),
            (0x0C, 0x00, "T1"),
            (0x0C, 0x01, "T2"),
            (0x0C, 0x02, "T1-T2"),
            (0x0D, 0x00, "Ω"),
            (0x0E, 0x00, "Capacitance"),
            (0x0F, 0x00, "Continuity"),
            (0x10, 0x00, "Diode"),
            (0x11, 0x00, "nS"),
            (0x12, 0x00, "Duty %"),
            (0x13, 0x00, "Logic Hz"),
            (0x22, 0x00, "EF-L"),
            (0x22, 0x01, "EF-H"),
            (0x23, 0x00, "Line Hz"),
        ] {
            assert_eq!(
                function(main, sub).map(|f| f.name),
                Ok(name),
                "{main:02X}/{sub:02X}"
            );
        }
    }

    #[test]
    fn only_the_dc_subs_are_dc() {
        let dc: Vec<&str> = MAINS
            .iter()
            .flat_map(|m| m.subs)
            .filter(|(_, f)| f.dc)
            .map(|(_, f)| f.name)
            .collect();
        assert_eq!(dc, ["LoZ DC V", "DC V", "DC mV", "DC µA", "DC mA", "DC A"]);
    }

    /// The app's subs r4 does not list, and mains it does not, are unknown.
    #[test]
    fn codes_outside_r4_do_not_look_up() {
        for (main, sub) in [
            (0x03, 0x04),
            (0x03, 0x08),
            (0x04, 0x03),
            (0x06, 0x04),
            (0x17, 0x02),
        ] {
            assert_eq!(
                function(main, sub).err(),
                Some(Unknown::Sub),
                "{main:02X}/{sub:02X}"
            );
        }
        for main in [0x00, 0x01, 0x08, 0x14, 0x16, 0x18, 0x21, 0x24, 0xFF] {
            assert_eq!(function(main, 0).err(), Some(Unknown::Main), "{main:02X}");
        }
    }

    #[test]
    fn prefixes_are_signed_powers_of_ten() {
        for (byte, shown) in [
            (0xF7, "nV"),
            (0xFA, "µV"),
            (0xFD, "mV"),
            (0x00, "V"),
            (0x03, "kV"),
            (0x06, "MV"),
            (0x09, "GV"),
        ] {
            assert_eq!(unit(0x02, prefix_index(byte)), Some(shown), "{byte:#04x}");
        }
        for byte in [0x01, 0x02, 0x0C, 0xF4, 0xFE, 0x80, 0x7F] {
            assert_eq!(prefix_index(byte), None, "{byte:#04x}");
        }
    }

    #[test]
    fn units_use_the_characters_si_prefix_reads() {
        assert_eq!(unit(0x05, prefix_index(0xF7)), Some("nS"));
        assert_eq!(unit(0x03, prefix_index(0xFA)), Some("\u{00B5}A"));
        assert_eq!(unit(0x04, prefix_index(0x03)), Some("k\u{03A9}"));
        assert_eq!(unit(0x06, prefix_index(0xFA)), Some("µF"));
        assert_eq!(unit(0x08, prefix_index(0x03)), Some("kHz"));
        for (code, shown) in [(0x0A, "%"), (0x14, "°C"), (0x15, "°F"), (0x4F, "%")] {
            assert_eq!(unit(code, prefix_index(0)), Some(shown));
        }
        for code in [0x00, 0x01, 0x07, 0x09, 0x0B, 0x16, 0x4E, 0xFF] {
            assert_eq!(unit(code, None), None, "{code:#04x}");
        }
    }

    #[test]
    fn every_ascii_code_shows_what_the_spec_says() {
        assert_eq!(shown(0, false), Shown::Overload);
        assert_eq!(shown(1, false), Shown::Word("Auto"));
        assert_eq!(shown(2, false), Shown::Word("InEr"));
        for (code, dashes) in (3..=7).zip(DASHES) {
            assert_eq!(shown(code, false), Shown::Word(dashes));
            assert_eq!(shown(code, true), Shown::FieldStrength((code - 2) as u8));
        }
        assert_eq!(shown(7, false), Shown::Word("- - - - -"));
        assert_eq!(shown(0x0A, true), Shown::FieldReady);
        assert_eq!(shown(0x0B, true), Shown::FieldReady);
        assert_eq!(shown(0x0A, false), Shown::OutsideEf("EF-H"));
        assert_eq!(shown(0x0B, false), Shown::OutsideEf("EF-L"));
        for (code, word) in [
            (0x08, "diSC"),
            (0x09, "CALi"),
            (0x0C, "rS-3"),
            (0x0D, "SoC"),
            (0x0E, "SoH"),
            (0x0F, "bAd"),
        ] {
            assert_eq!(shown(code, false), Shown::AppWord(word));
        }
        for code in [0x10, 0xFF, 0x7F_FFFF] {
            assert_eq!(shown(code, false), Shown::Unknown);
        }
    }
}
