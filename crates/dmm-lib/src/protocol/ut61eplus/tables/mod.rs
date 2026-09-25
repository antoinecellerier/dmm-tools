pub mod ut202bt;
pub mod ut60bt;
pub mod ut61b_plus;
pub mod ut61d_plus;
pub mod ut61e_plus;

use super::mode::Mode;
use crate::protocol::CaptureStep;
use crate::protocol::cycle::{self, DialPosition};
use std::borrow::Cow;

/// Information about a specific measurement range.
#[derive(Debug, Clone)]
pub struct RangeInfo {
    pub label: &'static str,
    pub unit: &'static str,
}

/// One range table entry. The manuals' full-scale limits are recorded in
/// `docs/research/ut61-family/reverse-engineered-protocol.md`, section 9,
/// and nowhere in the code: a software overload check or bar-graph scaling
/// would take them from there.
pub(crate) const fn r(label: &'static str, unit: &'static str) -> RangeInfo {
    RangeInfo { label, unit }
}

/// A range byte the model's table skips: the rungs above it exist, this one
/// does not. The UT202BT's LPF modes are listed at range byte 2 only, its °C
/// at byte 1. A gap reads as no range at all, and a table with one offers no
/// RANGE ladder.
pub(crate) const GAP: RangeInfo = r("", "");

impl RangeInfo {
    fn is_gap(&self) -> bool {
        self.label.is_empty()
    }
}

/// A [`Mode`] as the `mode_raw` value the dial tables are written in.
pub(crate) const fn m(mode: Mode) -> u16 {
    mode as u16
}

/// Look up a range entry by index. Shared by all device table implementations.
fn lookup_range(table: &[RangeInfo], range: u8) -> Option<&RangeInfo> {
    table.get(range as usize).filter(|r| !r.is_gap())
}

/// Trait for device-specific range/unit lookup tables.
pub trait DeviceTable: Send {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo>;
    fn model_name(&self) -> &'static str;

    /// The model's dial positions, for the cycle-to-target mode driver in
    /// `protocol::cycle`.
    ///
    /// Empty — the default — means this model's dial is not described, so the
    /// meter simply offers no remote mode selection.
    fn dial_positions(&self) -> &'static [DialPosition] {
        &[]
    }

    /// The range labels this model lists for `mode`, in range-byte order;
    /// a skipped range byte is a [`GAP`].
    fn ranges(&self, _mode: Mode) -> &[RangeInfo] {
        &[]
    }

    /// Every mode the model reaches, for a model whose dial is not described
    /// ([`DeviceTable::dial_positions`] empty); empty for one whose dial is.
    fn modes(&self) -> &'static [Mode] {
        &[]
    }

    /// Whether RANGE is dead in `mode` on this model, so no ladder is
    /// offered even though the table lists more than one entry.
    fn range_is_fixed(&self, _mode: Mode) -> bool {
        false
    }

    /// Modes where the Peak command (0x4D) does something on this model.
    ///
    /// Empty — the default — means the model has no Peak at all, which is
    /// the UT61B+: the family spec's flag matrix (§4) leaves its Peak MAX
    /// and Peak MIN cells blank and its command matrix (§6) marks 0x4D/0x4E
    /// "No effect".
    fn peak_modes(&self) -> &'static [Mode] {
        &[]
    }

    /// The commands this model's buttons take, by the names `send_command`
    /// answers. Default: the family's whole list.
    fn commands(&self) -> &'static [&'static str] {
        super::UT61EPLUS_COMMANDS
    }

    /// Whether the meter wants Get Name answered before it starts streaming
    /// over Bluetooth (family spec §6.4). Default: no.
    fn name_before_stream(&self) -> bool {
        false
    }

    /// What the capture asks for the duty-cycle step.
    fn duty_instruction(&self) -> &'static str {
        FAMILY_DUTY_INSTRUCTION
    }

    /// The model's own capture steps, for one whose buttons the family list
    /// does not describe; `None`, the default, for the family list.
    fn capture_steps(&self) -> Option<Vec<CaptureStep>> {
        None
    }
}

/// The duty-cycle step on a UT61+: its USB/Hz button.
const FAMILY_DUTY_INSTRUCTION: &str = "Hz/% position: short-press the USB button for Duty %.";

