use super::*;
use crate::protocol::Protocol;
use crate::protocol::test_support::unit_family;
use crate::protocol::ut61eplus::{Ut61PlusProtocol, make_payload};
use crate::specs::SpecSheetRow;
use std::collections::{HashMap, HashSet};

/// Each model, with the id its protocol is built from.
const MODELS: &[(SpecModel, &str)] = &[
    (SpecModel::Ut61ePlus, "ut61e+"),
    (SpecModel::Ut61bPlus, "ut61b+"),
    (SpecModel::Ut61dPlus, "ut61d+"),
    (SpecModel::Ut161e, "ut161e"),
    (SpecModel::Ut161b, "ut161b"),
    (SpecModel::Ut161d, "ut161d"),
];

/// Each UT161 model and the UT61+ one whose manual pages it shares.
const UT161: &[(SpecModel, SpecModel)] = &[
    (SpecModel::Ut161e, SpecModel::Ut61ePlus),
    (SpecModel::Ut161b, SpecModel::Ut61bPlus),
    (SpecModel::Ut161d, SpecModel::Ut61dPlus),
];

/// A reading of `mode` at range byte `range`, as the model's parser gives
/// it, and whether the parser reported anything in it.
fn parse(id: &str, mode: u8, range: u8) -> (Option<Measurement>, bool) {
    let proto = Ut61PlusProtocol::for_model(id).unwrap();
    // "3" reads as a number, and in NCV as a level.
    let payload = make_payload(mode, range, b"      3", (0, 0), (0, 0, 0));
    let (m, reports) = crate::protocol::capture_reports(|| proto.parse_payload(&payload));
    (m.ok(), !reports.is_empty())
}

/// Every mode and range byte the model's parser accepts without a report,
/// as its reading.
fn accepted_readings(id: &str) -> Vec<Measurement> {
    (0..=0x1F)
        .flat_map(|mode| (0..=0xF).map(move |range| (mode, range)))
        .filter_map(|(mode, range)| match parse(id, mode, range) {
            (Some(m), false) => Some(m),
            _ => None,
        })
        .collect()
}

fn mode_of(m: &Measurement) -> Mode {
    Mode::from_byte(m.mode_raw as u8).unwrap()
}

/// A reading of `mode` at range byte `range`, parsed or not.
fn reading(id: &str, mode: Mode, range: u8) -> Measurement {
    parse(id, mode as u8, range).0.unwrap()
}

/// Why a reading has no row, and which readings that is.
type Listed = (&'static str, fn(SpecModel, Mode, u8) -> bool);

/// Readings that take their table's mode data but no row.
const MODE_SPEC_ONLY: &[Listed] = &[(
    "UT61E+ and UT161E mV range 1: the mV position is fixed at 220mV in DC and AC (on a UT61E+, RANGE does nothing and only range byte 0 has been seen), so the meter never sends byte 1",
    |model, mode, range| {
        matches!(model, SpecModel::Ut61ePlus | SpecModel::Ut161e)
            && matches!(mode, Mode::DcMv | Mode::AcMv)
            && range == 1
    },
)];

/// Readings the parser accepts that have no spec in the manual.
const NO_SPEC: &[Listed] = &[("NCV: the manual has no NCV table", |_, mode, _| {
    mode == Mode::Ncv
})];

/// Each reading resolves a row, is listed as taking its table's mode data
/// only, or is listed as having no spec; every row of every table is some
/// reading's. The protocol answers with the very rows the model's tables
/// hold.
#[test]
fn every_reading_has_a_spec_or_is_listed() {
    let lists = [MODE_SPEC_ONLY, NO_SPEC];
    let mut listed_reached = HashSet::new();
    for &(model, id) in MODELS {
        let proto = Ut61PlusProtocol::for_model(id).unwrap();
        let mut rows_reached = HashSet::new();
        for m in accepted_readings(id) {
            let (mode, range) = (mode_of(&m), m.range_raw);
            let listed: Vec<(usize, &str)> = lists
                .iter()
                .enumerate()
                .flat_map(|(l, list)| list.iter().map(move |entry| (l, entry)))
                .filter(|(_, (_, hit))| hit(model, mode, range))
                .map(|(l, (why, _))| (l, *why))
                .collect();
            let table = model.table(&m);
            match (model.row(&m), listed.as_slice()) {
                (Some(row), []) => {
                    assert!(std::ptr::eq(proto.spec_info(&m).unwrap(), &row.spec));
                    let table = table.unwrap();
                    assert!(std::ptr::eq(proto.mode_spec_info(&m).unwrap(), &table.mode));
                    rows_reached.insert(std::ptr::from_ref(row));
                }
                (None, [(l, why)]) => {
                    assert!(proto.spec_info(&m).is_none());
                    // The first list keeps the mode data, the second has none.
                    assert_eq!(table.is_some(), *l == 0, "{id} {mode:?} {range}: {why}");
                    let mode_spec = proto.mode_spec_info(&m);
                    assert_eq!(mode_spec.is_some(), *l == 0, "{id} {mode:?} {range}");
                    listed_reached.insert(*why);
                }
                (Some(_), _) => panic!("{id} {mode:?} range {range}: has a row, yet is listed"),
                (None, _) => panic!("{id} {mode:?} range {range}: no row, and not listed once"),
            }
        }
        for table in model.tables() {
            for row in table.ranges {
                assert!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    "{id}: {} / {} is no reading's",
                    table.name,
                    row.label
                );
            }
        }
    }
    for (why, _) in lists.iter().flat_map(|list| list.iter()) {
        assert!(listed_reached.contains(why), "no reading is {why}");
    }
}

