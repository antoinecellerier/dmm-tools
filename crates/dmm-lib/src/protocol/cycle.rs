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
//! # Rings that are not modes
//!
//! The manual range ladder works the same way: RANGE is a button, the rungs
//! are a ring, and the reading names the rung the meter landed on. So the
//! same [`walk`] drives it, told through an [`Observable`] which part of the
//! reading to watch and how to name its values. A ladder is one ring on its
//! own — no dial table, no junction — and Auto is not part of it: every
//! family reaches auto-ranging with a command of its own
//! ([`CycleMeter::set_auto_range`]).
//!
//! A family implements [`CycleMeter`], keeps a [`DialState`] updated with
//! [`DialState::observe`] from every measurement it parses, and forwards
//! `Protocol::choices` / `Protocol::select` to the free functions
//! here.

use log::debug;
use std::borrow::Cow;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::flags::{Flag, StatusFlags};
use crate::measurement::Measurement;
use crate::protocol::{AUTO_RANGE_ID, AUTO_RANGE_LABEL, Choice, Setting};
use crate::transport::Transport;

/// A front-panel button that cycles the meter through a ring of modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleButton {
    /// The SELECT / FUNC button: steps the dial position's main functions.
    Select,
    /// The Hz/% button: steps a position's frequency and duty-cycle modes.
    Hz,
    /// The RANGE button: steps the current mode's manual range ladder.
    Range,
    /// The HOLD button: freezes and unfreezes the display.
    Hold,
    /// The REL button: enters and leaves relative reading.
    Rel,
    /// The MIN/MAX button: steps the min/max tracking states.
    MinMax,
    /// The PEAK button: steps the peak-hold states.
    Peak,
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

    /// Take one reading. The walk watches whichever part of it the
    /// [`Observable`] cares about.
    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement>;

    /// Display name in the same vocabulary as `Measurement::mode`.
    fn mode_label(&self, mode: u16) -> Cow<'static, str>;

    fn settle(&self) -> Settle;

    /// What the meter's front panel calls the button, for error text.
    fn button_name(&self, button: CycleButton) -> &'static str {
        match button {
            CycleButton::Select => "SELECT",
            CycleButton::Hz => "Hz/%",
            CycleButton::Range => RANGE_BUTTON_NAME,
            CycleButton::Hold => "HOLD",
            CycleButton::Rel => "REL",
            CycleButton::MinMax => "MIN/MAX",
            CycleButton::Peak => "PEAK",
        }
    }

    /// The manual range ladder `mode` offers: one label per rung, in ladder
    /// order, so choice id `n` is entry `n - 1`.
    ///
    /// Empty — the default — means this family offers no remote range
    /// selection, or this mode has no ladder worth offering
    /// ([`usable_ladder`]).
    fn range_ladder(&self, _mode: u16) -> Vec<Cow<'static, str>> {
        Vec::new()
    }

    /// The 1-based rung a reading's `range_raw` names.
    ///
    /// The default is the plain 0-based index the UT61+ family reports; the
    /// Voltcraft meters send it offset by 0x30 and override this.
    fn range_rung(&self, range_raw: u8) -> u16 {
        u16::from(range_raw) + 1
    }

    /// Put the meter back in auto-range: its own command on every family
    /// here, as it is its own button on every front panel. Never a press —
    /// walking the ladder cannot reach auto.
    fn set_auto_range(&mut self, _transport: &dyn Transport) -> Result<()> {
        Err(crate::protocol::unsupported_setting(
            crate::protocol::Setting::Range,
        ))
    }

    /// The states `setting` can be put in while the meter is in `mode`,
    /// [`OFF_STATE`] first.
    ///
    /// Empty — the default — means this family does not drive that setting
    /// remotely, or this mode does not offer it (Peak only does something in
    /// an AC context on the UT61+ family).
    fn flag_states(&self, _setting: FlagSetting, _mode: u16) -> &'static [u16] {
        &[]
    }

    /// Leave `setting`: its own command, as it is its own button on the
    /// front panel. Never a press — the MIN/MAX and Peak rings cannot be
    /// pressed back to off, only stepped between their active states.
    ///
    /// Only reached for the settings [`FlagSetting::toggles`] says are not
    /// plain toggles; HOLD and REL press their own button back off.
    fn exit_flag(&mut self, _transport: &dyn Transport, setting: FlagSetting) -> Result<()> {
        Err(crate::protocol::unsupported_setting(setting.setting()))
    }
}

/// What the front panel calls [`CycleButton::Range`] on every meter here.
pub(crate) const RANGE_BUTTON_NAME: &str = "RANGE";

/// One dimension of the meter a [`walk`] steps through.
///
/// The walk is the same whatever is being stepped — press, read back, stop
/// when the reading says the target arrived. What differs is which part of
/// the reading to watch and how to name its values.
pub(crate) trait Observable<M: CycleMeter + ?Sized> {
    /// The value `reading` reports for this dimension, or the error that
    /// ends the walk (the range ladder refuses to keep pressing once the
    /// mode has changed under it).
    fn value(&self, meter: &M, reading: &Measurement) -> Result<u16>;

    /// Display name for one value, for the messages the walk produces.
    fn label(&self, meter: &M, value: u16) -> Cow<'static, str>;
}

/// The mode byte: what [`select_mode`] walks.
struct ModeWalk;

impl<M: CycleMeter + ?Sized> Observable<M> for ModeWalk {
    fn value(&self, _meter: &M, reading: &Measurement) -> Result<u16> {
        Ok(reading.mode_raw)
    }