/// Modes where the RANGE button (0x46) does nothing on any model of the family.
///
/// [VERIFIED] on two meters: a RANGE press left a UT61E+ in Auto in both
/// (`ut61eplus-verify4.yaml`, steps `capacitance/range:22nF` and
/// `hz/range:22Hz`) and a UT61B+ likewise (issue #19, the same steps at 60nF
/// and 60Hz). The tables still name these modes' rungs — auto-ranging reaches
/// them and a reading has to be labelled; it is only the button that is dead.
pub(crate) const FAMILY_FIXED_RANGE_MODES: &[Mode] = &[Mode::Capacitance, Mode::Hz];

/// The AC modes where Peak is offered on the models that have it.
///
/// Verified on a UT61E+ only for AC mV, where 0x4D activates while DC V
/// ignores it (docs/verification-backlog.md, "MIN/MAX and Peak measurement
/// reporting"). The manual describes Peak as an AC-waveform measurement, so
/// the other pure-AC modes are offered with it; the AC+DC and LPF variants
/// are left out until a meter says otherwise (backlog, UT61E+ section).
pub(crate) const AC_PEAK_MODES: &[Mode] =
    &[Mode::AcV, Mode::AcMv, Mode::AcUa, Mode::AcMa, Mode::AcA];

/// The manual range ladder to offer in `mode`, or empty when there is none.
///
/// The table is the ladder: the meter reports the range byte as an index
/// into it, so rung `n` of the choice list is entry `n - 1`. Two table
/// shapes are not ladders and [`cycle::usable_ladder`] drops both, a table
/// with a [`GAP`] is not one either, and a model can say outright that RANGE
/// does nothing in a mode ([`DeviceTable::range_is_fixed`]).
pub(crate) fn range_ladder(table: &dyn DeviceTable, mode: Mode) -> Vec<Cow<'static, str>> {
    if table.range_is_fixed(mode) || table.ranges(mode).iter().any(RangeInfo::is_gap) {
        return Vec::new();
    }
    cycle::usable_ladder(
        table
            .ranges(mode)
            .iter()
            .map(|r| Cow::Borrowed(r.label))
            .collect(),
    )
}

/// Per-model data behind `DeviceTable`. The specs are the manual's tables in
/// `ut61eplus/specs/`.
pub(crate) trait ModeTables: Send {
    /// Model name reported by `DeviceTable::model_name`. An associated const
    /// rather than a method so it cannot collide with the trait method the
    /// blanket impl below derives from it.
    const MODEL_NAME: &'static str;

    /// Dial table returned by `DeviceTable::dial_positions`.
    const DIAL_POSITIONS: &'static [DialPosition];

    /// Modes returned by `DeviceTable::modes`: set by a table whose
    /// `DIAL_POSITIONS` is empty, and only by one.
    const MODES: &'static [Mode] = &[];

    /// The range table for `mode`, in range-byte order; `None` for a mode
    /// this model does not have, or that has no ranges (NCV).
    fn entry(&self, mode: Mode) -> Option<&[RangeInfo]>;

    /// Modes this model's RANGE button cannot change. Default:
    /// [`FAMILY_FIXED_RANGE_MODES`]. A model that fixes more of them
    /// overrides this and folds the default back in.
    fn range_is_fixed(&self, mode: Mode) -> bool {
        FAMILY_FIXED_RANGE_MODES.contains(&mode)
    }

    /// Modes where Peak works on this model. Default: none.
    fn peak_modes(&self) -> &'static [Mode] {
        &[]
    }

    /// Commands returned by `DeviceTable::commands`: set by a model whose
    /// buttons are known to take fewer.
    const COMMANDS: &'static [&'static str] = super::UT61EPLUS_COMMANDS;

    /// Returned by `DeviceTable::duty_instruction`.
    const DUTY_INSTRUCTION: &'static str = FAMILY_DUTY_INSTRUCTION;

    /// Returned by `DeviceTable::name_before_stream`.
    const NAME_BEFORE_STREAM: bool = false;

    /// Returned by `DeviceTable::capture_steps`. Default: the family list.
    fn capture_steps(&self) -> Option<Vec<CaptureStep>> {
        None
    }
}

impl<T: ModeTables> DeviceTable for T {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo> {
        self.entry(mode)
            .and_then(|table| lookup_range(table, range))
    }

