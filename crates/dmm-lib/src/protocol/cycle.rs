//! Reaching a mode on meters whose only mode control is a front-panel button.
//!
//! A meter like the UT181A takes an absolute "be in this mode" command. Most
//! meters take none: the dial picks a *position*, and within that position a
//! button steps through the functions it offers, one press at a time. This
//! module turns "put the meter in Hz" into the presses that get there, so each
//! family only has to say which buttons it has, which modes each one reaches,
//! how to press one, and how to read the mode back.
//!
//! # The model
//!
//! * A **position** is one setting of the dial. The host can't move it — the
//!   dial is the hard boundary of what remote mode selection can do.
//! * A position has one or two **rings**: a button plus the set of modes that
//!   button cycles through. A ring is *membership only*. The press order is
//!   never assumed, because it isn't reliably known and can differ between
//!   models sharing a protocol; the driver presses and reads the mode back
//!   until the target shows up, which works whatever the real order is.
//! * A two-ring position has exactly one **junction** mode, the one both
//!   buttons reach (on a UT61E+ V~ position, AC V is in both the SELECT ring
//!   {AC V, LPF V} and the Hz/% ring {AC V, Hz, Duty %}). Crossing from one
//!   ring to the other means walking to the junction with the first button,
//!   then away from it with the second.
//!
//! # What the driver relies on
//!
//! The meter's own reading is the only feedback: every family covered here
//! reports a mode byte that names the sub-mode, so a press can be confirmed by
//! reading. Nothing is assumed about how long that takes beyond the family's
//! [`Settle`], and a reading that still shows the old mode triggers another
//! read, never another press — a second press would step past the target.
//!
//! A family implements [`CycleMeter`], keeps a [`DialState`] updated with
//! [`DialState::observe`] from every measurement it parses, and forwards
//! `Protocol::choices` / `Protocol::select` to the free functions
//! here.

use log::debug;
use std::borrow::Cow;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::Choice;
use crate::transport::Transport;

/// A front-panel button that cycles the meter through a ring of modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleButton {
    /// The SELECT / FUNC button: steps the dial position's main functions.
    Select,
    /// The Hz/% button: steps a position's frequency and duty-cycle modes.
    Hz,
}

/// One button and the modes (family `mode_raw` values) it cycles through.
///
/// Membership only — see the module docs. The order the meter walks them in
/// is discovered by pressing, not declared here.
#[derive(Debug)]
pub struct Ring {
    pub(crate) button: CycleButton,
    pub(crate) modes: &'static [u16],
}

/// One position of the dial: one or two rings; two rings share exactly one
/// junction mode.
#[derive(Debug)]
pub struct DialPosition {
    pub(crate) rings: &'static [Ring],
}

impl DialPosition {
    /// Whether any of this position's rings reaches `mode`.
    pub(crate) fn contains(&self, mode: u16) -> bool {
        self.rings.iter().any(|r| r.modes.contains(&mode))
    }

    /// The mode both rings reach, or `None` for a single-ring position.
    ///
    /// A table with two rings and no shared mode is a table bug — the second
    /// ring would be unreachable — which is why
    /// [`assert_table_invariants`] checks for exactly one.
    pub(crate) fn junction(&self) -> Option<u16> {
        let [first, second] = self.rings else {
            return None;
        };
        first
            .modes
            .iter()
            .copied()
            .find(|m| second.modes.contains(m))
    }

    /// Every mode this position reaches, ring by ring, the junction listed
    /// once — the order the choice list is offered in.
    pub(crate) fn modes(&self) -> Vec<u16> {
        let mut out: Vec<u16> = Vec::new();
        for ring in self.rings {
            for &mode in ring.modes {
                if !out.contains(&mode) {
                    out.push(mode);
                }
            }
        }
        out
    }

    /// How many distinct modes this position reaches — the tie-break key when
    /// a mode sits on more than one position.
    pub(crate) fn mode_count(&self) -> usize {
        let total: usize = self.rings.iter().map(|r| r.modes.len()).sum();
        // The junction is in both rings; counting it twice would make a
        // two-ring position look bigger than it is.
        total - usize::from(self.junction().is_some())
    }

    /// The ring holding `mode`, or `None` when this position doesn't reach it.
    fn ring_for(&self, mode: u16) -> Option<&Ring> {
        self.rings.iter().find(|r| r.modes.contains(&mode))
    }
}

/// How long to wait after a press before the reading is expected to show it,
/// and how many readings to take before concluding the press changed nothing.
///
/// A meter that is mid-frame when the press lands reports the old mode once
/// more; that stale frame must cost a re-read, never a second press, which
/// would overshoot the target.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Settle {
    pub(crate) delay: Duration,
    pub(crate) reads: usize,
}

/// What the driver remembers between readings: the last mode and the dial
/// position it is inferred to be on (index into the family's table).
#[derive(Debug, Default)]
pub(crate) struct DialState {
    last_mode: Option<u16>,
    position: Option<usize>,
}

impl DialState {
    /// The mode of the last reading recorded.
    pub(crate) fn last_mode(&self) -> Option<u16> {
        self.last_mode
    }