    fn label(&self, meter: &M, value: u16) -> Cow<'static, str> {
        meter.mode_label(value)
    }
}

/// The manual range ladder: what [`select_range`] walks.
///
/// `start_mode` is the mode the walk began in. A UT61E+ capture (2026-07-29,
/// docs/verification-backlog.md) saw repeated RANGE presses flip the mode
/// byte DC V <-> AC+DC V, which is SELECT's documented effect, not RANGE's.
/// Whatever causes it, pressing on once the function has changed would be
/// stepping a ladder the user never asked for, so the walk stops instead.
struct RangeWalk {
    start_mode: u16,
    ladder: Vec<Cow<'static, str>>,
}

impl<M: CycleMeter + ?Sized> Observable<M> for RangeWalk {
    fn value(&self, meter: &M, reading: &Measurement) -> Result<u16> {
        if reading.mode_raw != self.start_mode {
            return Err(Error::CommandRejected(format!(
                "the mode changed to {}; stopped pressing {RANGE_BUTTON_NAME}",
                meter.mode_label(reading.mode_raw),
            )));
        }
        Ok(if reading.flags.auto_range {
            AUTO_RANGE_ID
        } else {
            meter.range_rung(reading.range_raw)
        })
    }

    fn label(&self, _meter: &M, value: u16) -> Cow<'static, str> {
        match usize::from(value).checked_sub(1) {
            None => Cow::Borrowed(AUTO_RANGE_LABEL),
            // A rung the ladder doesn't list is a meter that disagrees with
            // the family's table; name it by number rather than hide it.
            Some(i) => self
                .ladder
                .get(i)
                .cloned()
                .unwrap_or_else(|| Cow::Owned(format!("range {value}"))),
        }
    }
}

/// One of the meter settings the reading answers with a status flag.
///
/// [`Setting`] names six; these are the four the meter reports as a badge
/// rather than as the mode or range field, and all four are driven the same
/// way — a button that steps a ring of states, plus, where "off" is not one
/// of that ring's steps, a command that leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlagSetting {
    Hold,
    Rel,
    MinMax,
    Peak,
}

/// The state each of these settings is in when its badge is dark.
pub(crate) const OFF_STATE: u16 = 0;
/// Label of [`OFF_STATE`].
pub(crate) const OFF_LABEL: &str = "off";
/// Label of the on state of a plain toggle (HOLD, REL).
pub(crate) const ON_LABEL: &str = "on";

impl FlagSetting {
    /// The flag-backed half of [`Setting`], or `None` for the two settings
    /// the reading reports as fields of its own.
    pub(crate) fn of(setting: Setting) -> Option<Self> {
        Some(match setting {
            Setting::Hold => FlagSetting::Hold,
            Setting::Rel => FlagSetting::Rel,
            Setting::MinMax => FlagSetting::MinMax,
            Setting::Peak => FlagSetting::Peak,
            Setting::Mode | Setting::Range => return None,
        })
    }

    /// The [`Setting`] this is, for the ids, the CLI word and the refusals.
    pub(crate) fn setting(self) -> Setting {
        match self {
            FlagSetting::Hold => Setting::Hold,
            FlagSetting::Rel => Setting::Rel,
            FlagSetting::MinMax => Setting::MinMax,
            FlagSetting::Peak => Setting::Peak,
        }
    }

    /// The button whose presses step this setting's states.
    pub(crate) fn button(self) -> CycleButton {
        match self {
            FlagSetting::Hold => CycleButton::Hold,
            FlagSetting::Rel => CycleButton::Rel,
            FlagSetting::MinMax => CycleButton::MinMax,
            FlagSetting::Peak => CycleButton::Peak,
        }
    }

    /// Whether the button's ring includes off, so the setting is a plain
    /// toggle and no exit command is involved.
    ///
    /// HOLD and REL are. MIN/MAX and Peak are not: on a UT61E+ the first
    /// press of 0x41 enters MAX and further presses only swap MAX and MIN —
    /// off is reached by 0x42 alone (docs/verification-backlog.md, "MIN/MAX
    /// and Peak measurement reporting"), and 0x4D/0x4E behave the same.
    pub(crate) fn toggles(self) -> bool {
        matches!(self, FlagSetting::Hold | FlagSetting::Rel)
    }

    /// The badge the meter lights in state `value`, or `None` for off and
    /// for a value outside this setting's states.
    fn flag(self, value: u16) -> Option<Flag> {
        Some(match (self, value) {
            (FlagSetting::Hold, 1) => Flag::Hold,
            (FlagSetting::Rel, 1) => Flag::Rel,
            (FlagSetting::MinMax, 1) => Flag::Max,
            (FlagSetting::MinMax, 2) => Flag::Min,
            (FlagSetting::MinMax, 3) => Flag::Avg,
            (FlagSetting::Peak, 1) => Flag::PeakMax,
            (FlagSetting::Peak, 2) => Flag::PeakMin,
            _ => return None,
        })
    }

    /// The non-off states, in id order — every badge worth testing a
    /// reading for. A family that lacks one simply never sets its flag.
    fn active_states(self) -> &'static [u16] {
        match self {
            FlagSetting::Hold | FlagSetting::Rel => &[1],
            FlagSetting::MinMax => &[1, 2, 3],
            FlagSetting::Peak => &[1, 2],
        }
    }

    /// Which state the reading's flags put this setting in.
    pub(crate) fn state(self, flags: &StatusFlags) -> u16 {
        self.active_states()
            .iter()
            .copied()
            .find(|&value| self.flag(value).is_some_and(|flag| flags.get(flag)))
            .unwrap_or(OFF_STATE)
    }

    /// Display name of one state, in the vocabulary of the meter's own
    /// badges, so a choice list reads like the display it switches.
    pub(crate) fn label(self, value: u16) -> Cow<'static, str> {
        if value == OFF_STATE {
            return Cow::Borrowed(OFF_LABEL);
        }
        if self.toggles() {
            return Cow::Borrowed(ON_LABEL);
        }
        match self.flag(value).and_then(Flag::label) {
            Some(label) => Cow::Borrowed(label),
            // A state no family offers; name it by number rather than hide it.
            None => Cow::Owned(format!("{} {value}", self.setting())),
        }
    }
}