    fn model_name(&self) -> &'static str {
        T::MODEL_NAME
    }

    fn dial_positions(&self) -> &'static [DialPosition] {
        T::DIAL_POSITIONS
    }

    fn ranges(&self, mode: Mode) -> &[RangeInfo] {
        self.entry(mode).unwrap_or(&[])
    }

    fn modes(&self) -> &'static [Mode] {
        T::MODES
    }

    fn range_is_fixed(&self, mode: Mode) -> bool {
        ModeTables::range_is_fixed(self, mode)
    }

    fn peak_modes(&self) -> &'static [Mode] {
        ModeTables::peak_modes(self)
    }

    fn commands(&self) -> &'static [&'static str] {
        T::COMMANDS
    }

    fn duty_instruction(&self) -> &'static str {
        T::DUTY_INSTRUCTION
    }

    fn name_before_stream(&self) -> bool {
        T::NAME_BEFORE_STREAM
    }

    fn capture_steps(&self) -> Option<Vec<CaptureStep>> {
        ModeTables::capture_steps(self)
    }
}

#[cfg(test)]
mod tests {
    use super::ut61b_plus::Ut61bPlusTable;
    use super::ut61d_plus::Ut61dPlusTable;
    use super::ut61e_plus::Ut61ePlusTable;
    use super::*;
    use crate::protocol::cycle::{self, CycleButton};
    use std::borrow::Cow;

    /// Hz and Duty % are reported with the same byte whichever position
    /// produced them, so they are the only modes allowed on more than one.
    const SHARED: &[u16] = &[m(Mode::Hz), m(Mode::DutyCycle)];
    const BUTTONS: &[CycleButton] = &[CycleButton::Select, CycleButton::Hz];

    const DIAL_TABLES: [(&str, &[DialPosition]); 3] = [
        ("UT61E+", Ut61ePlusTable::DIAL_POSITIONS),
        ("UT61B+", Ut61bPlusTable::DIAL_POSITIONS),
        ("UT61D+", Ut61dPlusTable::DIAL_POSITIONS),
    ];