/// Every table a model's lookup answers with is one of the model's own, for
/// any mode byte: a lookup that strays into another model's part shows specs
/// that model's manual column does not print.
#[test]
fn lookups_stay_in_the_model() {
    for &(model, id) in MODELS {
        let own: Vec<&ModeSpecs> = model.tables().collect();
        for &mode in Mode::ALL {
            let m = Measurement {
                mode_raw: mode as u16,
                ..Measurement::from_payload(&[])
            };
            if let Some(table) = model.table(&m) {
                assert!(
                    own.iter().any(|t| std::ptr::eq(*t, table)),
                    "{id} {mode:?}: {} is not the model's",
                    table.name
                );
            }
        }
    }
}

/// `label` as a value in its base unit, and that unit: "2.2000kΩ" is
/// (2200.0, "Ω"). A label that names more than its range ("LoZ ACV 600.0V")
/// is read from its last word. `None` for a label that is not a number, an
/// SI prefix and a unit.
fn si_quantity(label: &str) -> Option<(f64, &str)> {
    let label = label.rsplit(' ').next().unwrap_or(label);
    let digits = label
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(label.len());
    let value: f64 = label[..digits].parse().ok()?;
    let unit = &label[digits..];
    let base = ["Hz", "Ω", "V", "A", "F"]
        .into_iter()
        .find(|base| unit.ends_with(base))?;
    let factor = match &unit[..unit.len() - base.len()] {
        "" => 1.0,
        "n" => 1e-9,
        "µ" => 1e-6,
        "m" => 1e-3,
        "k" => 1e3,
        "M" => 1e6,
        _ => return None,
    };
    Some((value * factor, base))
}

/// A row answers the reading's range: its label's full scale is the one the
/// range table labels the reading with. A row mapped to the wrong range byte
/// names another decade, or another quantity. Rows labelled by a function
/// ("Continuity") or a span ("10Hz~220MHz") are left out.
#[test]
fn rows_match_the_readings_full_scale() {
    for &(model, id) in MODELS {
        for m in accepted_readings(id) {
            let Some(row) = model.row(&m) else {
                continue;
            };
            let Some(label) = si_quantity(row.label) else {
                assert!(row.range.is_none(), "{id}: {} is not a quantity", row.label);
                continue;
            };
            let reading = si_quantity(&m.range_label).unwrap_or_else(|| {
                panic!(
                    "{id} {:?}: range {} is not a quantity",
                    mode_of(&m),
                    m.range_label
                )
            });
            assert!(
                label.1 == reading.1 && (label.0 - reading.0).abs() <= 1e-9 * reading.0,
                "{id} {:?} range {}: row {} for a {} range",
                mode_of(&m),
                m.range_raw,
                row.label,
                m.range_label
            );
        }
    }
}

/// What a volts or current reading measures.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Coupling {
    Ac,
    Dc,
    AcDc,
}