/// One flag-backed setting: what [`select_flag`] walks.
struct FlagWalk(FlagSetting);

impl<M: CycleMeter + ?Sized> Observable<M> for FlagWalk {
    fn value(&self, _meter: &M, reading: &Measurement) -> Result<u16> {
        Ok(self.0.state(&reading.flags))
    }

    fn label(&self, _meter: &M, value: u16) -> Cow<'static, str> {
        self.0.label(value)
    }
}

/// A ladder worth offering, or nothing.
///
/// A single rung is no choice at all, and a table whose rungs all carry the
/// same label is a placeholder rather than a ladder (the UT61E+ current
/// tables pair a spare index 0 with the one verified 20A entry). Neither is
/// offered: the consumers' rule is "fewer than two entries, draw nothing".
pub(crate) fn usable_ladder(labels: Vec<Cow<'static, str>>) -> Vec<Cow<'static, str>> {
    if labels.len() < 2 || labels.iter().all(|l| *l == labels[0]) {
        return Vec::new();
    }
    labels
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
        None => read_and_observe(meter, transport)?.mode_raw,
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
        if let Err(e) = walk(meter, transport, &leg, &ModeWalk, from) {
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
    /// Entries in the ring being walked — the press budget comes from it.
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
fn walk<M: CycleMeter + ?Sized, O: Observable<M> + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    leg: &Leg,
    obs: &O,
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
            obs.label(meter, seen),
            obs.label(meter, leg.target)
        );
        meter.press(transport, leg.button)?;
        let now = observe_after_press(meter, transport, obs, seen, settle)?;
        if now == leg.target {
            return Ok(());
        }
        if now == seen {
            return Err(Error::CommandRejected(format!(
                "{} did nothing in {}",
                meter.button_name(leg.button),
                obs.label(meter, seen)
            )));
        }
        if now == start {
            return Err(Error::CommandRejected(format!(
                "{} never appeared; the meter is back in {}",
                obs.label(meter, leg.target),
                obs.label(meter, start)
            )));
        }
        seen = now;
    }
    Err(Error::CommandRejected(format!(
        "gave up after {budget} presses of {}; the meter is in {}",
        meter.button_name(leg.button),
        obs.label(meter, seen)
    )))
}

/// Take one reading and let it update the inferred dial position.
///
/// Every reading the driver takes goes through here, walk or not: the dial
/// is only ever known from the modes the meter reports.
fn read_and_observe<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
) -> Result<Measurement> {
    let reading = meter.read(transport)?;
    let positions = meter.dial_positions();
    meter.dial_state_mut().observe(positions, reading.mode_raw);
    Ok(reading)
}

/// Read the meter back after a press, skipping frames that still show `seen`.
///
/// Waits `settle.delay` before each read and takes up to `settle.reads` of
/// them (always at least one), stopping at the first reading that differs.
/// Returns `seen` when none does — the caller decides what that means.
fn observe_after_press<M: CycleMeter + ?Sized, O: Observable<M> + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    obs: &O,
    seen: u16,
    settle: Settle,
) -> Result<u16> {
    let mut last = seen;
    for _ in 0..settle.reads.max(1) {
        if !settle.delay.is_zero() {
            // Real time, not the session clock: this waits for the meter's own
            // display to settle, which no clock flag makes faster.
            std::thread::sleep(settle.delay);
        }
        let reading = read_and_observe(meter, transport)?;
        last = obs.value(meter, &reading)?;
        if last != seen {
            break;
        }
        debug!(
            "cycle: meter still reports {}, re-reading",
            obs.label(meter, seen)
        );
    }
    Ok(last)
}

/// The ladder `mode` offers, once `id` is known to be one of its rungs.
///
/// Both refusals are decided from the family's own table, so a bad id never
/// reaches the meter.
fn check_range_id<M: CycleMeter + ?Sized>(
    meter: &M,
    mode: u16,
    id: u16,
) -> Result<Vec<Cow<'static, str>>> {
    let ladder = meter.range_ladder(mode);
    if ladder.is_empty() {
        return Err(Error::UnsupportedCommand(format!(
            "{} has no range to choose on this meter",
            meter.mode_label(mode)
        )));
    }
    if id != AUTO_RANGE_ID && usize::from(id) > ladder.len() {
        return Err(Error::UnsupportedCommand(format!(
            "range {id} is not one of the {} {} offers",
            ladder.len(),
            meter.mode_label(mode)
        )));
    }
    Ok(ladder)
}