    fn label(mode: u16) -> Cow<'static, str> {
        match u8::try_from(mode).map(Mode::from_byte) {
            Ok(Ok(mode)) => Cow::Borrowed(mode.as_static_str()),
            _ => Cow::Owned(format!("Unknown({mode:#04x})")),
        }
    }

    /// The modes of the single position that reaches `mode`, sorted.
    fn modes_at(positions: &[DialPosition], mode: Mode) -> Vec<u16> {
        let found: Vec<_> = positions.iter().filter(|p| p.contains(m(mode))).collect();
        assert_eq!(found.len(), 1, "{mode:?} should be on exactly one position");
        sorted(found[0].modes())
    }

    fn all_modes(positions: &[DialPosition]) -> Vec<u16> {
        sorted(positions.iter().flat_map(|p| p.modes()).collect())
    }

    fn sorted(mut modes: Vec<u16>) -> Vec<u16> {
        modes.sort_unstable();
        modes.dedup();
        modes
    }

    #[test]
    fn every_dial_table_is_well_formed() {
        for (model, positions) in DIAL_TABLES {
            println!("checking {model}");
            cycle::assert_table_invariants(positions, SHARED, BUTTONS, &label);
        }
    }

    /// Every mode a table lists must be one the parser can produce, or the
    /// driver would offer a switch to something no reading can confirm.
    #[test]
    fn every_dial_table_mode_round_trips_through_from_byte() {
        for (model, positions) in DIAL_TABLES {
            for mode in all_modes(positions) {
                let byte = u8::try_from(mode).unwrap_or_else(|_| panic!("{model}: {mode:#06x}"));
                let parsed = Mode::from_byte(byte)
                    .unwrap_or_else(|_| panic!("{model}: {byte:#04x} is not a mode"));
                assert_eq!(m(parsed), mode, "{model}");
            }
        }
    }

    /// The E+ reaches AC+DC and LPF; its V~ position pairs LPF with the Hz
    /// ring, joined at AC V.
    #[test]
    fn ut61e_plus_ac_volts_reaches_lpf_and_the_hz_ring() {
        assert_eq!(
            modes_at(Ut61ePlusTable::DIAL_POSITIONS, Mode::AcV),
            sorted(vec![
                m(Mode::AcV),
                m(Mode::LpfV),
                m(Mode::Hz),
                m(Mode::DutyCycle)
            ])
        );
    }

    /// The B+ has neither AC+DC nor LPF, so its V~ position has no SELECT
    /// ring at all — only the Hz/% one.
    #[test]
    fn ut61b_plus_ac_volts_has_only_the_hz_ring() {
        let positions = Ut61bPlusTable::DIAL_POSITIONS;
        assert_eq!(
            modes_at(positions, Mode::AcV),
            sorted(vec![m(Mode::AcV), m(Mode::Hz), m(Mode::DutyCycle)])
        );
        let ac_v = positions
            .iter()
            .find(|p| p.contains(m(Mode::AcV)))
            .expect("AC V position");
        assert!(
            ac_v.rings.iter().all(|r| r.button == CycleButton::Hz),
            "the UT61B+ V~ position has no SELECT ring"
        );
    }

    /// The D+ puts AC and DC volts on one dial position, so SELECT swaps them.
    #[test]
    fn ut61d_plus_combines_ac_and_dc_volts_on_one_position() {
        assert_eq!(
            modes_at(Ut61dPlusTable::DIAL_POSITIONS, Mode::AcV),
            sorted(vec![
                m(Mode::AcV),
                m(Mode::DcV),
                m(Mode::Hz),
                m(Mode::DutyCycle)
            ])
        );
    }

    /// Manual §11: "Short press the SELECT button to switch between °C and °F".
    #[test]
    fn ut61d_plus_switches_temperature_units_with_select() {
        let positions = Ut61dPlusTable::DIAL_POSITIONS;
        assert_eq!(
            modes_at(positions, Mode::TempC),
            sorted(vec![m(Mode::TempC), m(Mode::TempF)])
        );
        let temp = positions
            .iter()
            .find(|p| p.contains(m(Mode::TempC)))
            .expect("temperature position");
        assert_eq!(temp.rings.len(), 1);
        assert_eq!(temp.rings[0].button, CycleButton::Select);
    }

    /// A table must not offer a mode the model does not have: the driver would
    /// press the ring all the way round looking for it.
    #[test]
    fn dial_tables_list_no_mode_the_model_lacks() {
        let b_plus = all_modes(Ut61bPlusTable::DIAL_POSITIONS);
        for absent in [Mode::Hfe, Mode::TempC, Mode::TempF, Mode::LozV] {
            assert!(!b_plus.contains(&m(absent)), "UT61B+ has no {absent:?}");
        }
        // 0x15/0x16/0x17 were unreachable from every UT61E+ dial position
        // (backlog, "Modes not reachable on UT61E+"); the protocol deck makes
        // them LoZ and two clamp functions.
        let e_plus = all_modes(Ut61ePlusTable::DIAL_POSITIONS);
        for absent in [Mode::LozV, Mode::ClampAcA, Mode::ClampDcA] {
            assert!(
                !e_plus.contains(&m(absent)),
                "UT61E+ cannot reach {absent:?} ({:#04x})",
                m(absent)
            );
        }
    }

    /// A table without a dial lists its modes instead: each one it has
    /// ranges for, and NCV, which has none. A table with a dial leaves the
    /// list empty.
    #[test]
    fn dial_less_tables_list_exactly_their_modes() {
        let dial_less: [&dyn DeviceTable; 2] = [
            &super::ut60bt::Ut60btTable::new(),
            &super::ut202bt::Ut202btTable::new(),
        ];
        for table in dial_less {
            assert!(table.dial_positions().is_empty());
            for &mode in Mode::ALL {
                let listed = table.modes().contains(&mode);
                let has = mode == Mode::Ncv || !table.ranges(mode).is_empty();
                assert_eq!(listed, has, "{}: {mode:?}", table.model_name());
            }
        }
        let dialled: [&dyn DeviceTable; 3] = [
            &Ut61ePlusTable::new(),
            &Ut61bPlusTable::new(),
            &Ut61dPlusTable::new(),
        ];
        for table in dialled {
            assert!(table.modes().is_empty(), "{}", table.model_name());
        }
    }

    /// A gap is no range: it has no label to show and no rung to press to.
    #[test]
    fn a_gap_reads_as_no_range_and_breaks_the_ladder() {
        struct Sparse;
        impl ModeTables for Sparse {
            const MODEL_NAME: &'static str = "sparse";
            const DIAL_POSITIONS: &'static [DialPosition] = &[];
            fn entry(&self, mode: Mode) -> Option<&[RangeInfo]> {
                const ROW: [RangeInfo; 3] = [GAP, r("2V", "V"), r("20V", "V")];
                (mode == Mode::DcV).then_some(&ROW[..])
            }
        }
        assert!(Sparse.range_info(Mode::DcV, 0).is_none());
        assert_eq!(Sparse.range_info(Mode::DcV, 1).map(|r| r.label), Some("2V"));
        assert!(range_ladder(&Sparse, Mode::DcV).is_empty());
    }

    /// UT161B has no table of its own — `Ut61PlusProtocol::for_model` hands it
    /// the UT61B+'s, so these are the ranges a UT161B reports.
    #[test]
    fn ut161b_uses_the_ut61b_plus_table() {
        let t = Ut61bPlusTable::new();
        assert_eq!(t.model_name(), "UNI-T UT61B+");
        assert_eq!(t.range_info(Mode::DcV, 0).map(|r| r.label), Some("6V"));
    }
}