/// The coupling a reading in `mode` measures; `None` for the modes that
/// measure neither.
fn mode_coupling(mode: Mode) -> Option<Coupling> {
    match mode {
        Mode::DcV | Mode::DcMv | Mode::DcUa | Mode::DcMa | Mode::DcA => Some(Coupling::Dc),
        Mode::AcV
        | Mode::AcMv
        | Mode::AcUa
        | Mode::AcMa
        | Mode::AcA
        | Mode::LpfV
        | Mode::LpfMv
        | Mode::LpfA
        | Mode::LozV
        | Mode::LozV2 => Some(Coupling::Ac),
        Mode::AcDcV | Mode::AcDcMv | Mode::AcDcA2 => Some(Coupling::AcDc),
        _ => None,
    }
}

/// The coupling a table's title names, `None` for the other tables.
fn table_coupling(table: &ModeSpecs) -> Option<Coupling> {
    match table.name {
        "DC Voltage" | "DC Current" => Some(Coupling::Dc),
        "AC Voltage" | "AC Current" => Some(Coupling::Ac),
        "AC+DC Voltage" => Some(Coupling::AcDc),
        _ => None,
    }
}

/// A reading's table measures what the reading does: AC, DC, AC+DC, or
/// none of them.
#[test]
fn tables_match_the_readings_coupling() {
    for &(model, id) in MODELS {
        for m in accepted_readings(id) {
            let Some(table) = model.table(&m) else {
                continue;
            };
            let mode = mode_of(&m);
            assert_eq!(
                table_coupling(table),
                mode_coupling(mode),
                "{id} {mode:?}: {}",
                table.name
            );
        }
    }
}

/// A row's resolution is in the reading's unit, prefix aside: a row of
/// another quantity (continuity for diode, °F for °C) resolves in another
/// unit. A resolution that is a span ("0.01Hz~0.01MHz") is read up to its
/// `~`.
#[test]
fn rows_resolve_in_the_readings_unit() {
    for &(model, id) in MODELS {
        for m in accepted_readings(id) {
            let Some(row) = model.row(&m) else {
                continue;
            };
            let unit = row
                .spec
                .resolution
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            let unit = unit.split('~').next().unwrap_or(unit);
            assert_eq!(
                unit_family(unit),
                unit_family(&m.unit),
                "{id} {:?}: row {} resolves in {}",
                mode_of(&m),
                row.label,
                row.spec.resolution
            );
        }
    }
}

/// No two rows of a sheet share a table and a label, but for a printed row
/// split across parts that each hold some of its bands (the UT61E+ AC
/// Voltage rows, whose LPF band is a part of its own). Those parts agree on
/// the row's resolution, impedance and overload, and qualify every band,
/// each qualifier in one part only, so that together they give the printed
/// row: the compare against the verified transcription joins them into it.
#[test]
fn sheet_rows_are_unique() {
    for &(model, id) in MODELS {
        let sheet = model.sheet();
        let mut rows: HashMap<(&str, &str), Vec<(&SpecSheetTable, &SpecSheetRow)>> = HashMap::new();
        for table in &sheet {
            for row in &table.rows {
                rows.entry((table.name, row.label))
                    .or_default()
                    .push((table, row));
            }
        }
        for ((name, label), parts) in rows {
            let [(first_table, first_row), ..] = parts[..] else {
                continue;
            };
            if parts.len() == 1 {
                continue;
            }
            let mut qualifiers = HashSet::new();
            for (table, row) in parts {
                assert_eq!(
                    row.spec.resolution, first_row.spec.resolution,
                    "{id}: {name} / {label}"
                );
                assert_eq!(
                    table.mode.input_impedance, first_table.mode.input_impedance,
                    "{id}: {name} / {label}"
                );
                assert_eq!(
                    table.mode.overload_protection, first_table.mode.overload_protection,
                    "{id}: {name} / {label}"
                );
                for band in row.spec.accuracy {
                    let qualifier = band.freq_range.unwrap_or_else(|| {
                        panic!("{id}: {name} / {label} is split, yet has a band with no qualifier")
                    });
                    assert!(
                        qualifiers.insert(qualifier),
                        "{id}: {name} / {label}: {qualifier} twice"
                    );
                }
            }
        }
    }
}