/// The ranges reachable in the mode `current` reports, for
/// `Protocol::choices`.
///
/// Auto first, then the ladder, `id = index + 1`. Empty when the mode has no
/// ladder to offer. Does not touch the meter's state, like
/// [`mode_choices`].
pub(crate) fn range_choices<M: CycleMeter + ?Sized>(
    meter: &M,
    current: &Measurement,
) -> Vec<Choice> {
    let ladder = meter.range_ladder(current.mode_raw);
    if ladder.is_empty() {
        return Vec::new();
    }
    let live = (!current.flags.auto_range).then(|| meter.range_rung(current.range_raw));
    let mut choices = Vec::with_capacity(ladder.len() + 1);
    choices.push(Choice {
        id: AUTO_RANGE_ID,
        label: Cow::Borrowed(AUTO_RANGE_LABEL),
        current: current.flags.auto_range,
    });
    choices.extend(ladder.into_iter().enumerate().map(|(i, label)| {
        let id = i as u16 + 1;
        Choice {
            id,
            label,
            current: live == Some(id),
        }
    }));
    choices
}

/// Press the meter onto rung `id` of the current mode's ladder, or back to
/// auto-range for [`AUTO_RANGE_ID`], for `Protocol::select`.
///
/// Auto is never walked to: it is its own command, confirmed with the same
/// settle-and-re-read discipline a press gets. Everything else is one ring
/// walked with RANGE — and the first press engages manual ranging at
/// whatever rung auto had chosen (verified on a UT61E+), so a walk that
/// starts in auto simply sees the observable go from auto to a rung and
/// carries on.
pub(crate) fn select_range<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    id: u16,
) -> Result<()> {
    // An id the last known mode's ladder doesn't have costs no I/O at all.
    if let Some(mode) = meter.dial_state().last_mode() {
        check_range_id(meter, mode, id)?;
    }
    // Which rung the meter sits on is only in the stream, and the mode may
    // have moved since the last reading, so both come from a fresh one.
    let reading = read_and_observe(meter, transport)?;
    let ladder = check_range_id(meter, reading.mode_raw, id)?;
    let walk_obs = RangeWalk {
        start_mode: reading.mode_raw,
        ladder,
    };
    let seen = walk_obs.value(meter, &reading)?;
    if seen == id {
        return Ok(());
    }
    let settle = meter.settle();
    if id == AUTO_RANGE_ID {
        debug!(
            "cycle: setting auto-range (in {})",
            walk_obs.label(meter, seen)
        );
        meter.set_auto_range(transport)?;
        let now = observe_after_press(meter, transport, &walk_obs, seen, settle)?;
        return if now == AUTO_RANGE_ID {
            Ok(())
        } else {
            Err(Error::CommandRejected(format!(
                "AUTO did nothing; the meter is still in {}",
                walk_obs.label(meter, now)
            )))
        };
    }
    let leg = Leg {
        button: CycleButton::Range,
        ring_len: walk_obs.ladder.len(),
        target: id,
    };
    walk(meter, transport, &leg, &walk_obs, seen)
}

/// The states `setting` offers in `mode`, once `id` is known to be one.
///
/// Both refusals come from the family's own list, so a bad id never reaches
/// the meter.
fn check_flag_id<M: CycleMeter + ?Sized>(
    meter: &M,
    setting: FlagSetting,
    mode: u16,
    id: u16,
) -> Result<&'static [u16]> {
    let states = meter.flag_states(setting, mode);
    if states.is_empty() {
        return Err(Error::UnsupportedCommand(format!(
            "{} cannot be set in {} on this meter",
            setting.setting(),
            meter.mode_label(mode)
        )));
    }
    if !states.contains(&id) {
        let offered: Vec<_> = states.iter().map(|&s| setting.label(s)).collect();
        return Err(Error::UnsupportedCommand(format!(
            "{} has no state {id}; {} offers {}",
            setting.setting(),
            meter.mode_label(mode),
            offered.join(", ")
        )));
    }
    Ok(states)
}

/// The states `setting` can be switched to in the mode `current` reports,
/// for `Protocol::choices`.
///
/// Empty when the family or the mode does not offer the setting. Does not
/// touch the meter's state, like [`mode_choices`].
pub(crate) fn flag_choices<M: CycleMeter + ?Sized>(
    meter: &M,
    setting: FlagSetting,
    current: &Measurement,
) -> Vec<Choice> {
    let live = setting.state(&current.flags);
    meter
        .flag_states(setting, current.mode_raw)
        .iter()
        .map(|&id| Choice {
            id,
            label: setting.label(id),
            current: id == live,
        })
        .collect()
}

/// Put `setting` in state `id`, for `Protocol::select`.
///
/// Off is walked to only for a plain toggle; MIN/MAX and Peak leave by their
/// own command, confirmed with the same settle-and-re-read discipline a
/// press gets. Every other state is one ring walked with the setting's
/// button — and entering from off is simply that walk's first press, which
/// the observable sees take the setting from off to its first active state.
pub(crate) fn select_flag<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    setting: FlagSetting,
    id: u16,
) -> Result<()> {
    // A state the last known mode does not offer costs no I/O at all.
    if let Some(mode) = meter.dial_state().last_mode() {
        check_flag_id(meter, setting, mode, id)?;
    }
    // Which state the meter is in is only in the stream, and the mode may
    // have moved since the last reading, so both come from a fresh one.
    let reading = read_and_observe(meter, transport)?;
    let states = check_flag_id(meter, setting, reading.mode_raw, id)?;
    let walk_obs = FlagWalk(setting);
    let seen = walk_obs.value(meter, &reading)?;
    if seen == id {
        return Ok(());
    }
    let settle = meter.settle();
    let button = meter.button_name(setting.button());
    if id == OFF_STATE && !setting.toggles() {
        debug!(
            "cycle: leaving {} (in {})",
            setting.setting(),
            walk_obs.label(meter, seen)
        );
        meter.exit_flag(transport, setting)?;
        let now = observe_after_press(meter, transport, &walk_obs, seen, settle)?;
        return if now == OFF_STATE {
            Ok(())
        } else {
            Err(Error::CommandRejected(format!(
                "EXIT {button} did nothing; the meter is still in {}",
                walk_obs.label(meter, now)
            )))
        };
    }
    let leg = Leg {
        button: setting.button(),
        // Off is not a step of a non-toggle's ring: the button walks the
        // active states only, and the first press is what enters them.
        ring_len: states
            .iter()
            .filter(|&&s| setting.toggles() || s != OFF_STATE)
            .count(),
        target: id,
    };
    walk(meter, transport, &leg, &walk_obs, seen)
}

