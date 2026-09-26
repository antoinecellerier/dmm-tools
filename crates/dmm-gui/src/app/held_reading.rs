//! Keeping the reading on screen steady for a meter that sends the parts of
//! one reading in frames of their own — the UT61E+ in AC+DC V, whose DC and
//! AC components take turns.
//!
//! Every frame still goes to the graph, the statistics and the sample buffer
//! as it came. Only what the reading display shows is filled in: the last main
//! reading stays up while a frame carries only a sub-value, and the last such
//! sub-value stays in its row while a frame carries the main reading. Without
//! it the digits would blank every other frame and the row come and go.

use std::time::Instant;

use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};

/// Frames without a part before it may be let go. With the time below, keeps
/// a part across the runs of the other that slower polling produces — the
/// meter keeps its own clock, so a 1 s interval reads DAADDAAD.
const HOLD_FRAMES: u32 = 4;
/// Seconds without a part before it may be let go — and then it is, so a
/// meter stuck sending one part never shows the other as live.
const HOLD_SECS: f64 = 2.0;

/// When a part was last seen.
#[derive(Clone, Copy)]
struct Seen {
    /// Frames since, none of which carried it.
    missing: u32,
    at: Instant,
}

impl Seen {
    fn new(at: Instant) -> Self {
        Seen { missing: 0, at }
    }

    fn stale(&self, now: Instant) -> bool {
        self.missing >= HOLD_FRAMES
            && now
                .checked_duration_since(self.at)
                .is_some_and(|d| d.as_secs_f64() > HOLD_SECS)
    }
}

/// The parts of the reading on screen that came in frames of their own.
#[derive(Default)]
pub(super) struct HeldReading {
    /// When the main reading last came, for as long as it may stand in for
    /// one a frame lacks.
    main: Option<Seen>,
    /// Sub-values that came in frames without the main reading, in the order
    /// first seen.
    parts: Vec<(AuxValue, Seen)>,
}

impl HeldReading {
    /// Forget everything: the next frame is shown as it came. For a
    /// reconnect, a pause or a clear — a reading from before one of those
    /// must never sit beside one from after it.
    pub(super) fn clear(&mut self) {
        self.main = None;
        self.parts.clear();
    }

    /// What the reading display shows for `m`, given what it showed before.
    ///
    /// A frame without the main reading shows `shown` — digits, range,
    /// specifications and any other rows — with this frame's sub-values and
    /// flags in it. A frame with it shows itself, with the held sub-values in
    /// rows of their own ahead of its own. A meter that never sends a frame
    /// without the main reading never gets past the first check, and nothing
    /// is cloned for it.
    pub(super) fn fill_in(&mut self, shown: Option<&Measurement>, m: Measurement) -> Measurement {
        let now = m.timestamp;
        let absent = !m.has_main_reading();
        if !absent && self.parts.is_empty() {
            self.main = Some(Seen::new(now));
            return m;
        }
        // A turn of the dial, or HOLD, MIN/MAX or a range key pressed, and
        // what was held no longer describes what the meter shows.
        if shown.is_some_and(|s| !same_reading(s, &m)) {
            self.clear();
        }

        match &mut self.main {
            Some(seen) if absent => seen.missing += 1,
            _ if absent => {}
            main => *main = Some(Seen::new(now)),
        }
        for (aux, seen) in &mut self.parts {
            match m.aux_values.iter().find(|a| a.label == aux.label) {
                Some(new) if absent => {
                    *aux = new.clone();
                    *seen = Seen::new(now);
                }
                _ => seen.missing += 1,
            }
        }
        if absent {
            for aux in &m.aux_values {
                let held = matches!(aux.value, MeasuredValue::Absent)
                    || self.parts.iter().any(|(a, _)| a.label == aux.label);
                if !held {
                    self.parts.push((aux.clone(), Seen::new(now)));
                }
            }
        }
        self.parts.retain(|(_, seen)| !seen.stale(now));
        if self.main.is_some_and(|seen| seen.stale(now)) {
            self.main = None;
        }

        let base = match shown {
            Some(s) if absent && self.main.is_some() && s.has_main_reading() => {
                let mut out = s.clone();
                out.flags = m.flags;
                out
            }
            _ => m,
        };
        self.with_parts(base)
    }

