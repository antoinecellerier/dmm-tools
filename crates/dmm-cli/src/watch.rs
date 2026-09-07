//! When has the meter reached the state a capture step asked for?
//!
//! Pure decision logic over readings: the capture loop feeds frames in and
//! advances the step on the answer, so nothing here does I/O or prompting.
//!
//! A step at the dial position the previous reading was already in is
//! Enter-only: only the leads move, and open probes wander enough to satisfy
//! an expectation on their own — shorting them on DC V, or connecting a
//! battery. Such a watcher still reports a mismatch, it just never advances
//! on its own.

use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::protocol::{Expect, ValueExpect};

/// Frames that must agree before the meter counts as settled. Two would fire
/// mid-flip on a meter that reports the new mode a frame before the digits.
pub(crate) const STABLE_FRAMES: usize = 3;

/// What the watcher makes of the frames it has been given so far.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Nothing to report; keep reading.
    Waiting,
    /// The meter settled into a state that is not the one the step asked for.
    Mismatch(String),
    /// The step's state is on screen and holding — capture now.
    Ready,
}

/// The parts of a reading that say which state the meter is in rather than
/// what it is measuring, so moving digits don't read as a state change.
#[derive(Clone, PartialEq, Eq)]
struct Signature {
    mode_raw: u16,
    range_raw: u8,
    flags: StatusFlags,
    /// Payload length: a frame of a different shape is a different state.
    len: usize,
    /// The payload bytes at the baseline's constant positions; empty with no
    /// baseline, where the parsed fields above carry the whole signature.
    stable: Vec<u8>,
}

impl Signature {
    /// The same mode, range and payload shape: only the flags may differ.
    fn same_rung(&self, other: &Signature) -> bool {
        self.mode_raw == other.mode_raw
            && self.range_raw == other.range_raw
            && self.len == other.len
            && self.stable == other.stable
    }

    fn of(m: &Measurement, baseline: Option<&Baseline>) -> Self {
        Signature {
            mode_raw: m.mode_raw,
            range_raw: m.range_raw,
            flags: m.flags,
            len: m.raw_payload.len(),
            stable: baseline.map(|b| b.pick(&m.raw_payload)).unwrap_or_default(),
        }
    }
}

/// The state the meter was left in, as the payload bytes that held still
/// while the previous step was sampled.
///
/// Constancy is only as good as the samples it was built from: a step whose
/// readings never moved (OL, a shorted lead) marks its digits constant too,
/// and the next step then reads a digit change as a state change. The step
/// timeout is what covers that.
#[derive(Clone)]
pub(crate) struct Baseline {
    len: usize,
    /// Positions identical across every sample: the digits moved, the mode,
    /// range and flag bytes did not.
    constant: Vec<usize>,
    /// The bytes those positions held.
    bytes: Vec<u8>,
}

impl Baseline {
    /// From the payloads of the previous step's samples. `None` with no
    /// samples — the first step of a run has nothing to differ from.
    pub(crate) fn from_payloads<'a>(payloads: impl IntoIterator<Item = &'a [u8]>) -> Option<Self> {
        let mut rest = payloads.into_iter();
        let first = rest.next()?.to_vec();
        let rest: Vec<&[u8]> = rest.collect();
        let constant: Vec<usize> = (0..first.len())
            .filter(|&i| rest.iter().all(|p| p.get(i) == Some(&first[i])))
            .collect();
        let bytes = constant.iter().map(|&i| first[i]).collect();
        Some(Baseline {
            len: first.len(),
            constant,
            bytes,
        })
    }

    fn pick(&self, payload: &[u8]) -> Vec<u8> {
        self.constant
            .iter()
            .filter_map(|&i| payload.get(i).copied())
            .collect()
    }

    /// Whether this payload shows a different state from the recorded one.
    fn changed(&self, payload: &[u8]) -> bool {
        payload.len() != self.len || self.pick(payload) != self.bytes
    }
}