/// The whole of `Protocol::choices` for a family that reaches every setting
/// by pressing a button — the routing from [`Setting`] to the list above is
/// the same for all of them, so each family forwards here in one line.
pub(crate) fn choices<M: CycleMeter + ?Sized>(
    meter: &M,
    setting: Setting,
    current: &Measurement,
) -> Vec<Choice> {
    match setting {
        Setting::Mode => mode_choices(meter, current),
        Setting::Range => range_choices(meter, current),
        flag => match FlagSetting::of(flag) {
            Some(flag) => flag_choices(meter, flag, current),
            None => Vec::new(),
        },
    }
}

/// The whole of `Protocol::select` for such a family, routed like
/// [`choices`].
pub(crate) fn select<M: CycleMeter + ?Sized>(
    meter: &mut M,
    transport: &dyn Transport,
    setting: Setting,
    id: u16,
) -> Result<()> {
    match setting {
        Setting::Mode => select_mode(meter, transport, id),
        Setting::Range => select_range(meter, transport, id),
        flag => match FlagSetting::of(flag) {
            Some(flag) => select_flag(meter, transport, flag, id),
            None => Err(crate::protocol::unsupported_setting(setting)),
        },
    }
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
    use crate::flags::StatusFlags;
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

    /// A reading in `mode` on ladder rung `rung`, 0 meaning auto-range.
    fn reading_at(mode: u16, rung: u16) -> Measurement {
        reading_with(mode, rung, StatusFlags::default())
    }

    /// The same, with the meter's badges on it.
    fn reading_with(mode: u16, rung: u16, flags: StatusFlags) -> Measurement {
        Measurement {
            range_raw: rung.saturating_sub(1) as u8,
            flags: StatusFlags {
                auto_range: rung == AUTO_RANGE_ID,
                ..flags
            },
            ..reading(mode)
        }
    }

    /// A UT61E+-shaped DC V ladder.
    fn dc_v_ladder() -> Vec<Cow<'static, str>> {
        ["2.2V", "22V", "220V", "1000V"]
            .into_iter()
            .map(Cow::Borrowed)
            .collect()
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
        /// Readings after a press that still report the pre-press state.
        stale: usize,
        stale_left: usize,
        stale_state: (u16, u16),
        settle_reads: usize,
        /// The ladder the *driver* is told about — the family's table.
        ladder: Vec<Cow<'static, str>>,
        /// The rung the meter is on, 0 being auto-range.
        rung: u16,
        /// The ladder the meter really has: a press steps `r % real_rungs +
        /// 1`, and 0 means it never comes back around, so only the press
        /// budget can stop a walk.
        real_rungs: u16,
        /// The rung the first press out of auto lands on, as the meter's own
        /// auto-ranging had chosen it.
        auto_rung: u16,
        /// A meter that changes function under RANGE, as a UT61E+ was seen
        /// to do (docs/verification-backlog.md, 2026-07-29).
        range_flips_mode: Option<u16>,
        /// Auto-range commands sent, and whether the meter obeys them.
        autos: usize,
        auto_works: bool,
        /// The badges the meter is showing, and the ones it showed in the
        /// frame already in flight when a press landed.
        flags: StatusFlags,
        stale_flags: StatusFlags,
        /// Buttons the meter ignores, as a real one ignores a button its
        /// current function has no use for.
        dead: Vec<CycleButton>,
        /// Whether this meter has a Peak function at all.
        offers_peak: bool,
        /// Exit commands sent, and whether the meter obeys them.
        exits: usize,
        exit_works: bool,
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
                stale_state: (mode, 0),
                settle_reads: 1,
                ladder: Vec::new(),
                rung: AUTO_RANGE_ID,
                real_rungs: 0,
                auto_rung: 1,
                range_flips_mode: None,
                autos: 0,
                auto_works: true,
                flags: StatusFlags::default(),
                stale_flags: StatusFlags::default(),
                dead: Vec::new(),
                offers_peak: false,
                exits: 0,
                exit_works: true,
            }
        }

        /// The one-ring SELECT meter with its badges already in `flags`.
        fn showing(flags: StatusFlags) -> Self {
            let mut meter = Self::v_dc();
            meter.flags = flags;
            meter
        }

        /// A meter on the DC V ladder, sitting on `rung` (0 = auto), whose
        /// RANGE button really does step that ladder.
        fn on_ladder(rung: u16) -> Self {
            let mut meter = Self::v_dc();
            meter.ladder = dc_v_ladder();
            meter.real_rungs = 4;
            meter.rung = rung;
            meter
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

        /// This meter's badges with `setting` moved to `state`.
        fn flags_for(&self, setting: FlagSetting, state: u16) -> StatusFlags {
            let mut flags = self.flags;
            match setting {
                FlagSetting::Hold => flags.hold = state == 1,
                FlagSetting::Rel => flags.rel = state == 1,
                FlagSetting::MinMax => {
                    flags.max = state == 1;
                    flags.min = state == 2;
                }
                FlagSetting::Peak => {
                    flags.peak_max = state == 1;
                    flags.peak_min = state == 2;
                }
            }
            flags
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
                self.stale_state = (self.mode, self.rung);
                self.mode = next;
                self.stale_left = self.stale;
            }
            if let Some(setting) = flag_setting_of(button) {
                self.stale_state = (self.mode, self.rung);
                self.stale_flags = self.flags;
                if !self.dead.contains(&button) {
                    self.flags = self.flags_for(setting, next_state(setting, &self.flags));
                    self.stale_left = self.stale;
                }
            }
            if button == CycleButton::Range && !self.ladder.is_empty() {
                self.stale_state = (self.mode, self.rung);
                self.rung = match (self.rung, self.real_rungs) {
                    (AUTO_RANGE_ID, _) => self.auto_rung,
                    (r, 0) => r + 1,
                    (r, n) => r % n + 1,
                };
                if let Some(mode) = self.range_flips_mode {
                    self.mode = mode;
                }
                self.stale_left = self.stale;
            }
            Ok(())
        }

        fn read(&mut self, _transport: &dyn Transport) -> Result<Measurement> {
            self.reads += 1;
            if self.stale_left > 0 {
                self.stale_left -= 1;
                let (mode, rung) = self.stale_state;
                return Ok(reading_with(mode, rung, self.stale_flags));
            }
            Ok(reading_with(self.mode, self.rung, self.flags))
        }

        fn flag_states(&self, setting: FlagSetting, _mode: u16) -> &'static [u16] {
            match setting {
                FlagSetting::Hold | FlagSetting::Rel => &[0, 1],
                FlagSetting::MinMax => &[0, 1, 2],
                FlagSetting::Peak if self.offers_peak => &[0, 1, 2],
                FlagSetting::Peak => &[],
            }
        }

        fn exit_flag(&mut self, _transport: &dyn Transport, setting: FlagSetting) -> Result<()> {
            self.exits += 1;
            if self.exit_works {
                self.stale_state = (self.mode, self.rung);
                self.stale_flags = self.flags;
                self.flags = self.flags_for(setting, OFF_STATE);
                self.stale_left = self.stale;
            }
            Ok(())
        }

        fn range_ladder(&self, _mode: u16) -> Vec<Cow<'static, str>> {
            self.ladder.clone()
        }

        fn set_auto_range(&mut self, _transport: &dyn Transport) -> Result<()> {
            self.autos += 1;
            if self.auto_works {
                self.stale_state = (self.mode, self.rung);
                self.rung = AUTO_RANGE_ID;
                self.stale_left = self.stale;
            }
            Ok(())
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

    /// The setting `button` steps, for the fake meter's press handler.
    fn flag_setting_of(button: CycleButton) -> Option<FlagSetting> {
        match button {
            CycleButton::Hold => Some(FlagSetting::Hold),
            CycleButton::Rel => Some(FlagSetting::Rel),
            CycleButton::MinMax => Some(FlagSetting::MinMax),
            CycleButton::Peak => Some(FlagSetting::Peak),
            CycleButton::Select | CycleButton::Hz | CycleButton::Range => None,
        }
    }

    /// Where one press of the setting's button lands: a toggle flips, and a
    /// ring goes off -> MAX, then round the active states only, never back
    /// to off.
    fn next_state(setting: FlagSetting, flags: &StatusFlags) -> u16 {
        let now = setting.state(flags);
        if setting.toggles() {
            return 1 - now;
        }
        if now == 1 { 2 } else { 1 }
    }

    fn labels(choices: &[Choice]) -> Vec<String> {
        choices.iter().map(|c| c.label.to_string()).collect()
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

    // ---- the range ladder ----

    #[test]
    fn range_choices_list_auto_then_the_ladder() {
        let meter = FakeMeter::on_ladder(2);
        let choices = range_choices(&meter, &reading_at(DC_V, 2));
        assert_eq!(ids(&choices), vec![0, 1, 2, 3, 4]);
        assert_eq!(choices[0].label, "Auto");
        assert_eq!(choices[2].label, "22V");
        assert_eq!(current_ids(&choices), vec![2]);
    }

    #[test]
    fn an_auto_ranging_meter_marks_auto_current() {
        let meter = FakeMeter::on_ladder(AUTO_RANGE_ID);
        let choices = range_choices(&meter, &reading_at(DC_V, AUTO_RANGE_ID));
        assert_eq!(current_ids(&choices), vec![AUTO_RANGE_ID]);
    }

    #[test]
    fn a_mode_with_no_ladder_offers_no_ranges() {
        let meter = FakeMeter::v_dc();
        assert!(range_choices(&meter, &reading_at(DC_V, 1)).is_empty());
    }

    #[test]
    fn select_range_presses_once_per_rung() {
        let mut meter = FakeMeter::on_ladder(1);
        select_range(&mut meter, &NullTransport, 3).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Range; 2]);
        assert_eq!(meter.rung, 3);
    }

    /// The first press out of auto engages manual ranging at whatever rung
    /// auto had chosen, so the walk simply carries on from there.
    #[test]
    fn a_walk_out_of_auto_starts_where_auto_was() {
        let mut meter = FakeMeter::on_ladder(AUTO_RANGE_ID);
        meter.auto_rung = 2;
        select_range(&mut meter, &NullTransport, 4).expect("switched");
        assert_eq!(meter.presses.len(), 3);
        assert_eq!(meter.rung, 4);
    }

    #[test]
    fn selecting_the_rung_the_meter_is_on_presses_nothing() {
        let mut meter = FakeMeter::on_ladder(2);
        select_range(&mut meter, &NullTransport, 2).expect("already there");
        assert!(meter.presses.is_empty());
        assert_eq!(meter.autos, 0);
    }

    #[test]
    fn a_rung_the_ladder_does_not_have_costs_no_io() {
        for id in [5, 99] {
            let mut meter = FakeMeter::on_ladder(1);
            let err = select_range(&mut meter, &NullTransport, id).unwrap_err();
            assert!(
                matches!(&err, Error::UnsupportedCommand(m)
                    if m.contains("is not one of the 4")),
                "got {err:?}"
            );
            assert_eq!(meter.reads, 0, "{id} read the meter");
            assert!(meter.presses.is_empty());
        }
    }

    #[test]
    fn a_mode_without_a_ladder_refuses_before_any_io() {
        let mut meter = FakeMeter::v_dc();
        let err = select_range(&mut meter, &NullTransport, 1).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("no range to choose")),
            "got {err:?}"
        );
        assert_eq!(meter.reads, 0);
    }

    /// Auto is a command of its own, never a press.
    #[test]
    fn auto_is_one_command_and_no_press() {
        let mut meter = FakeMeter::on_ladder(3);
        select_range(&mut meter, &NullTransport, AUTO_RANGE_ID).expect("back to auto");
        assert_eq!(meter.autos, 1);
        assert!(meter.presses.is_empty());
        assert_eq!(meter.rung, AUTO_RANGE_ID);
    }

    #[test]
    fn a_meter_already_in_auto_is_left_alone() {
        let mut meter = FakeMeter::on_ladder(AUTO_RANGE_ID);
        select_range(&mut meter, &NullTransport, AUTO_RANGE_ID).expect("already auto");
        assert_eq!(meter.autos, 0);
        assert!(meter.presses.is_empty());
    }

    #[test]
    fn an_auto_command_the_meter_ignores_is_reported() {
        let mut meter = FakeMeter::on_ladder(2);
        meter.auto_works = false;
        let err = select_range(&mut meter, &NullTransport, AUTO_RANGE_ID).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m == "AUTO did nothing; the meter is still in 22V"),
            "got {err:?}"
        );
    }

    /// The meter was mid-frame when the press landed: re-read, never press
    /// again, or the ladder steps past the target.
    #[test]
    fn a_stale_frame_after_a_range_press_costs_a_read() {
        let mut meter = FakeMeter::on_ladder(1);
        meter.stale = 2;
        meter.settle_reads = 3;
        select_range(&mut meter, &NullTransport, 2).expect("switched");
        assert_eq!(meter.presses, vec![CycleButton::Range]);
        // One read to find the starting rung, three to see the press land.
        assert_eq!(meter.reads, 4);
    }

    /// Repeated RANGE presses were seen to flip a UT61E+ between DC V and
    /// AC+DC V. Stepping a ladder that belongs to another function is not
    /// what was asked for, so the walk stops on the spot.
    #[test]
    fn a_mode_change_mid_walk_stops_the_walk() {
        let mut meter = FakeMeter::on_ladder(1);
        meter.range_flips_mode = Some(ACDC_V);
        let err = select_range(&mut meter, &NullTransport, 4).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m == "the mode changed to AC+DC V; stopped pressing RANGE"),
            "got {err:?}"
        );
        assert_eq!(meter.presses.len(), 1, "no further press after the change");
        assert_eq!(
            meter.state.last_mode(),
            Some(ACDC_V),
            "the reading still moved the dial state"
        );
    }

    /// The family's table claims a rung this meter doesn't have: the ladder
    /// coming back around to where it started is what says so.
    #[test]
    fn a_ladder_coming_back_around_reports_the_missing_rung() {
        let mut meter = FakeMeter::on_ladder(1);
        meter.real_rungs = 2;
        let err = select_range(&mut meter, &NullTransport, 4).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m == "1000V never appeared; the meter is back in 2.2V"),
            "got {err:?}"
        );
        assert_eq!(meter.presses.len(), 2);
    }

    /// A meter whose RANGE never comes back around: only the budget — one
    /// press per rung plus one — ends the walk.
    #[test]
    fn the_range_walk_gives_up_after_one_press_per_rung_plus_one() {
        let mut meter = FakeMeter::on_ladder(2);
        meter.real_rungs = 0;
        let err = select_range(&mut meter, &NullTransport, 1).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m.contains("gave up after 5 presses of RANGE")
                    && m.contains("the meter is in range 7")),
            "got {err:?}"
        );
        assert_eq!(meter.presses.len(), 5);
    }

    #[test]
    fn a_ladder_that_offers_no_choice_is_dropped() {
        assert!(usable_ladder(vec![Cow::Borrowed("20A")]).is_empty());
        assert!(usable_ladder(vec![Cow::Borrowed("20A"), Cow::Borrowed("20A")]).is_empty());
        assert_eq!(usable_ladder(dc_v_ladder()).len(), 4);
    }

    // ---- table invariants ----

    // --- Flag-backed settings ---------------------------------------------

    #[test]
    fn flag_choices_name_the_badges_and_mark_the_live_one() {
        let meter = FakeMeter::showing(StatusFlags {
            min: true,
            ..Default::default()
        });
        let current = reading_with(DC_V, AUTO_RANGE_ID, meter.flags);

        let hold = flag_choices(&meter, FlagSetting::Hold, &current);
        assert_eq!(ids(&hold), vec![0, 1]);
        assert_eq!(labels(&hold), vec!["off", "on"]);
        assert_eq!(current_ids(&hold), vec![0]);

        let minmax = flag_choices(&meter, FlagSetting::MinMax, &current);
        assert_eq!(ids(&minmax), vec![0, 1, 2]);
        assert_eq!(labels(&minmax), vec!["off", "MAX", "MIN"]);
        assert_eq!(current_ids(&minmax), vec![2], "MIN is lit");
    }

    #[test]
    fn a_setting_the_meter_lacks_offers_nothing() {
        let meter = FakeMeter::v_dc();
        let current = reading(DC_V);
        assert!(flag_choices(&meter, FlagSetting::Peak, &current).is_empty());
    }

    #[test]
    fn a_toggle_is_reached_in_one_press() {
        let mut meter = FakeMeter::v_dc();
        select_flag(&mut meter, &NullTransport, FlagSetting::Hold, 1).expect("held");
        assert_eq!(meter.presses, vec![CycleButton::Hold]);
        assert!(meter.flags.hold);
    }

    #[test]
    fn a_toggle_presses_its_own_button_back_off() {
        let mut meter = FakeMeter::showing(StatusFlags {
            rel: true,
            ..Default::default()
        });
        select_flag(&mut meter, &NullTransport, FlagSetting::Rel, 0).expect("released");
        assert_eq!(meter.presses, vec![CycleButton::Rel]);
        assert_eq!(meter.exits, 0, "REL has no exit command");
        assert!(!meter.flags.rel);
    }

    #[test]
    fn a_setting_already_in_the_state_is_left_alone() {
        let mut meter = FakeMeter::showing(StatusFlags {
            hold: true,
            ..Default::default()
        });
        select_flag(&mut meter, &NullTransport, FlagSetting::Hold, 1).expect("already held");
        assert!(meter.presses.is_empty());
        assert_eq!(meter.reads, 1, "only the reading that says where it is");
    }

    #[test]
    fn a_button_the_meter_ignores_is_reported() {
        let mut meter = FakeMeter::v_dc();
        meter.dead = vec![CycleButton::Hold];
        let err = select_flag(&mut meter, &NullTransport, FlagSetting::Hold, 1).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m) if m == "HOLD did nothing in off"),
            "{err}"
        );
    }

    #[test]
    fn the_minmax_ring_is_entered_and_walked_by_its_button() {
        let mut meter = FakeMeter::v_dc();
        select_flag(&mut meter, &NullTransport, FlagSetting::MinMax, 2).expect("in MIN");
        assert_eq!(
            meter.presses,
            vec![CycleButton::MinMax, CycleButton::MinMax],
            "off -> MAX -> MIN"
        );
        assert!(meter.flags.min && !meter.flags.max);
        assert_eq!(meter.exits, 0);
    }

    #[test]
    fn swapping_max_for_min_is_one_press() {
        let mut meter = FakeMeter::showing(StatusFlags {
            max: true,
            ..Default::default()
        });
        select_flag(&mut meter, &NullTransport, FlagSetting::MinMax, 2).expect("in MIN");
        assert_eq!(meter.presses, vec![CycleButton::MinMax]);
    }

    #[test]
    fn leaving_minmax_is_the_exit_command_and_no_press() {
        let mut meter = FakeMeter::showing(StatusFlags {
            max: true,
            ..Default::default()
        });
        select_flag(&mut meter, &NullTransport, FlagSetting::MinMax, 0).expect("left MIN/MAX");
        assert_eq!(meter.exits, 1);
        assert!(meter.presses.is_empty(), "the button cannot reach off");
        assert_eq!(FlagSetting::MinMax.state(&meter.flags), OFF_STATE);
    }

    #[test]
    fn an_exit_the_meter_ignores_is_reported() {
        let mut meter = FakeMeter::showing(StatusFlags {
            max: true,
            ..Default::default()
        });
        meter.exit_works = false;
        let err = select_flag(&mut meter, &NullTransport, FlagSetting::MinMax, 0).unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(m)
                if m == "EXIT MIN/MAX did nothing; the meter is still in MAX"),
            "{err}"
        );
    }

    #[test]
    fn a_state_outside_the_ring_costs_no_io() {
        let mut meter = FakeMeter::v_dc();
        let err = select_flag(&mut meter, &NullTransport, FlagSetting::MinMax, 3).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m)
                if m == "minmax has no state 3; DC V offers off, MAX, MIN"),
            "{err}"
        );
        assert_eq!(meter.reads, 0);
        assert!(meter.presses.is_empty());
    }

    #[test]
    fn a_setting_the_meter_lacks_is_refused_before_any_io() {
        let mut meter = FakeMeter::v_dc();
        let err = select_flag(&mut meter, &NullTransport, FlagSetting::Peak, 1).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m)
                if m == "peak cannot be set in DC V on this meter"),
            "{err}"
        );
        assert_eq!(meter.reads, 0);
    }

    #[test]
    fn a_stale_frame_after_a_flag_press_costs_a_read() {
        let mut meter = FakeMeter::v_dc();
        meter.stale = 1;
        meter.settle_reads = 2;
        select_flag(&mut meter, &NullTransport, FlagSetting::Hold, 1).expect("held");
        assert_eq!(meter.presses.len(), 1, "the stale frame cost a read");
        assert_eq!(meter.reads, 3, "one before, then the stale one and the new");
    }

    #[test]
    fn peak_states_are_named_after_the_peak_badges() {
        let mut meter = FakeMeter::v_dc();
        meter.offers_peak = true;
        let current = reading(DC_V);
        let peak = flag_choices(&meter, FlagSetting::Peak, &current);
        assert_eq!(labels(&peak), vec!["off", "P-MAX", "P-MIN"]);
    }

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