#[cfg(test)]
mod spec_tables {
    //! Section 9 of the family spec prints every rung of every table and says
    //! it is generated from the source, but nothing generated it: commit
    //! 6406037 corrected the UT61B+/UT61D+ voltage ladders in code and §5.1
    //! and left §9 printing the six-rung ones for a day. This reads the rows
    //! back and holds them to the tables.

    use super::*;
    use crate::protocol::ut61eplus::mode::Mode;
    use std::collections::{BTreeSet, HashMap};

    /// The `### <model>` heading, its `Source: \`<file>.rs\`` line and the
    /// table beneath it, for each model section of §9.
    fn sections(spec: &str) -> Vec<(String, Vec<String>)> {
        let mut out = Vec::new();
        let mut current: Option<(String, Vec<String>)> = None;
        for line in spec.lines() {
            if let Some(rest) = line.strip_prefix("Source: `") {
                if let Some(done) = current.take() {
                    out.push(done);
                }
                let file = rest.split('`').next().unwrap_or_default().to_string();
                current = Some((file, Vec::new()));
            } else if line.starts_with("### ") || line.starts_with("## ") {
                if let Some(done) = current.take() {
                    out.push(done);
                }
            } else if let Some((_, rows)) = current.as_mut()
                && line.starts_with("| `")
            {
                rows.push(line.to_string());
            }
        }
        out.extend(current);
        out
    }

    /// `| \`dc_v\` | DcV, AcDcV | 0 | 2.2V | V | 2.2 | -2.2 |` — every mode
    /// the row's ladder is shared by, its index, label and unit.
    fn parse(row: &str) -> (Vec<String>, u8, String, String) {
        let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
        let modes = cells[1].split(',').map(|m| m.trim().to_string()).collect();
        (
            modes,
            cells[2].parse().expect("index"),
            cells[3].to_string(),
            cells[4].to_string(),
        )
    }

    fn mode_named(name: &str) -> Mode {
        *Mode::ALL
            .iter()
            .find(|m| format!("{m:?}") == name)
            .unwrap_or_else(|| panic!("spec names a mode that does not exist: {name}"))
    }

    fn table_for(source: &str) -> Box<dyn DeviceTable> {
        match source {
            "ut61e_plus.rs" => Box::new(ut61e_plus::Ut61ePlusTable::new()),
            "ut61b_plus.rs" => Box::new(ut61b_plus::Ut61bPlusTable::new()),
            "ut61d_plus.rs" => Box::new(ut61d_plus::Ut61dPlusTable::new()),
            "ut60bt.rs" => Box::new(ut60bt::Ut60btTable::new()),
            "ut202bt.rs" => Box::new(ut202bt::Ut202btTable::new()),
            other => panic!("spec section 9 names an unknown source file: {other}"),
        }
    }

    #[test]
    fn spec_section_9_matches_the_range_tables() {
        let spec = include_str!(
            "../../../../../../docs/research/ut61-family/reverse-engineered-protocol.md"
        );
        let sections = sections(spec);
        assert_eq!(sections.len(), 5, "expected one section per model");

        for (source, rows) in sections {
            let table = table_for(&source);
            // Keyed by mode byte: `Mode` is not `Ord`.
            let mut printed: HashMap<u8, BTreeSet<u8>> = HashMap::new();
            for row in &rows {
                let (modes, idx, label, unit) = parse(row);
                for name in modes {
                    let mode = mode_named(&name);
                    let info = table
                        .range_info(mode, idx)
                        .unwrap_or_else(|| panic!("{source}: {name} has no rung {idx}"));
                    assert_eq!(info.label, label, "{source}: {name} rung {idx} label");
                    assert_eq!(info.unit, unit, "{source}: {name} rung {idx} unit");
                    printed.entry(mode as u8).or_default().insert(idx);
                }
            }
            // The other direction: a rung added in code must reach the spec.
            // A gap is no rung, so it has no row.
            for &mode in Mode::ALL {
                let want: BTreeSet<u8> = (0..table.ranges(mode).len() as u8)
                    .filter(|&idx| table.range_info(mode, idx).is_some())
                    .collect();
                if want.is_empty() {
                    continue;
                }
                assert_eq!(
                    printed.get(&(mode as u8)).cloned().unwrap_or_default(),
                    want,
                    "{source}: section 9 does not print every rung of {mode:?}"
                );
            }
        }
    }
}