/// How a step decides it has arrived.
enum Detector {
    /// The step says what a correct reading looks like, so the parse decides.
    /// Consecutive steps at the same dial position share an expectation, so
    /// the capture loop turns off auto-advance there rather than asking this
    /// detector to tell open leads from shorted ones.
    Semantic(Expect),
    /// Nothing can be asserted about the parse, so any new state that holds
    /// counts. Without a baseline only the operator's Enter ends the wait.
    RawDiff(Option<Baseline>),
}

/// Watches one capture step's readings for the state its instruction asked
/// for.
pub(crate) struct StateWatcher {
    detector: Detector,
    /// Whether a satisfied expectation may capture on its own.
    auto_advance: bool,
    /// Only after a reading in the right mode has failed the expectation:
    /// a step that needs something on the probes is usually reached dial
    /// first, and open leads pass "DC V, finite" before the battery is on.
    /// Continuity going OL to a reading, or NCV from level 0 to 1, is the
    /// change such a step captures on.
    gated: bool,
    armed: bool,
    /// The signature the current run of frames shares, and its length.
    run: Option<(Signature, usize)>,
    /// Frames in a row that satisfied the expectation.
    matched: usize,
    /// The last two signatures, newest first, and how many frames in a row
    /// have matched the one two frames back with only the flags moving: a
    /// meter alternating two states by design, as the UT61E+ does in AC+DC V
    /// where the AC and DC components take turns with a flag saying which.
    /// A range hunt alternating two rungs does not count.
    previous: [Option<Signature>; 2],
    alternating: usize,
    /// The settled state last reported, so a mismatch is printed once instead
    /// of on every frame. A meter that moves away and back reports again.
    reported: Option<Signature>,
}

impl StateWatcher {
    /// Semantic when the step declares what a correct reading looks like,
    /// raw-diff against the previous step's state otherwise. With
    /// `auto_advance` false the watcher only ever reports mismatches.
    pub(crate) fn for_step(
        expect: Option<Expect>,
        baseline: Option<&Baseline>,
        auto_advance: bool,
    ) -> Self {
        StateWatcher {
            detector: match expect {
                Some(expect) => Detector::Semantic(expect),
                None => Detector::RawDiff(baseline.cloned()),
            },
            auto_advance,
            gated: false,
            armed: false,
            run: None,
            matched: 0,
            previous: [None, None],
            alternating: 0,
            reported: None,
        }
    }

    /// Capture on its own only once a reading in the step's mode has failed
    /// the expectation first.
    pub(crate) fn gated(mut self) -> Self {
        self.gated = true;
        self
    }

    /// Judge one reading.
    pub(crate) fn feed(&mut self, m: &Measurement) -> Verdict {
        // Semantic signatures leave the baseline's bytes out: they only have
        // to say when the meter stopped moving, and a previous step whose
        // reading never moved marks its digits constant too.
        let sig = Signature::of(
            m,
            match &self.detector {
                Detector::RawDiff(baseline) => baseline.as_ref(),
                Detector::Semantic(_) => None,
            },
        );
        let run = match self.run.take() {
            Some((prev, n)) if prev == sig => n + 1,
            _ => 1,
        };
        self.run = Some((sig.clone(), run));
        let blinking = self.previous[1].as_ref() == Some(&sig)
            && self.previous[0]
                .as_ref()
                .is_some_and(|last| last.same_rung(&sig));
        self.alternating = if blinking { self.alternating + 1 } else { 0 };
        self.previous = [Some(sig.clone()), self.previous[0].take()];
        let settled = run >= STABLE_FRAMES || self.alternating >= 2 * STABLE_FRAMES;

        if let Detector::Semantic(expect) = &self.detector {
            // Read out before the arms below touch the watcher's counters.
            let expect = *expect;
            return match expect.check(m) {
                Ok(()) => {
                    self.matched += 1;
                    // The expectation holding is not the meter having settled:
                    // an autoranging meter satisfies "Ω, OL" on every rung it
                    // hunts through. The signature has to hold too.
                    let armed = self.armed || !self.gated;
                    if self.matched >= STABLE_FRAMES && settled && self.auto_advance && armed {
                        Verdict::Ready
                    } else {
                        Verdict::Waiting
                    }
                }
                Err(reason) => {
                    self.matched = 0;
                    if expect.mode.is_none_or(|want| m.mode == want) {
                        self.armed = true;
                    }
                    if run >= STABLE_FRAMES && self.reported.as_ref() != Some(&sig) {
                        self.reported = Some(sig);
                        Verdict::Mismatch(reason)
                    } else {
                        Verdict::Waiting
                    }
                }
            };
        }

        let Detector::RawDiff(Some(baseline)) = &self.detector else {
            return Verdict::Waiting;
        };
        if baseline.changed(&m.raw_payload) && run >= STABLE_FRAMES && self.auto_advance {
            Verdict::Ready
        } else {
            Verdict::Waiting
        }
    }

