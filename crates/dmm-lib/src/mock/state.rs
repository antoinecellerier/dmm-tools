//! What the mock's buttons change, and what that does to a reading.
//!
//! The scenario ([`super::scenarios`]) decides the raw value; everything
//! here is the meter's own doing — the display frozen by HOLD, the baseline
//! REL subtracts, the rung RANGE stepped to, and the extremes MIN/MAX and
//! Peak report instead of the live reading. Each press is modelled on the
//! UT61E+ button of the same name, so the mock answers a driver the way the
//! hardware does.

use crate::flags::StatusFlags;
use crate::measurement::MeasuredValue;

/// MIN/MAX display cycling state, matching real device behavior.
/// The meter cycles MAX → MIN → MAX as a 2-state toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MinMaxState {
    Off,
    Max,
    Min,
}

/// Peak display cycling state, matching real device behavior.
/// The meter cycles P-MAX → P-MIN → P-MAX as a 2-state toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeakState {
    Off,
    Max,
    Min,
}

/// The meter state one reading is filtered through.
pub(super) struct MeterState {
    hold: bool,
    held_value: Option<MeasuredValue>,
    /// Elapsed time HOLD was pressed at. Sub-values are functions of time, so
    /// freezing them means re-evaluating at this instant rather than caching
    /// each one.
    held_elapsed: Option<f64>,
    rel: bool,
    rel_base: Option<f64>,
    auto_range: bool,
    /// The ladder rung RANGE has been stepped to, as a range byte; `None`
    /// means the scenario's own range, which is what auto-ranging picked.
    manual_range: Option<u8>,
    /// Saved auto_range state before MIN/MAX activation (restored on exit).
    auto_range_before_minmax: bool,
    minmax: MinMaxState,
    stored_min: Option<f64>,
    stored_max: Option<f64>,
    peak: PeakState,
    stored_peak_min: Option<f64>,
    stored_peak_max: Option<f64>,
}

impl Default for MeterState {
    /// The state the mock powers on in: nothing held, nothing recorded, and
    /// auto-ranging, like a meter just switched on.
    fn default() -> Self {
        Self {
            hold: false,
            held_value: None,
            held_elapsed: None,
            rel: false,
            rel_base: None,
            auto_range: true,
            manual_range: None,
            auto_range_before_minmax: true,
            minmax: MinMaxState::Off,
            stored_min: None,
            stored_max: None,
            peak: PeakState::Off,
            stored_peak_min: None,
            stored_peak_max: None,
        }
    }
}

impl MeterState {
    /// What the display shows for one raw reading: HOLD, then REL, then the
    /// stored extreme MIN/MAX or Peak is reporting instead of the live value.
    ///
    /// Takes `&mut self` because the extremes are accumulated from the very
    /// readings they filter, as the meter does while it records.
    pub(super) fn apply(&mut self, raw: MeasuredValue) -> MeasuredValue {
        // Apply hold: freeze the value.
        let live_value = if self.hold {
            self.held_value.clone().unwrap_or(raw)
        } else {
            raw
        };

        // Apply rel: subtract baseline.
        let live_value = match (&live_value, self.rel_base) {
            (MeasuredValue::Normal(v), Some(base)) if self.rel => MeasuredValue::Normal(v - base),
            _ => live_value,
        };

        // Update the stored MIN/MAX and Peak values from the live reading.
        if let MeasuredValue::Normal(v) = &live_value {
            if self.minmax != MinMaxState::Off {
                self.stored_min = Some(self.stored_min.map_or(*v, |prev| prev.min(*v)));
                self.stored_max = Some(self.stored_max.map_or(*v, |prev| prev.max(*v)));
            }
            if self.peak != PeakState::Off {
                self.stored_peak_min = Some(self.stored_peak_min.map_or(*v, |prev| prev.min(*v)));
                self.stored_peak_max = Some(self.stored_peak_max.map_or(*v, |prev| prev.max(*v)));
            }
        }

        // Select display value: stored min/max/peak when active, live
        // otherwise. Real device sends the stored value, not the live
        // reading.
        let stored = match (self.minmax, self.peak) {
            (MinMaxState::Max, _) => self.stored_max,
            (MinMaxState::Min, _) => self.stored_min,
            (MinMaxState::Off, PeakState::Max) => self.stored_peak_max,
            (MinMaxState::Off, PeakState::Min) => self.stored_peak_min,
            (MinMaxState::Off, PeakState::Off) => None,
        };
        match stored {
            Some(v) => MeasuredValue::Normal(v),
            None => live_value,
        }
    }

    /// The instant sub-values are evaluated at: the one HOLD was pressed at
    /// while the display is frozen, so they freeze with the main value; the
    /// live one otherwise. REL, MIN/MAX and Peak act on the main reading
    /// only, as on the meter.
    pub(super) fn aux_elapsed(&self, live: f64) -> f64 {
        match (self.hold, self.held_elapsed) {
            (true, Some(held)) => held,
            _ => live,
        }
    }