/// The parts of a manual table follow each other: the review sheet lays a
/// table out from its adjacent parts.
#[test]
fn parts_of_a_table_are_adjacent() {
    for &(model, id) in MODELS {
        let names: Vec<&str> = model.tables().map(|t| t.name).collect();
        for (i, name) in names.iter().enumerate() {
            if i > 0 && names[i - 1] != *name {
                assert!(
                    !names[..i].contains(name),
                    "{id}: {name} is split by another table"
                );
            }
        }
    }
}

/// The manual gives the mV range an input impedance of its own, in DC only.
#[test]
fn mv_rows_carry_the_mv_impedance() {
    for &(model, id) in MODELS {
        let impedance = |mode| {
            let m = reading(id, mode, 0);
            model.table(&m).unwrap().mode.input_impedance
        };
        assert_eq!(impedance(Mode::DcMv), Some("About 1GΩ"), "{id}");
        assert_eq!(impedance(Mode::DcV), Some("About 10MΩ"), "{id}");
        assert_eq!(impedance(Mode::AcMv), Some("About 10MΩ"), "{id}");
    }
}

/// The only readings of the current ranges that carry the >5A limit are the
/// A ranges', and the only ones that carry the µA minimum are the µA
/// ranges'.
#[test]
fn only_the_a_ranges_carry_the_5a_note() {
    for &(model, id) in MODELS {
        for m in accepted_readings(id) {
            let mode = mode_of(&m);
            let Some(table) = model.table(&m) else {
                continue;
            };
            let has = |prefix: &str| table.mode.notes.iter().any(|n| n.starts_with(prefix));
            let amps = matches!(mode, Mode::DcA | Mode::AcA);
            let micro = matches!(mode, Mode::DcUa | Mode::AcUa);
            if amps || micro || matches!(mode, Mode::DcMa | Mode::AcMa) {
                assert_eq!(has(">5A"), amps, "{id} {mode:?}");
            }
            assert!(!has("µA ranges") || micro, "{id} {mode:?}");
        }
    }
}

/// AC V and AC mV readings show the frequency bands, not the LPF one: that
/// band applies only with LPF on.
#[test]
fn ac_v_has_no_lpf_band() {
    for &(model, id) in MODELS {
        for m in accepted_readings(id) {
            if !matches!(mode_of(&m), Mode::AcV | Mode::AcMv) {
                continue;
            }
            let Some(row) = model.row(&m) else {
                continue;
            };
            assert!(
                row.spec
                    .accuracy
                    .iter()
                    .all(|b| !b.freq_range.is_some_and(|f| f.contains("LPF"))),
                "{id} {:?}: {}",
                mode_of(&m),
                row.label
            );
        }
    }
}

/// An LPF V reading shows the LPF band alone, on the AC V row of its range:
/// with LPF on, it is the only band that applies.
#[test]
fn lpf_v_has_only_the_lpf_band() {
    let (model, id) = (SpecModel::Ut61ePlus, "ut61e+");
    for range in 0..=3 {
        let lpf = model.row(&reading(id, Mode::LpfV, range)).unwrap();
        let ac = model.row(&reading(id, Mode::AcV, range)).unwrap();
        assert_eq!(lpf.label, ac.label, "range {range}");
        let bands: Vec<_> = lpf.spec.accuracy.iter().map(|b| b.freq_range).collect();
        assert_eq!(bands, [Some("40Hz~100Hz (LPF)")], "range {range}");
    }
}

/// The manual prints frequency as one row, a span, whichever range byte the
/// reading carries.
#[test]
fn hz_is_one_row() {
    for &(model, id) in MODELS {
        let rows: Vec<&RangeSpec> = accepted_readings(id)
            .iter()
            .filter(|m| mode_of(m) == Mode::Hz)
            .map(|m| model.row(m).unwrap())
            .collect();
        assert!(rows.len() > 1, "{id}");
        assert!(rows.iter().all(|r| std::ptr::eq(*r, rows[0])), "{id}");
        assert_eq!(rows[0].range, None, "{id}");
    }
}