    /// A frame the parser rejected: the meter is mid-change or misread, so
    /// the run of agreeing frames starts over.
    pub(crate) fn feed_error(&mut self) -> Verdict {
        self.run = None;
        self.matched = 0;
        Verdict::Waiting
    }
}

/// Whether nothing observable would announce that the step was done: the
/// meter is already in the step's mode, so only the leads move.
///
/// Open probes wander, and a wobble is not the action: -0.0013 V of lead
/// noise satisfied "DC V, negative" before the battery was connected. The one
/// exception is OL turning into a reading — Ω open to Ω across the body —
/// which open leads cannot fake, so that step waits for it.
pub(crate) fn enter_only(expect: Option<Expect>, previous: Option<&Measurement>) -> bool {
    // Raw-diff spots its own change, and a run's first step has nothing to
    // compare against.
    let (Some(expect), Some(m)) = (expect, previous) else {
        return false;
    };
    if !matches!(expect.mode, Some(mode) if m.mode == mode) {
        return false;
    }
    !(matches!(m.value, MeasuredValue::Overload) && expect.value == Some(ValueExpect::Finite))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::ValueExpect;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;

    /// A UT61E+ reading: mode byte, autorange, the digits given.
    fn reading(mode: u8, digits: &[u8; 7]) -> Measurement {
        make_test_measurement(mode, 0x01, digits, (0x00, 0x00), (0x00, 0x00, 0x00))
    }

    fn dcv(digits: &[u8; 7]) -> Measurement {
        reading(0x02, digits)
    }

    fn acv(digits: &[u8; 7]) -> Measurement {
        reading(0x00, digits)
    }

    fn ohm(digits: &[u8; 7]) -> Measurement {
        reading(0x06, digits)
    }

    /// DC V on a named range rung, for the meter that is still hunting.
    fn dcv_on(range: u8) -> Measurement {
        make_test_measurement(0x02, range, b"     OL", (0x00, 0x00), (0x00, 0x00, 0x00))
    }

    /// The previous step's samples, as the capture loop hands them over.
    fn baseline_of(samples: &[Measurement]) -> Baseline {
        Baseline::from_payloads(samples.iter().map(|m| m.raw_payload.as_slice()))
            .expect("samples to build a baseline from")
    }

    /// Three agreeing frames are the whole condition: the step captures
    /// without the operator touching a key.
    #[test]
    fn a_matching_state_is_ready_on_the_third_frame() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.235")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.236")), Verdict::Ready);
    }

    /// A meter mid-flip alternates; capturing there files half a state.
    #[test]
    fn a_flapping_meter_never_becomes_ready() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        for i in 0..10 {
            let m = if i % 2 == 0 {
                dcv(b"  1.234")
            } else {
                acv(b"  1.234")
            };
            assert_eq!(w.feed(&m), Verdict::Waiting, "frame {i}");
        }
    }

    /// An autoranging meter satisfies "DC V, OL" on every rung it hunts
    /// through, so the expectation alone would file a sample from a range the
    /// meter is about to leave.
    #[test]
    fn a_hunting_range_never_becomes_ready() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        for i in 0..8 {
            let rung = if i % 2 == 0 { 0x01 } else { 0x02 };
            assert_eq!(w.feed(&dcv_on(rung)), Verdict::Waiting, "frame {i}");
        }
        // The same mode once one rung holds is what the step is waiting for.
        assert_eq!(w.feed(&dcv_on(0x01)), Verdict::Waiting);
        assert_eq!(w.feed(&dcv_on(0x01)), Verdict::Waiting);
        assert_eq!(w.feed(&dcv_on(0x01)), Verdict::Ready);
    }

    /// AC+DC V on the UT61E+ sends the AC and DC components in turn, a flag
    /// toggling with them, so no three frames ever agree; the step still has
    /// to capture. A hunt between two rungs is not that (above).
    #[test]
    fn a_flag_blinking_by_design_still_settles() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        let frame = |i: usize| {
            let flag3 = if i.is_multiple_of(2) { 0x00 } else { 0x08 };
            make_test_measurement(0x02, 0x01, b" 0.0008", (0x00, 0x00), (0x00, 0x00, flag3))
        };
        for i in 0..7 {
            assert_eq!(w.feed(&frame(i)), Verdict::Waiting, "frame {i}");
        }
        assert_eq!(w.feed(&frame(7)), Verdict::Ready);
    }

    /// A gated step reached dial first: open leads already pass "DC V,
    /// finite", so the pass alone is not the battery. Only a failing reading
    /// in the right mode arms it, and the wrong mode failing does not.
    #[test]
    fn a_gated_step_captures_only_after_a_failing_reading_in_its_mode() {
        let finite_dcv = Expect::mode("DC V").value(ValueExpect::Finite);
        let mut w = StateWatcher::for_step(Some(finite_dcv), None, true).gated();
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        for i in 0..6 {
            assert_eq!(w.feed(&dcv(b" 0.0007")), Verdict::Waiting, "frame {i}");
        }

        let mut w = StateWatcher::for_step(Some(finite_dcv), None, true).gated();
        assert_eq!(w.feed(&dcv_on(0x01)), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b" 1.6108")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b" 1.6108")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b" 1.6108")), Verdict::Ready);
    }

    /// The wrong dial position settles too, and the operator has to be told
    /// once — not on every frame for as long as they take to find the right
    /// one.
    #[test]
    fn a_settled_wrong_mode_is_reported_once() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        let Verdict::Mismatch(reason) = w.feed(&acv(b"  1.234")) else {
            panic!("a settled wrong mode must be reported");
        };
        assert!(reason.contains("AC V"), "got {reason}");
        for _ in 0..5 {
            assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        }
        // And the right mode still captures.
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Ready);
    }

    /// With no expectation to check, a state that is simply not the previous
    /// step's is what advances — this is the detector for a family whose
    /// parser is not trusted yet.
    #[test]
    fn raw_diff_advances_on_a_new_state() {
        let previous = [dcv(b"     OL"), dcv(b"     OL"), dcv(b"     OL")];
        let mut w = StateWatcher::for_step(None, Some(&baseline_of(&previous)), true);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&acv(b"  1.234")), Verdict::Ready);
    }

    /// A reading that wanders is the meter doing its job, not the operator
    /// turning the dial.
    #[test]
    fn moving_digits_are_not_a_new_state() {
        let previous = [dcv(b"  1.234"), dcv(b"  1.298"), dcv(b"  1.351")];
        let mut w = StateWatcher::for_step(None, Some(&baseline_of(&previous)), true);
        for digits in [b"  1.240", b"  1.241", b"  1.242", b"  1.987"] {
            assert_eq!(w.feed(&dcv(digits)), Verdict::Waiting);
        }
    }

    /// The first step of a run has no previous state, so raw-diff has nothing
    /// to conclude and the operator's Enter is the only way on.
    #[test]
    fn raw_diff_without_a_baseline_never_becomes_ready() {
        let mut w = StateWatcher::for_step(None, None, true);
        for _ in 0..5 {
            assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        }
    }

    /// An Enter-only step never captures on its own — nothing distinguishes
    /// open leads from shorted ones on DC V — but the wrong dial position is
    /// still worth saying once.
    #[test]
    fn an_enter_only_watcher_never_advances_but_still_reports() {
        let expect = Expect::mode("DC V").value(ValueExpect::Finite);
        let mut w = StateWatcher::for_step(Some(expect), None, false);
        for _ in 0..6 {
            assert_eq!(w.feed(&dcv(b" 0.0000")), Verdict::Waiting);
        }
        assert_eq!(w.feed(&acv(b" 0.0000")), Verdict::Waiting);
        assert_eq!(w.feed(&acv(b" 0.0000")), Verdict::Waiting);
        let Verdict::Mismatch(reason) = w.feed(&acv(b" 0.0000")) else {
            panic!("a settled wrong mode must be reported");
        };
        assert!(reason.contains("AC V"), "got {reason}");
    }

    /// The state the step asked for, held for three frames, is the whole
    /// condition once auto-advance is on.
    #[test]
    fn a_matching_state_is_ready_with_a_value_expectation() {
        let expect = Expect::mode("\u{3a9}").value(ValueExpect::Finite);
        let mut w = StateWatcher::for_step(Some(expect), None, true);
        assert_eq!(w.feed(&ohm(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&ohm(b"  1.235")), Verdict::Waiting);
        assert_eq!(w.feed(&ohm(b"  1.236")), Verdict::Ready);
    }

    /// The cases the gate walk produces: a step the dial doesn't move for
    /// asks for Enter, and only OL becoming a reading is watched for.
    #[test]
    fn enter_only_at_the_previous_reading_s_dial_position() {
        let finite_dcv = Expect::mode("DC V").value(ValueExpect::Finite);
        // dcv → dcv_short: 0.0002 is finite already, so shorting the
        // probes changes nothing the tool can see.
        assert!(enter_only(Some(finite_dcv), Some(&dcv(b" 0.0002"))));

        // dcv → dcv_negative: lead noise on open probes goes negative on its
        // own, so the sign is not evidence the battery was connected.
        let negative_dcv = Expect::mode("DC V").value(ValueExpect::Negative);
        assert!(enter_only(Some(negative_dcv), Some(&dcv(b" 0.0000"))));

        let finite_ohm = Expect::mode("\u{3a9}").value(ValueExpect::Finite);
        // ohm → ohm_body: same dial position, but OL to a finite
        // reading is a change open leads cannot fake.
        assert!(!enter_only(Some(finite_ohm), Some(&ohm(b"     OL"))));
        // ohm_body → ohm_short: both finite, so it asks.
        assert!(enter_only(Some(finite_ohm), Some(&ohm(b"  1.234"))));

        // A different mode still has to be watched for, as do the first step
        // of a run and any raw-diff step.
        assert!(!enter_only(Some(finite_dcv), Some(&acv(b" 0.0001"))));
        assert!(!enter_only(Some(finite_dcv), None));
        assert!(!enter_only(None, Some(&dcv(b" 0.0001"))));
    }

    /// A rejected frame means the state is not settled: the run restarts so
    /// the frames either side of it aren't counted as consecutive.
    #[test]
    fn a_rejected_frame_restarts_the_run() {
        let mut w = StateWatcher::for_step(Some(Expect::mode("DC V")), None, true);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed_error(), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Waiting);
        assert_eq!(w.feed(&dcv(b"  1.234")), Verdict::Ready);
    }

    /// Nothing to compare against and nothing to assert: an empty previous
    /// step must not produce a baseline that matches everything.
    #[test]
    fn no_samples_give_no_baseline() {
        let empty: [&[u8]; 0] = [];
        assert!(Baseline::from_payloads(empty).is_none());
    }
}
