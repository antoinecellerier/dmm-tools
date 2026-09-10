//! The minimap's min/max level: the whole session reduced to one bucket per
//! strip pixel, grown a sample at a time instead of rebuilt.
//!
//! The strip shows every point there has ever been, so building its trace from
//! the raw history costs O(history) per push *and* per frame — the one part of
//! the graph whose cost grew with the session. A bucket keyed on session time
//! (not on screen column, see `minimap`) has fixed membership, so a new sample
//! only ever touches the last bucket and an evicted one only the first. What
//! is left is a per-frame cost that follows the strip's width.
//!
//! Pure data, no egui: the projection to screen and back lives in `minimap`,
//! and the bucketing rules are testable on their own.

use std::collections::VecDeque;

use super::GapKind;

/// One time bucket: the vertical extent of every sample that fell into it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Bucket {
    /// `floor(t / width)` — membership depends only on the sample's time, so
    /// it cannot change under a later sample.
    pub(super) key: i64,
    pub(super) y_min: f64,
    pub(super) y_max: f64,
    /// Samples folded in so far. Only eviction reads it.
    count: u32,
    /// The trace is interrupted immediately before this bucket, so it starts
    /// a new polyline even when it shares a key with the bucket before it.
    pub(super) starts_segment: bool,
}

/// One interruption of the trace, as the minimap's bands draw it.
#[derive(Debug, Clone, PartialEq)]
struct Gap {
    start: f64,
    end: f64,
    kind: GapKind,
    /// Sequence number of the point that closed the gap. Its opening point is
    /// the one before, which is what decides when eviction drops the gap.
    close_seq: u64,
}

/// The minimap's whole-session trace, bucketed at a fixed width.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MinimapLevel {
    width: f64,
    buckets: VecDeque<Bucket>,
    gaps: VecDeque<Gap>,
}

impl MinimapLevel {
    pub(super) fn new(width: f64) -> Self {
        Self {
            width,
            buckets: VecDeque::new(),
            gaps: VecDeque::new(),
        }
    }

    /// Bucket width in seconds. The level is recut when the strip's scale
    /// steps past it.
    pub(super) fn width(&self) -> f64 {
        self.width
    }

    /// Fold one sample in. `break_kind` is why the trace is interrupted
    /// immediately before it, or `None` when it continues the previous one.
    pub(super) fn append(&mut self, t: f64, value: f64, break_kind: Option<GapKind>) {
        // The strip has always projected a sample as `(t / width) as f32` and
        // floored *that* to pick its column, so round to f32 before flooring:
        // a quotient that rounds up across an integer has always been drawn in
        // the column above, and the level must put it in the same one.
        let key = ((t / self.width) as f32).floor() as i64;
        let starts_segment = break_kind.is_some();
        match self.buckets.back_mut() {
            // A break opens a new bucket even in the middle of one: the trace
            // has to stop and restart there, exactly as on the main plot.
            Some(back) if !starts_segment && back.key == key => {
                back.y_min = back.y_min.min(value);
                back.y_max = back.y_max.max(value);
                back.count += 1;
            }
            _ => self.buckets.push_back(Bucket {
                key,
                y_min: value,
                y_max: value,
                count: 1,
                starts_segment,
            }),
        }
    }

    /// Record an interruption, closed by the point with sequence `close_seq`.
    pub(super) fn push_gap(&mut self, start: f64, end: f64, kind: GapKind, close_seq: u64) {
        self.gaps.push_back(Gap {
            start,
            end,
            kind,
            close_seq,
        });
    }

    /// Drop the oldest sample, which had value `evicted`.
    ///
    /// `remaining_front_points` yields the values of the points still in the
    /// front bucket, oldest first — the caller's history, which the level does
    /// not keep a copy of. It is only walked when the evicted sample was one
    /// of the bucket's extremes, and never further than that bucket's own
    /// sample count. `first_seq` is the sequence number of the oldest point
    /// left in the history.
    pub(super) fn evict(
        &mut self,
        evicted: f64,
        remaining_front_points: impl Iterator<Item = f64>,
        first_seq: u64,
    ) {
        let emptied = match self.buckets.front_mut() {
            Some(front) => {
                front.count = front.count.saturating_sub(1);
                if front.count == 0 {
                    true
                } else {
                    // Anything but an extreme left the bucket's extent alone.
                    if evicted == front.y_min || evicted == front.y_max {
                        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                        for v in remaining_front_points.take(front.count as usize) {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                        front.y_min = lo;
                        front.y_max = hi;
                    }
                    false
                }
            }
            None => false,
        };
        if emptied {
            self.buckets.pop_front();
            // Whatever broke the trace before the new front bucket went with
            // the points it separated: nothing precedes it any more, and a
            // rebuild from what is left would not mark it either.
            if let Some(front) = self.buckets.front_mut() {
                front.starts_segment = false;
            }
        }
        // A gap is drawn between the point that opened it and the one that
        // closed it, so it outlives neither: once the opener (`close_seq - 1`)
        // is gone the band has nothing to hang from.
        while self.gaps.front().is_some_and(|g| g.close_seq <= first_seq) {
            self.gaps.pop_front();
        }
    }

    /// Lowest and highest sample in the level, unpadded.
    pub(super) fn value_range(&self) -> Option<(f64, f64)> {
        let mut buckets = self.buckets.iter();
        let first = buckets.next()?;
        let (mut lo, mut hi) = (first.y_min, first.y_max);
        for b in buckets {
            lo = lo.min(b.y_min);
            hi = hi.max(b.y_max);
        }
        Some((lo, hi))
    }

    /// The interruptions, as the bands draw them: (start, end, why).
    pub(super) fn gaps(&self) -> impl Iterator<Item = (f64, f64, GapKind)> {
        self.gaps.iter().map(|g| (g.start, g.end, g.kind))
    }

    /// The runs of buckets that each draw as one polyline, split wherever the
    /// trace is interrupted.
    ///
    /// One `make_contiguous` per frame rather than indexing the deque's two
    /// halves apart: it is a memmove only on the frame after the ring wraps.
    pub(super) fn runs(&mut self) -> impl Iterator<Item = &[Bucket]> {
        self.buckets
            .make_contiguous()
            .chunk_by(|_, b| !b.starts_segment)
    }

    /// Number of buckets — the level's whole size, and what the per-frame
    /// cost follows.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.buckets.len()
    }
}