    /// The dial position the meter is inferred to be on.
    pub(crate) fn position(&self) -> Option<usize> {
        self.position
    }

    /// Record `mode` from a reading and re-infer the dial position.
    ///
    /// The current position is kept as long as it still reaches `mode`,
    /// because some modes carry no dial information: a UT61E+ reports Hz and
    /// Duty % identically from the V~, mV and Hz positions, and a VC-880
    /// reports code 0x02 from two of its positions. Without history such a
    /// mode lands on the *smallest* position holding it (ties: the first
    /// listed) — but only when that position's modes are a subset of every
    /// other candidate's, which is what makes it the safe guess: a switch
    /// planned from it is valid wherever the dial really is. Candidates that
    /// overlap without nesting (the VC-880's mV and V positions) leave the
    /// position unknown until a reading names one of them outright.
    pub(crate) fn observe(&mut self, positions: &[DialPosition], mode: u16) {
        self.last_mode = Some(mode);
        self.position = resolve_position(positions, self.position, mode);
    }
}

/// The position a reading of `mode` puts the meter on, preferring `hint`.
fn resolve_position(positions: &[DialPosition], hint: Option<usize>, mode: u16) -> Option<usize> {
    if let Some(i) = hint
        && positions.get(i).is_some_and(|p| p.contains(mode))
    {
        return Some(i);
    }
    let candidates: Vec<(usize, &DialPosition)> = positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p.contains(mode))
        .collect();
    // `min_by_key` keeps the first of equal keys, so ties go to the position
    // listed first.
    let (i, smallest) = candidates.iter().min_by_key(|(_, p)| p.mode_count())?;
    let nested = candidates
        .iter()
        .all(|(_, p)| smallest.modes().iter().all(|&m| p.contains(m)));
    nested.then_some(*i)
}

/// A meter whose modes are reached by pressing front-panel buttons.
pub(crate) trait CycleMeter {
    /// The family's dial table.
    fn dial_positions(&self) -> &'static [DialPosition];

    fn dial_state(&self) -> &DialState;

    fn dial_state_mut(&mut self) -> &mut DialState;

    /// Press one button: write the frame plus whatever ack or drain the
    /// family needs. Does not read a measurement.
    fn press(&mut self, transport: &dyn Transport, button: CycleButton) -> Result<()>;

    /// Take one reading and return its `mode_raw`.
    fn read_mode(&mut self, transport: &dyn Transport) -> Result<u16>;

    /// Display name in the same vocabulary as `Measurement::mode`.
    fn mode_label(&self, mode: u16) -> Cow<'static, str>;

    fn settle(&self) -> Settle;

    /// What the meter's front panel calls the button, for error text.
    fn button_name(&self, button: CycleButton) -> &'static str {
        match button {
            CycleButton::Select => "SELECT",
            CycleButton::Hz => "Hz/%",
        }
    }
}

/// Modes reachable from the reading `current`, for `Protocol::choices`.
///
/// Empty when the mode is on no position of the family's table. Does not
/// touch the meter's state: a caller may hand back an older reading, and a
/// list of choices is not an observation.
pub(crate) fn mode_choices<M: CycleMeter + ?Sized>(
    meter: &M,
    current: &Measurement,
) -> Vec<Choice> {
    let positions = meter.dial_positions();
    let Some(index) = resolve_position(positions, meter.dial_state().position(), current.mode_raw)
    else {
        return Vec::new();
    };
    positions[index]
        .modes()
        .into_iter()
        .map(|id| Choice {
            id,
            label: meter.mode_label(id),
            current: id == current.mode_raw,
        })
        .collect()
}

/// Press the meter into mode `id`, for `Protocol::select`.
///
/// Everything that can be decided without touching the meter is decided
/// first: an id on another dial position costs no I/O at all.
pub(crate) fn select_mode<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    id: u16,
) -> Result<()> {
    let positions = meter.dial_positions();
    // The stream is the only place the meter states its mode, so a command
    // issued before the first reading takes one itself.
    let current = match meter.dial_state().last_mode() {
        Some(mode) => mode,
        None => {
            let mode = meter.read_mode(transport)?;
            meter.dial_state_mut().observe(positions, mode);
            mode
        }
    };

    let Some(index) = resolve_position(positions, meter.dial_state().position(), current) else {
        let known = positions.iter().any(|p| p.contains(current));
        return Err(Error::UnsupportedCommand(if known {
            format!(
                "{} is reported from more than one dial position — take a reading in another mode first",
                meter.mode_label(current)
            )
        } else {
            format!("{} is on no known dial position", meter.mode_label(current))
        }));
    };

    if id == current {
        return Ok(());
    }

    let Some(legs) = plan(&positions[index], current, id) else {
        return Err(Error::UnsupportedCommand(format!(
            "mode {} is not reachable from {} — turn the dial first",
            meter.mode_label(id),
            meter.mode_label(current)
        )));
    };

    let mut from = current;
    for leg in legs {
        if let Err(e) = walk(meter, transport, &leg, from) {
            // `walk` says where its own presses left the meter; when an
            // earlier leg already moved it, say where it came from too, so
            // the user knows the function changed under the failure.
            return Err(match e {
                Error::CommandRejected(detail) if from != current => Error::CommandRejected(
                    format!("{detail}; started in {}", meter.mode_label(current)),
                ),
                other => other,
            });
        }
        from = leg.target;
    }
    Ok(())
}

