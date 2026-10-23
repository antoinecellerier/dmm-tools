//! The interruption the plotted series is in between its last point and its
//! next, which the next point records as the break before it.

use std::time::Instant;

use super::{DataPoint, GapKind};

/// Whether the plotted series is interrupted since its last point, and why,
/// plus the over-range readings counted since the trace began. Restarting
/// the trace starts this over: an open break belongs to the trace that
/// ended, which the new one has no point before its first to break from.
#[derive(Debug, Default)]
pub(super) struct Breaks {
    /// A non-plottable sample arrived since the last plotted point, so the
    /// next one starts a new segment. Time-based gap detection can't see
    /// this: an over-range excursion shorter than the threshold leaves no
    /// time hole.
    pub(super) kind: Option<GapKind>,
    /// A genuine dropout happened during the open interruption: the link
    /// went down, or acquisition was stopped. Distinct from the meter simply
    /// not sending — see `Graph::push_data_loss`.
    pub(super) data_loss: bool,
    /// The newest overload sample's time: where the over-range band ends.
    pub(super) last_overload: Option<Instant>,
    /// The newest sample's time, an overload or a word shown instead of a
    /// reading.
    ///
    /// These carry a timestamp like any other; they just have no plottable
    /// value. Keeping the newest one lets the live view follow the present
    /// while the meter sends them, instead of freezing at the last plotted
    /// point.
    pub(super) heard_until: Option<Instant>,
    /// The first overload sample's time, in a break that began with a word
    /// shown instead of a reading: where the band starts.
    pub(super) band_from: Option<Instant>,
    /// Over-range readings since the trace began — not since the break
    /// opened — for the next point's `overloads_before`.
    pub(super) overloads: u64,
}

impl Breaks {
    /// Whether the series is over range now.
    pub(super) fn overloaded(&self) -> bool {
        self.kind == Some(GapKind::Overload)
    }

    /// Break before the next point for want of data, unless it breaks
    /// already.
    pub(super) fn gap(&mut self) {
        if self.kind.is_none() {
            self.kind = Some(GapKind::NoData);
        }
    }

    /// An overload sample arrived at `t`: see `Graph::push_break`.
    pub(super) fn overload(&mut self, t: Instant) {
        // Updated on every overload sample, not just the first: they close
        // the band and advance the live view for as long as the meter stays
        // over range.
        self.last_overload = Some(t);
        self.heard_until = Some(t);
        self.overloads += 1;
        match self.kind {
            None => self.kind = Some(GapKind::Overload),
            // In a gap for want of data only `mark_data_loss` marks a loss
            // (`no_reading` does only while over range), so without one this
            // gap is the word's.
            Some(GapKind::NoData) if !self.data_loss => {
                self.kind = Some(GapKind::Overload);
                self.band_from = Some(t);
            }
            Some(_) => {}
        }
    }

    /// A word shown instead of a reading arrived at `t`: see
    /// `Graph::push_no_reading`.
    pub(super) fn no_reading(&mut self, t: Instant) {
        self.heard_until = Some(t);
        if self.overloaded() {
            self.data_loss = true;
            return;
        }
        self.kind = Some(GapKind::NoData);
    }

    /// Data was genuinely lost: see `Graph::push_data_loss`.
    pub(super) fn mark_data_loss(&mut self) {
        self.data_loss = true;
        self.gap();
    }

    /// The plotted point at `time`, carrying the break it ends, which closes.
    pub(super) fn point(&mut self, time: Instant, value: f64) -> DataPoint {
        let band_from = self.band_from.take();
        let last_overload = self.last_overload.take();
        self.heard_until = None;
        DataPoint {
            time,
            value,
            break_before: self.kind.take(),
            break_last_sample: band_from.or(last_overload),
            break_had_data_loss: std::mem::take(&mut self.data_loss),
            break_band_late: band_from.is_some(),
            overloads_before: self.overloads,
        }
    }
}