    /// The badges this state lights, for the reading's flags.
    pub(super) fn flags(&self) -> StatusFlags {
        StatusFlags {
            hold: self.hold,
            rel: self.rel,
            auto_range: self.auto_range,
            min: self.minmax == MinMaxState::Min,
            max: self.minmax == MinMaxState::Max,
            peak_min: self.peak == PeakState::Min,
            peak_max: self.peak == PeakState::Max,
            ..Default::default()
        }
    }

    /// The range byte the meter reports: the rung RANGE was stepped to, or
    /// `auto_rung`, the one auto-ranging picked for the scenario.
    pub(super) fn reported_range(&self, auto_rung: u8) -> u8 {
        self.manual_range.unwrap_or(auto_rung)
    }

    /// Whether RANGE has been stepped off auto — the reading's range label
    /// comes from the ladder rather than the scenario when it has.
    pub(super) fn on_a_manual_rung(&self) -> bool {
        self.manual_range.is_some()
    }

    /// Whether MIN/MAX is recording, which locks the range.
    pub(super) fn minmax_recording(&self) -> bool {
        self.minmax != MinMaxState::Off
    }

    /// HOLD: freeze the display at `value`, or release it. `elapsed` is the
    /// instant `value` was evaluated at, which is where the sub-values are
    /// re-evaluated while the display is frozen.
    pub(super) fn press_hold(&mut self, value: MeasuredValue, elapsed: f64) {
        self.hold = !self.hold;
        if self.hold {
            self.held_value = Some(value);
            self.held_elapsed = Some(elapsed);
        } else {
            self.held_value = None;
            self.held_elapsed = None;
        }
    }

    /// REL: take `value` as the baseline later readings are measured
    /// against, or drop it. An overload is no number to subtract from, so it
    /// leaves REL without a baseline and readings pass through unchanged.
    pub(super) fn press_rel(&mut self, value: MeasuredValue) {
        self.rel = !self.rel;
        self.rel_base = match (self.rel, value) {
            (true, MeasuredValue::Normal(v)) => Some(v),
            _ => None,
        };
    }

    /// RANGE: engage manual ranging at `auto_rung`, the rung auto-ranging
    /// had picked, then step the ladder one rung per press (verified on a
    /// UT61E+ for the first press). A mode with no ladder ignores it, as DC
    /// mV does on the real meter, and so does MIN/MAX, which locks the
    /// range.
    pub(super) fn press_range(&mut self, ladder_len: usize, auto_rung: u8) {
        if ladder_len == 0 || self.minmax_recording() {
            return;
        }
        let next = match self.manual_range {
            None => usize::from(auto_rung).min(ladder_len - 1),
            Some(rung) => (usize::from(rung) + 1) % ladder_len,
        };
        self.manual_range = Some(next as u8);
        self.auto_range = false;
    }

    /// AUTO: back to the rung the scenario ranges itself to.
    pub(super) fn set_auto_range(&mut self) {
        self.manual_range = None;
        self.auto_range = true;
    }

    /// MIN/MAX: Off → MAX → MIN → MAX …, as the real device cycles. Mutually
    /// exclusive with Peak, and it locks the range while it records.
    pub(super) fn press_minmax(&mut self) {
        if self.peak != PeakState::Off {
            self.exit_peak();
        }
        self.minmax = match self.minmax {
            MinMaxState::Off => {
                self.auto_range_before_minmax = self.auto_range;
                self.auto_range = false;
                self.stored_min = None;
                self.stored_max = None;
                MinMaxState::Max
            }
            MinMaxState::Max => MinMaxState::Min,
            MinMaxState::Min => MinMaxState::Max,
        };
    }

    /// Leave MIN/MAX: the recording is dropped and the range goes back to
    /// what it was before it locked.
    pub(super) fn exit_minmax(&mut self) {
        self.minmax = MinMaxState::Off;
        self.stored_min = None;
        self.stored_max = None;
        self.auto_range = self.auto_range_before_minmax;
    }

    /// Peak: Off → P-MAX → P-MIN → P-MAX …, as the real device cycles.
    /// Entering it ends MIN/MAX — the meter never shows both.
    ///
    /// The caller decides whether the mode reacts to Peak at all
    /// ([`super::scenarios::Scenario::peak_applies`]).
    pub(super) fn press_peak(&mut self) {
        // Only when it is recording: `exit_minmax` puts the range back to
        // what MIN/MAX saved, which would undo a rung chosen since.
        if self.minmax_recording() {
            self.exit_minmax();
        }
        self.peak = match self.peak {
            PeakState::Off => {
                self.stored_peak_min = None;
                self.stored_peak_max = None;
                PeakState::Max
            }
            PeakState::Max => PeakState::Min,
            PeakState::Min => PeakState::Max,
        };
    }

    /// Leave Peak: the recording is dropped.
    pub(super) fn exit_peak(&mut self) {
        self.peak = PeakState::Off;
        self.stored_peak_min = None;
        self.stored_peak_max = None;
    }

    /// The scenario changed under the state: the ladder belongs to the mode
    /// that was left behind.
    pub(super) fn leave_scenario(&mut self) {
        self.manual_range = None;
    }
}