/// One button walked from wherever the meter is to `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Leg {
    pub(crate) button: CycleButton,
    /// Modes in the ring being walked — the press budget comes from it.
    pub(crate) ring_len: usize,
    pub(crate) target: u16,
}

/// The legs that take `position` from `from` to `to`.
///
/// `Some(vec![])` when the meter is already there, one leg when a single ring
/// holds both modes, two when they sit in different rings and the junction has
/// to be crossed. `None` when either mode is on no ring of this position —
/// the caller turns that into a "turn the dial first" error before any write.
pub(crate) fn plan(position: &DialPosition, from: u16, to: u16) -> Option<Vec<Leg>> {
    if !position.contains(from) || !position.contains(to) {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    if let Some(ring) = position
        .rings
        .iter()
        .find(|r| r.modes.contains(&from) && r.modes.contains(&to))
    {
        return Some(vec![Leg {
            button: ring.button,
            ring_len: ring.modes.len(),
            target: to,
        }]);
    }
    // Different rings, so neither mode is the junction: walk `from`'s ring to
    // the junction, then the other ring away from it.
    let junction = position.junction()?;
    let from_ring = position.ring_for(from)?;
    let to_ring = position.ring_for(to)?;
    Some(vec![
        Leg {
            button: from_ring.button,
            ring_len: from_ring.modes.len(),
            target: junction,
        },
        Leg {
            button: to_ring.button,
            ring_len: to_ring.modes.len(),
            target: to,
        },
    ])
}

/// Press one button until the meter reports `leg.target`.
///
/// `start` is the mode before the first press. Three things end the walk
/// early, and all three are the meter's answer rather than a timeout: the
/// press changed nothing (wrong dial position for that button), the ring came
/// back around to where it started without the target showing (the table
/// claims a mode this model doesn't have), or the press budget ran out.
fn walk<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    leg: &Leg,
    start: u16,
) -> Result<()> {
    let settle = meter.settle();
    // One more than the ring length: a healthy ring needs at most one press
    // per other mode, and the spare press keeps an unexpected extra step from
    // reading as a failure.
    let budget = leg.ring_len + 1;
    let mut seen = start;
    for _ in 0..budget {
        debug!(
            "cycle: pressing {} (in {}, want {})",
            meter.button_name(leg.button),
            meter.mode_label(seen),
            meter.mode_label(leg.target)
        );
        meter.press(transport, leg.button)?;
        let mode = observe_after_press(meter, transport, seen, settle)?;
        if mode == leg.target {
            return Ok(());
        }
        if mode == seen {
            return Err(Error::CommandRejected(format!(
                "{} did nothing in {}",
                meter.button_name(leg.button),
                meter.mode_label(seen)
            )));
        }
        if mode == start {
            return Err(Error::CommandRejected(format!(
                "{} never appeared; the meter is back in {}",
                meter.mode_label(leg.target),
                meter.mode_label(start)
            )));
        }
        seen = mode;
    }
    Err(Error::CommandRejected(format!(
        "gave up after {budget} presses of {}; the meter is in {}",
        meter.button_name(leg.button),
        meter.mode_label(seen)
    )))
}

/// Read the mode back after a press, skipping frames that still show `seen`.
///
/// Waits `settle.delay` before each read and takes up to `settle.reads` of
/// them (always at least one), stopping at the first reading that differs.
/// Returns `seen` when none does — the caller decides what that means.
fn observe_after_press<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    seen: u16,
    settle: Settle,
) -> Result<u16> {
    let positions = meter.dial_positions();
    let mut last = seen;
    for _ in 0..settle.reads.max(1) {
        if !settle.delay.is_zero() {
            std::thread::sleep(settle.delay);
        }
        last = meter.read_mode(transport)?;
        meter.dial_state_mut().observe(positions, last);
        if last != seen {
            break;
        }
        debug!(
            "cycle: meter still reports {}, re-reading",
            meter.mode_label(seen)
        );
    }
    Ok(last)
}

/// Panic unless a family's dial table is well formed.
///
/// For family tests to call on their own table: every mistake checked here
/// produces a driver that misbehaves against real hardware rather than
/// failing to compile.
///
/// `shared_modes` lists the modes a family reports identically from several
/// positions (UT61E+ Hz and Duty %); every other mode must be on at most one.
#[cfg(test)]
pub(crate) fn assert_table_invariants(
    positions: &[DialPosition],
    shared_modes: &[u16],
    allowed_buttons: &[CycleButton],
    label: &dyn Fn(u16) -> Cow<'static, str>,
) {
    if let Err(msg) = check_table(positions, shared_modes, allowed_buttons, label) {
        panic!("{msg}");
    }
}