/// Each A range byte takes the model's own A row: (model, range byte, DC
/// row, AC row).
#[test]
fn amps_follow_the_model() {
    let cases = [
        (SpecModel::Ut61ePlus, 0, "20.000A", "20A"),
        (SpecModel::Ut61ePlus, 1, "20.000A", "20A"),
        (SpecModel::Ut61bPlus, 0, "6.000A", "6.000A"),
        (SpecModel::Ut61bPlus, 1, "10.00A", "10.00A"),
        (SpecModel::Ut61dPlus, 0, "6.000A", "6.000A"),
        (SpecModel::Ut61dPlus, 1, "20.00A", "20.00A"),
    ];
    for (model, range, dc, ac) in cases {
        let id = MODELS.iter().find(|(m, _)| *m == model).unwrap().1;
        let label = |mode| model.row(&reading(id, mode, range)).unwrap().label;
        assert_eq!(label(Mode::DcA), dc, "{id} range {range}");
        assert_eq!(label(Mode::AcA), ac, "{id} range {range}");
    }
}

/// The manual gives temperature to the UT61D+ (and UT161D) only ("8)
/// Temperature"), one row per unit.
#[test]
fn temperature_is_the_ut61d_plus_only() {
    for &(model, id) in MODELS {
        for (mode, row) in [(Mode::TempC, "-40~1000°C"), (Mode::TempF, "-40~1832°F")] {
            let m = Measurement {
                mode_raw: mode as u16,
                ..Measurement::from_payload(&[])
            };
            let label = model.row(&m).map(|r| r.label);
            let want = matches!(model, SpecModel::Ut61dPlus | SpecModel::Ut161d).then_some(row);
            assert_eq!(label, want, "{id} {mode:?}");
        }
    }
}

/// The UT61D+'s two LoZ mode bytes read the one LoZ part, whose rows have
/// no input impedance.
#[test]
fn loz_bytes_share_one_part() {
    let model = SpecModel::Ut61dPlus;
    for (range, label) in [(0, "LoZ ACV 600.0V"), (1, "LoZ ACV 1000V")] {
        let a = model.table(&reading("ut61d+", Mode::LozV, range)).unwrap();
        let b = model.table(&reading("ut61d+", Mode::LozV2, range)).unwrap();
        assert!(std::ptr::eq(a, b), "range {range}");
        assert_eq!(a.mode.input_impedance, None);
        assert_eq!(a.row(range).unwrap().label, label);
    }
}

/// A UT161 reads its UT61+ counterpart's tables, rows and notes; only the
/// current tables differ, in their fuses ("9) DC Current", PDF p. 17).
#[test]
fn ut161_differs_only_in_current_fuses() {
    for &(ut161, ut61) in UT161 {
        let pairs: Vec<_> = ut161.tables().zip(ut61.tables()).collect();
        assert_eq!(pairs.len(), ut61.tables().count(), "{ut161:?}");
        for (a, b) in pairs {
            assert_eq!(a.name, b.name, "{ut161:?}");
            assert_eq!(a.page, b.page, "{ut161:?} {}", a.name);
            assert!(std::ptr::eq(a.ranges, b.ranges), "{ut161:?} {}", a.name);
            assert_eq!(a.mode.input_impedance, b.mode.input_impedance);
            assert!(
                std::ptr::eq(a.mode.notes, b.mode.notes),
                "{ut161:?} {}",
                a.name
            );
            let fuse = match b.mode.overload_protection {
                Some("Fuse 1A 240V") => Some(UT161_F1),
                Some("Fuse 10A 240V") => Some(UT161_F2),
                other => other,
            };
            let current = matches!(a.name, "DC Current" | "AC Current");
            assert_eq!(
                fuse != b.mode.overload_protection,
                current,
                "{ut161:?} {}",
                a.name
            );
            assert_eq!(a.mode.overload_protection, fuse, "{ut161:?} {}", a.name);
        }
        let id = MODELS.iter().find(|(m, _)| *m == ut161).unwrap().1;
        for m in accepted_readings(id) {
            let (a, b) = (ut161.row(&m), ut61.row(&m));
            assert_eq!(a.map(std::ptr::from_ref), b.map(std::ptr::from_ref), "{id}");
            let (a, b) = (ut161.table(&m), ut61.table(&m));
            assert_eq!(a.map(|t| t.name), b.map(|t| t.name), "{id}");
        }
    }
}