    /// `m` with each held part in its row: replacing a row of the same
    /// label, or ahead of the frame's own rows.
    fn with_parts(&self, mut m: Measurement) -> Measurement {
        let mut front = 0;
        for (aux, _) in &self.parts {
            match m.aux_values.iter_mut().find(|a| a.label == aux.label) {
                Some(row) => *row = aux.clone(),
                None => {
                    m.aux_values.insert(front, aux.clone());
                    front += 1;
                }
            }
        }
        m
    }
}

/// Whether `m` continues the reading `shown` is of: the same mode, and the
/// same flags among those that change what the meter shows — HOLD, REL,
/// MIN/MAX and Peak. The others can differ between one reading's parts: the
/// AC/DC flag says which part a frame carries, and on mains the high-voltage
/// warning may be lit for the AC part and not the DC one.
fn same_reading(shown: &Measurement, m: &Measurement) -> bool {
    let shows = |f: &StatusFlags| (f.hold, f.rel, f.min, f.max, f.avg, f.peak_min, f.peak_max);
    shown.mode == m.mode && shows(&shown.flags) == shows(&m.flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    fn row(label: &'static str, value: f64) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(value),
            unit: "".into(),
            display_raw: Some(format!("{value:.4}")),
            elapsed_secs: None,
        }
    }

    /// A DC frame of AC+DC V.
    fn dc(value: f64, t: Instant) -> Measurement {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(value),
            "V",
            StatusFlags {
                auto_range: true,
                dc: true,
                ..Default::default()
            },
        );
        m.mode = "AC+DC V".into();
        m.display_raw = Some(format!("{value:.4}"));
        m.range_label = "2.2V".into();
        m.timestamp = t;
        m
    }

    /// An AC frame of AC+DC V: no main reading, the AC component beside it.
    fn ac(value: f64, t: Instant) -> Measurement {
        let mut m = dc(0.0, t);
        m.value = MeasuredValue::Absent;
        m.display_raw = None;
        m.flags.dc = false;
        m.range_label = "22V".into();
        m.aux_values = vec![row("AC", value)];
        m
    }

    /// Feed frames through, as the App does, returning what is shown last.
    fn show(held: &mut HeldReading, frames: Vec<Measurement>) -> Measurement {
        let mut shown: Option<Measurement> = None;
        for m in frames {
            shown = Some(held.fill_in(shown.as_ref(), m));
        }
        shown.expect("at least one frame")
    }

    fn labels(m: &Measurement) -> Vec<&str> {
        m.aux_values.iter().map(|a| a.label.as_ref()).collect()
    }

    #[test]
    fn an_ac_frame_shows_the_last_dc_reading_with_its_ac_row() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let m = show(
            &mut held,
            vec![dc(1.6112, at(t0, 0)), ac(0.0123, at(t0, 667))],
        );
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 1.6112));
        assert_eq!(m.display_raw.as_deref(), Some("1.6112"));
        // The DC frame's range and specifications, not the AC frame's.
        assert_eq!(m.range_label, "2.2V");
        assert!(!m.flags.dc, "the flags are the newest frame's");
        assert_eq!(labels(&m), ["AC"]);
        assert!(matches!(m.aux_values[0].value, MeasuredValue::Normal(v) if v == 0.0123));
    }

    #[test]
    fn a_dc_frame_keeps_the_last_ac_row() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let m = show(
            &mut held,
            vec![
                dc(1.6112, at(t0, 0)),
                ac(0.0123, at(t0, 667)),
                dc(1.6111, at(t0, 1334)),
            ],
        );
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 1.6111));
        assert_eq!(labels(&m), ["AC"]);
    }

    /// A scale's Raw row belongs to the DC reading: it neither blanks on an
    /// AC frame nor swaps places with the AC row.
    #[test]
    fn the_rows_keep_their_order_and_raw_stays_the_dc_one() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let with_raw = |mut m: Measurement, raw: MeasuredValue| {
            let mut r = row("Raw", 0.0);
            r.value = raw;
            m.aux_values.push(r);
            m
        };
        let frames = vec![
            with_raw(dc(1.6112, at(t0, 0)), MeasuredValue::Normal(16.112)),
            with_raw(ac(0.0123, at(t0, 667)), MeasuredValue::Absent),
        ];
        let m = show(&mut held, frames);
        assert_eq!(labels(&m), ["AC", "Raw"]);
        assert!(matches!(m.aux_values[1].value, MeasuredValue::Normal(v) if v == 16.112));

        let m = held.fill_in(
            Some(&m),
            with_raw(dc(1.6111, at(t0, 1334)), MeasuredValue::Normal(16.111)),
        );
        assert_eq!(labels(&m), ["AC", "Raw"]);
    }

    /// The meter's first frame after a connect may be AC: blank digits and
    /// the AC row, not a reading from before.
    #[test]
    fn an_ac_frame_first_shows_blank_digits_and_its_row() {
        let mut held = HeldReading::default();
        let m = show(&mut held, vec![ac(0.0123, Instant::now())]);
        assert!(matches!(m.value, MeasuredValue::Absent));
        assert_eq!(labels(&m), ["AC"]);
    }

    /// HOLD freezes whichever part was on screen, and the meter then sends
    /// only that one: the other part must not stay up beside it as if live.
    #[test]
    fn a_change_of_mode_or_flags_forgets_what_was_held() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let mut on_hold = ac(0.0092, at(t0, 1334));
        on_hold.flags.hold = true;
        let m = show(
            &mut held,
            vec![dc(1.6112, at(t0, 0)), ac(0.0123, at(t0, 667)), on_hold],
        );
        assert!(
            matches!(m.value, MeasuredValue::Absent),
            "no DC beside a held AC"
        );
        assert_eq!(labels(&m), ["AC"]);

        let mut dcv = dc(5.0, at(t0, 2000));
        dcv.mode = "DC V".into();
        let m = held.fill_in(Some(&m), dcv);
        assert!(m.aux_values.is_empty(), "the AC row went with the mode");
    }

    /// On mains the high-voltage warning may be lit for one part and not the
    /// other: that is the same reading, and what is held stays.
    #[test]
    fn a_warning_lit_for_one_part_keeps_what_is_held() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let mut mains_ac = ac(230.1, at(t0, 667));
        mains_ac.flags.hv_warning = true;
        let m = show(&mut held, vec![dc(0.0012, at(t0, 0)), mains_ac]);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0012));
        let m = held.fill_in(Some(&m), dc(0.0013, at(t0, 1334)));
        assert_eq!(labels(&m), ["AC"], "the AC row stays through the DC frame");
    }

    /// A part that stops coming is let go once it is both several frames and
    /// a couple of seconds old — never shown as live indefinitely.
    #[test]
    fn a_part_that_stops_coming_expires() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let mut frames = vec![dc(1.6112, at(t0, 0))];
        frames.extend((1..=3).map(|i| ac(0.0123, at(t0, i * 667))));
        let m = show(&mut held, frames);
        assert!(
            matches!(m.value, MeasuredValue::Normal(_)),
            "three frames is aliasing, not a stop"
        );

        let m = held.fill_in(Some(&m), ac(0.0123, at(t0, 4 * 667)));
        assert!(
            matches!(m.value, MeasuredValue::Absent),
            "gone after four and 2 s"
        );
    }

    /// Other meters never send a frame without the main reading: their
    /// sub-values are shown exactly as they came, a missing one not held.
    #[test]
    fn a_meter_with_sub_values_in_its_frames_is_passed_through() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let mut t2 = dc(23.5, at(t0, 0));
        t2.aux_values = vec![row("T2", 24.1)];
        let m = held.fill_in(None, t2);
        let m = held.fill_in(Some(&m), dc(23.6, at(t0, 100)));
        assert!(m.aux_values.is_empty());
    }

    #[test]
    fn clear_forgets_everything() {
        let t0 = Instant::now();
        let mut held = HeldReading::default();
        let m = show(
            &mut held,
            vec![dc(1.6112, at(t0, 0)), ac(0.0123, at(t0, 667))],
        );
        held.clear();
        let m = held.fill_in(Some(&m), ac(0.0124, at(t0, 1334)));
        assert!(matches!(m.value, MeasuredValue::Absent));
    }
}