/// The body of [`assert_table_invariants`], as a `Result` so the checks can
/// themselves be tested against a deliberately broken table.
#[cfg(test)]
fn check_table(
    positions: &[DialPosition],
    shared_modes: &[u16],
    allowed_buttons: &[CycleButton],
    label: &dyn Fn(u16) -> Cow<'static, str>,
) -> std::result::Result<(), String> {
    let mut seen: Vec<u16> = Vec::new();
    for (i, position) in positions.iter().enumerate() {
        if position.rings.is_empty() || position.rings.len() > 2 {
            return Err(format!(
                "position {i} has {} rings, want 1 or 2",
                position.rings.len()
            ));
        }
        for ring in position.rings {
            if !allowed_buttons.contains(&ring.button) {
                return Err(format!(
                    "position {i} uses {:?}, which this meter does not have",
                    ring.button
                ));
            }
            if ring.modes.is_empty() {
                return Err(format!("position {i} has an empty {:?} ring", ring.button));
            }
            for (n, &mode) in ring.modes.iter().enumerate() {
                if ring.modes[..n].contains(&mode) {
                    return Err(format!(
                        "position {i} lists {mode:#04x} twice in its {:?} ring",
                        ring.button
                    ));
                }
                let name = label(mode);
                if name.starts_with("Unknown") {
                    return Err(format!("position {i} lists {mode:#04x}, which has no name"));
                }
            }
        }
        if position.rings.len() == 2 && position.junction().is_none() {
            return Err(format!(
                "position {i} has two rings that share no mode, so its second button is unreachable"
            ));
        }
        if let [first, second] = position.rings {
            let shared = first
                .modes
                .iter()
                .filter(|m| second.modes.contains(m))
                .count();
            if shared != 1 {
                return Err(format!(
                    "position {i} rings share {shared} modes, want exactly 1"
                ));
            }
        }
        for mode in position.modes() {
            if shared_modes.contains(&mode) {
                continue;
            }
            if seen.contains(&mode) {
                return Err(format!(
                    "{mode:#04x} is on more than one position but is not listed as shared"
                ));
            }
            seen.push(mode);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::NullTransport;

    // Mode ids of a UT61E+-shaped meter.
    const AC_V: u16 = 0x00;
    const AC_MV: u16 = 0x01;
    const DC_V: u16 = 0x02;
    const DC_MV: u16 = 0x03;
    const HZ: u16 = 0x04;
    const DUTY: u16 = 0x05;
    const OHM: u16 = 0x06;
    const CONT: u16 = 0x07;
    const DIODE: u16 = 0x08;
    const CAP: u16 = 0x09;
    const HFE: u16 = 0x12;
    const LPF_V: u16 = 0x18;
    const ACDC_V: u16 = 0x19;

    // Dial positions, in table order: V~, V⎓, mV, Ω, hFE, Hz.
    const P_V_AC: usize = 0;
    const P_V_DC: usize = 1;
    const P_MV: usize = 2;
    const P_OHM: usize = 3;
    const P_HFE: usize = 4;
    const P_HZ: usize = 5;

    const DIAL: &[DialPosition] = &[
        DialPosition {
            rings: &[
                Ring {
                    button: CycleButton::Select,
                    modes: &[AC_V, LPF_V],
                },
                Ring {
                    button: CycleButton::Hz,
                    modes: &[AC_V, HZ, DUTY],
                },
            ],
        },
        DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[DC_V, ACDC_V],
            }],
        },
        DialPosition {
            rings: &[
                Ring {
                    button: CycleButton::Select,
                    modes: &[DC_MV, AC_MV],
                },
                Ring {
                    button: CycleButton::Hz,
                    modes: &[AC_MV, HZ, DUTY],
                },
            ],
        },
        DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[OHM, CONT, DIODE, CAP],
            }],
        },
        DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[HFE],
            }],
        },
        DialPosition {
            rings: &[Ring {
                button: CycleButton::Hz,
                modes: &[HZ, DUTY],
            }],
        },
    ];

    fn label(mode: u16) -> Cow<'static, str> {
        match mode {
            AC_V => Cow::Borrowed("AC V"),
            AC_MV => Cow::Borrowed("AC mV"),
            DC_V => Cow::Borrowed("DC V"),
            DC_MV => Cow::Borrowed("DC mV"),
            HZ => Cow::Borrowed("Hz"),
            DUTY => Cow::Borrowed("Duty %"),
            OHM => Cow::Borrowed("Ω"),
            CONT => Cow::Borrowed("Continuity"),
            DIODE => Cow::Borrowed("Diode"),
            CAP => Cow::Borrowed("Capacitance"),
            HFE => Cow::Borrowed("hFE"),
            LPF_V => Cow::Borrowed("LPF V"),
            ACDC_V => Cow::Borrowed("AC+DC V"),
            other => Cow::Owned(format!("Unknown({other:#04x})")),
        }
    }

    /// A reading that says nothing but which mode the meter is in.
    fn reading(mode: u16) -> Measurement {
        Measurement {
            mode: label(mode),
            mode_raw: mode,
            ..Measurement::from_payload(&[])
        }
    }

    /// A meter whose real buttons are `rings` — deliberately independent of
    /// the table the driver plans from, so a test can make the two disagree
    /// the way a wrong table disagrees with hardware.
    struct FakeMeter {
        state: DialState,
        rings: Vec<(CycleButton, Vec<u16>)>,
        mode: u16,
        presses: Vec<CycleButton>,
        reads: usize,
        /// Readings after a press that still report the pre-press mode.
        stale: usize,
        stale_left: usize,
        stale_mode: u16,
        settle_reads: usize,
    }

    impl FakeMeter {
        /// A meter that has produced no reading yet.
        fn new(mode: u16, rings: Vec<(CycleButton, Vec<u16>)>) -> Self {
            Self {
                state: DialState::default(),
                rings,
                mode,
                presses: Vec::new(),
                reads: 0,
                stale: 0,
                stale_left: 0,
                stale_mode: mode,
                settle_reads: 1,
            }
        }

        /// A meter one reading in, as the driver normally finds it.
        fn seeded(mode: u16, rings: Vec<(CycleButton, Vec<u16>)>) -> Self {
            let mut meter = Self::new(mode, rings);
            meter.state.observe(DIAL, mode);
            meter
        }

        /// The one-ring SELECT meter of the V⎓ position.
        fn v_dc() -> Self {
            Self::seeded(DC_V, vec![(CycleButton::Select, vec![DC_V, ACDC_V])])
        }

        /// The two-ring meter of the V~ position.
        fn v_ac(mode: u16) -> Self {
            let mut meter = Self::new(
                mode,
                vec![
                    (CycleButton::Select, vec![AC_V, LPF_V]),
                    (CycleButton::Hz, vec![AC_V, HZ, DUTY]),
                ],
            );
            // AC V is on this position alone, so seeing it first is what puts
            // the dial here — Hz and Duty % are reported from three positions
            // and would otherwise resolve to the smallest of them.
            meter.state.observe(DIAL, AC_V);
            meter.state.observe(DIAL, mode);
            meter
        }
    }

    impl CycleMeter for FakeMeter {
        fn dial_positions(&self) -> &'static [DialPosition] {
            DIAL
        }

        fn dial_state(&self) -> &DialState {
            &self.state
        }

        fn dial_state_mut(&mut self) -> &mut DialState {
            &mut self.state
        }

        fn press(&mut self, _transport: &dyn Transport, button: CycleButton) -> Result<()> {
            self.presses.push(button);
            let next = self
                .rings
                .iter()
                .find(|(b, modes)| *b == button && modes.contains(&self.mode))
                .map(|(_, modes)| {
                    let at = modes
                        .iter()
                        .position(|&m| m == self.mode)
                        .expect("ring contains the mode");
                    modes[(at + 1) % modes.len()]
                });
            // A button that doesn't apply to the current mode changes nothing
            // (the real meter just beeps).
            if let Some(next) = next {
                self.stale_mode = self.mode;
                self.mode = next;
                self.stale_left = self.stale;
            }
            Ok(())
        }

        fn read_mode(&mut self, _transport: &dyn Transport) -> Result<u16> {
            self.reads += 1;
            if self.stale_left > 0 {
                self.stale_left -= 1;
                return Ok(self.stale_mode);
            }
            Ok(self.mode)
        }

        fn mode_label(&self, mode: u16) -> Cow<'static, str> {
            label(mode)
        }

        fn settle(&self) -> Settle {
            Settle {
                delay: Duration::ZERO,
                reads: self.settle_reads,
            }
        }
    }

    fn ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().map(|c| c.id).collect()
    }

    fn current_ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().filter(|c| c.current).map(|c| c.id).collect()
    }

    // ---- choices ----

    #[test]
    fn choices_list_both_rings_with_the_junction_once() {
        let meter = FakeMeter::v_ac(AC_V);
        let choices = mode_choices(&meter, &reading(AC_V));
        assert_eq!(ids(&choices), vec![AC_V, LPF_V, HZ, DUTY]);
        assert_eq!(current_ids(&choices), vec![AC_V]);
        assert_eq!(choices[1].label, "LPF V");
    }

    /// Hz reads the same from three positions. With no history to say which
    /// dial position produced it, the smallest one is the safe guess.
    #[test]
    fn a_shared_mode_without_history_picks_the_smallest_position() {
        let meter = FakeMeter::new(HZ, vec![(CycleButton::Hz, vec![HZ, DUTY])]);
        assert_eq!(resolve_position(DIAL, None, HZ), Some(P_HZ));
        let choices = mode_choices(&meter, &reading(HZ));
        assert_eq!(ids(&choices), vec![HZ, DUTY]);
        assert_eq!(current_ids(&choices), vec![HZ]);
    }

    /// ...but a meter that was just in AC mV can only have reached Hz from
    /// the mV position, so the choice list stays that position's.
    #[test]
    fn a_shared_mode_keeps_the_position_history_established() {
        let mut meter = FakeMeter::new(AC_MV, vec![]);
        meter.state.observe(DIAL, AC_MV);
        meter.state.observe(DIAL, HZ);
        assert_eq!(meter.state.position(), Some(P_MV));

        let choices = mode_choices(&meter, &reading(HZ));
        assert_eq!(ids(&choices), vec![DC_MV, AC_MV, HZ, DUTY]);
        assert_eq!(current_ids(&choices), vec![HZ]);
    }

    /// The VC-880 reports 0x02 from its mV and V positions, whose other
    /// modes differ. Neither is a safe guess for the other, so a bare 0x02
    /// resolves nowhere until a reading has named one of them.
    #[test]
    fn a_shared_mode_between_non_nested_positions_resolves_nowhere() {
        const OVERLAP: &[DialPosition] = &[
            DialPosition {
                rings: &[Ring {
                    button: CycleButton::Select,
                    modes: &[0x02, 0x03, 0x04],
                }],
            },
            DialPosition {
                rings: &[Ring {
                    button: CycleButton::Select,
                    modes: &[0x00, 0x01, 0x02],
                }],
            },
        ];
        assert_eq!(resolve_position(OVERLAP, None, 0x02), None);

        let mut state = DialState::default();
        state.observe(OVERLAP, 0x02);
        assert_eq!(state.position(), None);
        state.observe(OVERLAP, 0x00);
        state.observe(OVERLAP, 0x02);
        assert_eq!(
            state.position(),
            Some(1),
            "0x00 put the dial on V, 0x02 keeps it"
        );
    }

    #[test]
    fn a_single_function_position_offers_one_choice() {
        let meter = FakeMeter::seeded(HFE, vec![(CycleButton::Select, vec![HFE])]);
        assert_eq!(meter.state.position(), Some(P_HFE));
        let choices = mode_choices(&meter, &reading(HFE));
        assert_eq!(ids(&choices), vec![HFE]);
        assert_eq!(current_ids(&choices), vec![HFE]);
    }

    #[test]
    fn a_mode_on_no_position_offers_nothing() {
        let meter = FakeMeter::new(0xAA, vec![]);
        assert!(mode_choices(&meter, &reading(0xAA)).is_empty());
    }

    /// The GUI asks for choices on every frame, and may hand back a reading
    /// older than the one that set the state.
    #[test]
    fn listing_choices_does_not_move_the_state() {
        let meter = FakeMeter::v_dc();
        let _ = mode_choices(&meter, &reading(HZ));
        assert_eq!(meter.state.position(), Some(P_V_DC));
        assert_eq!(meter.state.last_mode(), Some(DC_V));
        assert_eq!(meter.reads, 0);
    }

    // ---- planning ----

    #[test]
    fn plan_walks_one_ring_when_it_holds_both_modes() {
        assert_eq!(
            plan(&DIAL[P_V_DC], DC_V, ACDC_V),
            Some(vec![Leg {
                button: CycleButton::Select,
                ring_len: 2,
                target: ACDC_V,
            }])
        );
    }

    #[test]
    fn plan_crosses_the_junction_between_rings() {
        assert_eq!(
            plan(&DIAL[P_V_AC], LPF_V, HZ),
            Some(vec![
                Leg {
                    button: CycleButton::Select,
                    ring_len: 2,
                    target: AC_V,
                },
                Leg {
                    button: CycleButton::Hz,
                    ring_len: 3,
                    target: HZ,
                },
            ])
        );
        assert_eq!(
            plan(&DIAL[P_V_AC], DUTY, LPF_V),
            Some(vec![
                Leg {
                    button: CycleButton::Hz,
                    ring_len: 3,
                    target: AC_V,
                },
                Leg {
                    button: CycleButton::Select,
                    ring_len: 2,
                    target: LPF_V,
                },
            ])
        );
    }

    /// The junction is in both rings, so reaching it or leaving it is a
    /// one-button walk, not a crossing.
    #[test]
    fn plan_to_or_from_the_junction_is_one_leg() {
        let to_junction = plan(&DIAL[P_V_AC], DUTY, AC_V).expect("planned");
        assert_eq!(to_junction.len(), 1);
        assert_eq!(to_junction[0].button, CycleButton::Hz);

        let from_junction = plan(&DIAL[P_V_AC], AC_V, LPF_V).expect("planned");
        assert_eq!(from_junction.len(), 1);
        assert_eq!(from_junction[0].button, CycleButton::Select);
    }

    #[test]
    fn plan_to_the_current_mode_presses_nothing() {
        assert_eq!(plan(&DIAL[P_V_AC], HZ, HZ), Some(Vec::new()));
    }

    #[test]
    fn plan_fails_for_a_mode_this_position_cannot_reach() {
        assert_eq!(plan(&DIAL[P_V_DC], DC_V, OHM), None);
        assert_eq!(plan(&DIAL[P_V_DC], DC_V, 0xAA), None);
        assert_eq!(plan(&DIAL[P_V_DC], OHM, DC_V), None);
    }

    /// A position with one ring has no second button to offer, whatever the
    /// other positions do with the same mode.
    #[test]
    fn plan_never_reaches_for_a_button_the_position_lacks() {
        let legs = plan(&DIAL[P_OHM], OHM, CAP).expect("planned");
        assert!(legs.iter().all(|l| l.button == CycleButton::Select));
        assert_eq!(plan(&DIAL[P_OHM], OHM, HZ), None);
    }

    // ---- driving ----

    #[test]
    fn select_walks_one_ring_to_the_target() {
        let mut meter = FakeMeter::v_dc();
        select_mode(&mut meter, &NullTransport, ACDC_V).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Select]);
        assert_eq!(meter.mode, ACDC_V);
        assert_eq!(meter.state.last_mode(), Some(ACDC_V));
    }

    #[test]
    fn select_crosses_the_junction_in_both_directions() {
        let mut meter = FakeMeter::v_ac(LPF_V);
        select_mode(&mut meter, &NullTransport, HZ).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Select, CycleButton::Hz]);
        assert_eq!(meter.mode, HZ);

        let mut meter = FakeMeter::v_ac(DUTY);
        select_mode(&mut meter, &NullTransport, LPF_V).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Hz, CycleButton::Select]);
        assert_eq!(meter.mode, LPF_V);
    }

    /// A failure on the second leg comes after the first one moved the
    /// meter, so the error says where the walk started as well as where the
    /// presses left it.
    #[test]
    fn a_failed_second_leg_names_the_starting_mode() {
        let mut meter = FakeMeter::new(
            LPF_V,
            vec![
                (CycleButton::Select, vec![AC_V, LPF_V]),
                // A meter whose Hz/% ring has no Duty %: leg two cannot land.
                (CycleButton::Hz, vec![AC_V, HZ]),
            ],
        );
        meter.state.observe(DIAL, AC_V);
        meter.state.observe(DIAL, LPF_V);
        let err = select_mode(&mut meter, &NullTransport, DUTY).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m.contains("back in AC V") && m.ends_with("started in LPF V")),
            "got {err:?}"
        );
        assert_eq!(meter.mode, AC_V, "the first leg's press stands");
    }

    #[test]
    fn selecting_the_current_mode_touches_nothing() {
        let mut meter = FakeMeter::v_dc();
        select_mode(&mut meter, &NullTransport, DC_V).expect("already there");
        assert!(meter.presses.is_empty());
        assert_eq!(meter.reads, 0);
    }

    #[test]
    fn an_id_on_another_dial_position_costs_no_io() {
        for id in [OHM, 0xAA] {
            let mut meter = FakeMeter::v_dc();
            let err = select_mode(&mut meter, &NullTransport, id).unwrap_err();
            assert!(
                matches!(err, Error::UnsupportedCommand(_)),
                "got {err:?}, want UnsupportedCommand"
            );
            assert!(meter.presses.is_empty(), "{id:#04x} pressed a button");
            assert_eq!(meter.reads, 0, "{id:#04x} read the meter");
        }
    }

    #[test]
    fn a_mode_on_no_position_cannot_be_switched_from() {
        let mut meter = FakeMeter::seeded(0xAA, vec![]);
        let err = select_mode(&mut meter, &NullTransport, DC_V).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("no known dial position")),
            "got {err:?}"
        );
        assert!(meter.presses.is_empty());
    }

    /// A fresh process has never read the stream, so the first thing a switch
    /// does is take a reading — nothing is pressed on a guess.
    #[test]
    fn a_switch_without_history_reads_before_pressing() {
        let mut meter = FakeMeter::new(DC_V, vec![(CycleButton::Select, vec![DC_V, ACDC_V])]);
        select_mode(&mut meter, &NullTransport, ACDC_V).expect("switched");
        // The mode read first, then the one confirming the press.
        assert_eq!(meter.reads, 2);
        assert_eq!(meter.presses, vec![CycleButton::Select]);
    }

    /// The table claims four modes on the Ω position but this meter only has
    /// two, so Diode never comes round. The ring returning to where it
    /// started is what says so.
    #[test]
    fn a_ring_coming_back_around_reports_the_missing_mode() {
        let mut meter = FakeMeter::seeded(OHM, vec![(CycleButton::Select, vec![OHM, CONT])]);
        let err = select_mode(&mut meter, &NullTransport, DIODE).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m.contains("Diode never appeared") && m.contains("back in Ω")),
            "got {err:?}"
        );
        assert_eq!(meter.presses.len(), 2, "walked past the whole ring");
        assert_eq!(meter.mode, OHM, "the meter is back where it started");
    }

    /// The dial is on a position whose Hz/% button does nothing here. Every
    /// read shows the same mode, and no further press is attempted.
    #[test]
    fn a_button_that_changes_nothing_is_reported_after_one_press() {
        let mut meter = FakeMeter::seeded(AC_V, vec![(CycleButton::Select, vec![AC_V, LPF_V])]);
        meter.settle_reads = 3;
        let err = select_mode(&mut meter, &NullTransport, HZ).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m.contains("Hz/% did nothing in AC V")),
            "got {err:?}"
        );
        assert_eq!(meter.presses, vec![CycleButton::Hz]);
        assert_eq!(meter.reads, 3, "a silent press is retried by reading");
    }

    /// A ring longer than the table says, whose modes never come back to the
    /// start: only the press budget can stop this walk.
    #[test]
    fn the_walk_gives_up_after_one_press_per_ring_entry_plus_one() {
        let mut meter = FakeMeter::seeded(
            OHM,
            vec![(
                CycleButton::Select,
                vec![OHM, CONT, DC_MV, AC_MV, DC_V, ACDC_V, HFE, LPF_V],
            )],
        );
        let err = select_mode(&mut meter, &NullTransport, CAP).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m.contains("gave up after 5 presses of SELECT")
                    && m.contains("the meter is in AC+DC V")),
            "got {err:?}"
        );
        assert_eq!(meter.presses.len(), 5);
    }

    /// The meter was mid-frame when the press landed. Re-reading is the only
    /// answer: a second press would step past the target.
    #[test]
    fn stale_frames_after_a_press_cost_reads_not_presses() {
        let mut meter = FakeMeter::v_dc();
        meter.stale = 2;
        meter.settle_reads = 3;
        select_mode(&mut meter, &NullTransport, ACDC_V).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Select]);
        assert_eq!(meter.reads, 3);
        assert_eq!(meter.mode, ACDC_V);
    }

    #[test]
    fn a_settle_of_zero_reads_still_reads_once() {
        let mut meter = FakeMeter::v_dc();
        meter.settle_reads = 0;
        select_mode(&mut meter, &NullTransport, ACDC_V).expect("switched");
        assert_eq!(meter.reads, 1);
    }

    // ---- table invariants ----

    #[test]
    fn the_test_table_satisfies_the_invariants() {
        assert_table_invariants(
            DIAL,
            &[HZ, DUTY],
            &[CycleButton::Select, CycleButton::Hz],
            &label,
        );
    }

    #[test]
    fn the_invariants_catch_a_mode_on_two_positions() {
        const BROKEN: &[DialPosition] = &[
            DialPosition {
                rings: &[Ring {
                    button: CycleButton::Select,
                    modes: &[DC_V, ACDC_V],
                }],
            },
            DialPosition {
                rings: &[Ring {
                    button: CycleButton::Select,
                    modes: &[OHM, ACDC_V],
                }],
            },
        ];
        let err = check_table(BROKEN, &[], &[CycleButton::Select], &label)
            .expect_err("duplicate mode accepted");
        assert!(err.contains("more than one position"), "{err}");

        // Declaring it shared is how a family says it is deliberate.
        assert!(check_table(BROKEN, &[ACDC_V], &[CycleButton::Select], &label).is_ok());
    }

    #[test]
    fn the_invariants_catch_rings_with_no_junction() {
        const BROKEN: &[DialPosition] = &[DialPosition {
            rings: &[
                Ring {
                    button: CycleButton::Select,
                    modes: &[DC_V, ACDC_V],
                },
                Ring {
                    button: CycleButton::Hz,
                    modes: &[HZ, DUTY],
                },
            ],
        }];
        let err = check_table(BROKEN, &[], &[CycleButton::Select, CycleButton::Hz], &label)
            .expect_err("junctionless position accepted");
        assert!(err.contains("share no mode"), "{err}");
    }

    #[test]
    fn the_invariants_catch_a_ring_that_cannot_be_walked() {
        const EMPTY_RING: &[DialPosition] = &[DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[],
            }],
        }];
        let err = check_table(EMPTY_RING, &[], &[CycleButton::Select], &label)
            .expect_err("empty ring accepted");
        assert!(err.contains("empty"), "{err}");

        const REPEATED: &[DialPosition] = &[DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[DC_V, ACDC_V, DC_V],
            }],
        }];
        let err = check_table(REPEATED, &[], &[CycleButton::Select], &label)
            .expect_err("repeated mode accepted");
        assert!(err.contains("twice"), "{err}");

        const NO_RINGS: &[DialPosition] = &[DialPosition { rings: &[] }];
        let err = check_table(NO_RINGS, &[], &[CycleButton::Select], &label)
            .expect_err("ringless position accepted");
        assert!(err.contains("want 1 or 2"), "{err}");
    }

    #[test]
    fn the_invariants_catch_a_button_the_meter_lacks_and_a_nameless_mode() {
        let err = check_table(DIAL, &[HZ, DUTY], &[CycleButton::Select], &label)
            .expect_err("Hz/% accepted on a meter without one");
        assert!(err.contains("does not have"), "{err}");

        const UNNAMED: &[DialPosition] = &[DialPosition {
            rings: &[Ring {
                button: CycleButton::Select,
                modes: &[0xAA],
            }],
        }];
        let err = check_table(UNNAMED, &[], &[CycleButton::Select], &label)
            .expect_err("unnamed mode accepted");
        assert!(err.contains("no name"), "{err}");
    }
}
