//! Paced measurement stream.
//!
//! Wraps a [`Dmm`] with the sample interval and consecutive-timeout
//! counting, so the CLI `read`/`debug` loop and the GUI background thread
//! share the same acquisition logic. The interval means one thing for every
//! meter — at most one reading per interval, a zero interval every reading
//! the meter produces — and is met two ways, by the meter's
//! [`Delivery`](crate::protocol::Delivery):
//!
//! - **Polled**: the stream sleeps to each tick, then asks for a reading.
//! - **Streamed**: the stream reads every frame as it arrives, so none waits
//!   in a queue and each is stamped when it came, and keeps the one nearest
//!   each tick. Reading only when a reading is wanted would leave the meter's
//!   frames queuing in between, handed out later stamped as new.
//!
//! The stream intentionally does not own cancellation — the CLI uses an
//! `AtomicBool` driven by the Ctrl-C handler while the GUI uses an `mpsc`
//! stop channel, and neither fits naturally inside the other. Callers check
//! their own stop signal around each [`MeasurementStream::tick`] call, and
//! can hand the stream a predicate via [`MeasurementStream::with_cancel`] so
//! the wait for a reading gives up on that same signal instead of running
//! to term.

use crate::Dmm;
use crate::clock::Clock;
use crate::error::{Error, ErrorKind, Result};
use crate::measurement::Measurement;
use crate::protocol::Delivery;
use crate::transport::{LateReadings, Transport};
use log::{debug, trace, warn};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Consecutive read timeouts after which both binaries treat the meter as not
/// responding: the CLI prints its activation help, the GUI says so and marks
/// the graph with a genuine loss of data rather than a quiet meter.
pub const NO_RESPONSE_TIMEOUTS: u32 = 5;

/// Outcome of one stream tick.
///
/// The `Measurement` variant carries the full parsed struct, which is larger
/// than the `Timeout` variant but short-lived — events are matched on the
/// same thread they're produced and do not accumulate.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum StreamEvent {
    /// Measurement received. The consecutive-timeout counter has been reset.
    Measurement(Measurement),
    /// No response within the protocol's read timeout. `consecutive` is the
    /// new counter value (1 on the first timeout after a successful read).
    Timeout { consecutive: u32 },
    /// Nothing more will come: a replay has handed out its last frame. Every
    /// later tick says so again.
    Ended,
}

/// Paced acquisition wrapper around a [`Dmm`].
///
/// Construct with [`MeasurementStream::new`] and drive by calling
/// [`tick`](Self::tick) repeatedly; each call returns one reading (or a
/// timeout), paced to the interval as the module doc describes.
///
/// The stream borrows the `Dmm` mutably so callers can keep control over
/// ownership (e.g. to call `send_command` between ticks). Ticks are
/// absolute — the Nth lands at `start + N*tick` regardless of how long a
/// request took — so the cadence does not drift.
pub struct MeasurementStream<'a, T: Transport> {
    dmm: &'a mut Dmm<T>,
    /// The session clock, cloned from the `Dmm` so pacing and the timestamps
    /// it produces share one time base.
    clock: Clock,
    tick: Duration,
    next_tick: Option<Instant>,
    /// Where a streaming meter's grid of ticks starts: the first reading
    /// kept.
    anchor: Option<Instant>,
    /// The streaming meter's recent frame spacings, for [`Self::tolerance`].
    spacing: Spacing,
    /// Readings seen arriving two at a time, for [`Self::take_late_readings`].
    pairs: Pairs,
    consecutive_timeouts: u32,
    /// `'static` rather than `'a`: a borrowed predicate would make the struct
    /// invariant in `'a`, which stops callers shortening the `&mut Dmm` borrow
    /// (the GUI reassigns `dmm` after a reconnect). Both callers hand over an
    /// owned `Arc` anyway.
    cancel: Option<Box<dyn Fn() -> bool + Send + 'static>>,
}

/// Longest single sleep inside a cancellable pacing wait.
///
/// The wait is split into slices so the cancel predicate is polled about this
/// often. Small enough that shutdown feels immediate, large enough that a slow
/// sample interval doesn't spin.
const CANCEL_POLL_SLICE: Duration = Duration::from_millis(50);

/// The longest tick a stream runs at. Anything longer is a mistyped
/// interval (the GUI already clamps to a minute), and a bound keeps the tick
/// arithmetic on `Instant` clear of overflow.
const MAX_TICK: Duration = Duration::from_secs(24 * 3600);

/// Frame spacings the tolerance is worked out from.
const SPACINGS_KEPT: usize = 16;

/// Spacings needed before there is a tolerance at all: fewer say too little
/// about the meter's rate.
const SPACINGS_NEEDED: usize = 3;

/// Readings that must arrive close behind the one before, each well apart
/// from the last, before the link counts as handing them over two at a time.
const PAIRS_FOR_NOTICE: u32 = 3;

/// Readings arriving two at a time: a link slower than the meter carries
/// one late, together with the next, which comes with a gap as long again
/// before or after it.
#[derive(Default)]
struct Pairs {
    counted: u32,
    /// When the last one counted came: the rest of a backlog handed over
    /// at once, after a stall, is one burst and counts once.
    last: Option<Instant>,
    /// The count is reached and the link asked; nothing more is watched.
    settled: bool,
    notice: Option<LateReadings>,
}

/// The time between a streaming meter's recent frames, whose median is its
/// frame period.
#[derive(Default)]
struct Spacing {
    last: Option<Instant>,
    recent: VecDeque<Duration>,
}

impl Spacing {
    /// Take in a frame at `at`, handing back its gap from the one before.
    fn observe(&mut self, at: Instant) -> Option<Duration> {
        let gap = self.last.and_then(|last| at.checked_duration_since(last));
        if let Some(gap) = gap {
            if self.recent.len() == SPACINGS_KEPT {
                self.recent.pop_front();
            }
            self.recent.push_back(gap);
        }
        self.last = Some(at);
        gap
    }

    fn period(&self) -> Option<Duration> {
        if self.recent.len() < SPACINGS_NEEDED {
            return None;
        }
        let mut sorted: Vec<Duration> = self.recent.iter().copied().collect();
        sorted.sort_unstable();
        Some(sorted[sorted.len() / 2])
    }
}

impl<'a, T: Transport> MeasurementStream<'a, T> {
    /// Build a stream around `dmm` keeping at most one reading per `tick`.
    /// A zero tick keeps every reading the meter produces, as fast as it
    /// produces them.
    pub fn new(dmm: &'a mut Dmm<T>, tick: Duration) -> Self {
        Self {
            clock: dmm.clock().clone(),
            dmm,
            tick: tick.min(MAX_TICK),
            next_tick: None,
            anchor: None,
            spacing: Spacing::default(),
            pairs: Pairs::default(),
            consecutive_timeouts: 0,
            cancel: None,
        }
    }

    /// Poll `cancel` while waiting for a reading, and cut the wait short
    /// when it returns true.
    ///
    /// Without this the wait runs for the whole sample interval no matter
    /// what: at a 2 s interval a shutdown request isn't noticed until the tick
    /// elapses (plus the read timeout that follows), so the caller keeps the
    /// device open long after the user asked it to stop. A hand-edited
    /// interval of minutes wedges it entirely.
    ///
    /// The predicate only shortens the wait — `tick` still returns an event:
    /// a polled meter is asked once more, and a streaming one hands over the
    /// newest frame it read, or the next one to arrive.
    pub fn with_cancel(mut self, cancel: impl Fn() -> bool + Send + 'static) -> Self {
        self.cancel = Some(Box::new(cancel));
        self
    }

    /// Read one measurement, paced to the tick schedule.
    ///
    /// Returns `Ok(Measurement)` or `Ok(Timeout)`. Non-timeout transport
    /// errors bubble up as `Err(_)` and leave the counter unchanged; the
    /// caller decides whether to reconnect or abort.
    ///
    /// Named `tick` (not `next`) to avoid colliding with [`Iterator::next`],
    /// which this type deliberately does not implement — iterators can't
    /// return errors without the caller explicitly handling the `Result`.
    pub fn tick(&mut self) -> Result<StreamEvent> {
        if self.dmm.ended() {
            return Ok(StreamEvent::Ended);
        }
        let result = match self.dmm.delivery() {
            Delivery::Polled => {
                self.sleep_until_tick();
                self.dmm.request_measurement().map(Some)
            }
            Delivery::Streamed => self.next_streamed(),
        };
        match result {
            Ok(None) => Ok(StreamEvent::Ended),
            Ok(Some(m)) => {
                self.consecutive_timeouts = 0;
                Ok(StreamEvent::Measurement(m))
            }
            Err(Error::Timeout) => {
                self.consecutive_timeouts = self.consecutive_timeouts.saturating_add(1);
                Ok(StreamEvent::Timeout {
                    consecutive: self.consecutive_timeouts,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Keep at most one reading per `tick` from now on, a zero tick every
    /// reading. The schedule starts afresh at the next reading, as it did
    /// when the stream was built.
    pub fn set_tick(&mut self, tick: Duration) {
        self.tick = tick.min(MAX_TICK);
        self.next_tick = None;
        self.anchor = None;
    }

    /// The notice for readings arriving two at a time, once, when a
    /// streaming meter's link has been seen handing them over that way and
    /// knows what to say about it ([`Transport::late_readings`]). The stream
    /// has logged it as a warning already.
    pub fn take_late_readings(&mut self) -> Option<LateReadings> {
        self.pairs.notice.take()
    }

    /// Number of consecutive timeouts since the last successful measurement.
    pub fn consecutive_timeouts(&self) -> u32 {
        self.consecutive_timeouts
    }

    /// Read-only access to the underlying `Dmm`, for queries that don't need
    /// the mutable borrow — device profile, mode choices, transport info.
    pub fn dmm(&self) -> &Dmm<T> {
        self.dmm
    }

    /// Mutable access to the underlying `Dmm`. Useful for sending commands
    /// or reading transport info between ticks.
    pub fn dmm_mut(&mut self) -> &mut Dmm<T> {
        self.dmm
    }

    /// Read a streaming meter's frames until one is due, keeping none of the
    /// others. A protocol error in between is dropped as the frames are: it
    /// stands for the interval only if nothing good arrives by the tick.
    /// `None` once the source has ended with nothing left to hand over; the
    /// last frame of a replay is kept whatever the tick.
    fn next_streamed(&mut self) -> Result<Option<Measurement>> {
        let mut dropped: Option<Measurement> = None;
        let mut skipped = 0u32;
        loop {
            if self.dmm.ended() {
                return Ok(dropped);
            }
            if self.cancelled()
                && let Some(m) = dropped.take()
            {
                self.keep(m.timestamp);
                return Ok(Some(m));
            }
            match self.dmm.request_measurement() {
                Ok(m) => {
                    let gap = self.spacing.observe(m.timestamp);
                    self.watch_pairs(m.timestamp, gap);
                    if self.keep(m.timestamp) {
                        trace!("stream: kept a reading, {skipped} frames since the last");
                        return Ok(Some(m));
                    }
                    skipped += 1;
                    dropped = Some(m);
                }
                Err(e) if e.kind() == ErrorKind::Protocol && !self.due(self.clock.now()) => {
                    skipped += 1;
                }
                Err(e) => {
                    if e.kind() == ErrorKind::Protocol && !self.tick.is_zero() {
                        // The tick has come with only an error in hand: a
                        // good frame from this window stands for it instead.
                        self.advance(self.clock.now());
                        if let Some(m) = dropped {
                            return Ok(Some(m));
                        }
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Count a frame at `at` that came `gap` after the one before it if it
    /// came with that one, well under the frame period after it; at
    /// [`PAIRS_FOR_NOTICE`], ask the link what to say.
    ///
    /// Only one per two frame periods counts, so a backlog handed over at
    /// once counts once; a link that bunches does it again and again, a
    /// reading at a time.
    fn watch_pairs(&mut self, at: Instant, gap: Option<Duration>) {
        if self.pairs.settled {
            return;
        }
        let (Some(gap), Some(period)) = (gap, self.spacing.period()) else {
            return;
        };
        // Strict, so a meter bunched so often that the median itself is
        // nothing never counts.
        if gap >= period / 4 {
            return;
        }
        if let Some(last) = self.pairs.last
            && at
                .checked_duration_since(last)
                .is_none_or(|d| d < period * 2)
        {
            return;
        }
        self.pairs.last = Some(at);
        self.pairs.counted += 1;
        if self.pairs.counted < PAIRS_FOR_NOTICE {
            return;
        }
        self.pairs.settled = true;
        match self.dmm.transport().late_readings() {
            Some(notice) => {
                warn!("{notice}");
                debug!(
                    "stream: readings in pairs, frame period {period:?}; link: {:?}",
                    self.dmm.transport().transport_status()
                );
                self.pairs.notice = Some(notice);
            }
            None => debug!("stream: readings in pairs, frame period {period:?}"),
        }
    }

    /// Whether a frame at `at` is the one to keep for the next tick, moving
    /// the tick on when it is.
    ///
    /// The frame nearest each tick, not the first one after it: a meter's
    /// frames wander by a few milliseconds, and with an interval a whole
    /// multiple of its frame period they sit right on the ticks, so taking
    /// the first after each would jitter by a whole period, dropping frames
    /// an interval equal to the period should keep.
    fn keep(&mut self, at: Instant) -> bool {
        if self.tick.is_zero() {
            return true;
        }
        if !self.due(at) {
            return false;
        }
        self.advance(at);
        true
    }

    /// Move the next tick to the first point of the grid after a reading at
    /// `at`, starting the grid there if there is none yet.
    fn advance(&mut self, at: Instant) {
        let anchor = *self.anchor.get_or_insert(at);
        let past = (at + self.tolerance())
            .checked_duration_since(anchor)
            .unwrap_or_default();
        let tick_ns = self.tick.as_nanos();
        let offset_ns = (past.as_nanos() / tick_ns + 1).saturating_mul(tick_ns);
        let offset = Duration::new(
            u64::try_from(offset_ns / 1_000_000_000).unwrap_or(u64::MAX),
            (offset_ns % 1_000_000_000) as u32,
        );
        // Unreachable within a clamped tick of any real session; the old
        // tick stands rather than a panic.
        if let Some(next) = anchor.checked_add(offset) {
            self.next_tick = Some(next);
        }
    }

    /// Whether `at` is within the tolerance of the next tick, or past it.
    fn due(&self, at: Instant) -> bool {
        self.next_tick
            .is_none_or(|next| at + self.tolerance() >= next)
    }

    /// How early a frame may come and still count for the next tick: half
    /// the meter's frame period, so each tick takes the frame nearest it,
    /// and at most half the interval, so no two frames count for one tick.
    /// None until the period is known, when the first frame at or after the
    /// tick counts.
    fn tolerance(&self) -> Duration {
        self.spacing
            .period()
            .map_or(Duration::ZERO, |period| (period / 2).min(self.tick / 2))
    }

    fn cancelled(&self) -> bool {
        self.cancel.as_ref().is_some_and(|cancel| cancel())
    }

    fn sleep_until_tick(&mut self) {
        if self.tick.is_zero() {
            return;
        }
        let now = self.clock.now();
        match self.next_tick {
            Some(target) => {
                if let Some(wait) = target.checked_duration_since(now) {
                    self.sleep_cancellable(wait);
                }
                let mut next = target + self.tick;
                let now2 = self.clock.now();
                if next < now2 {
                    next = now2 + self.tick;
                }
                self.next_tick = Some(next);
            }
            None => {
                // First tick fires immediately; subsequent ticks land on the
                // schedule anchored here.
                self.next_tick = Some(now + self.tick);
            }
        }
    }

    /// Sleep for `wait`, returning early if the cancel predicate fires.
    ///
    /// With no predicate this is a single clock sleep; the slicing costs
    /// nothing when nobody is watching for cancellation.
    fn sleep_cancellable(&self, wait: Duration) {
        let Some(cancel) = &self.cancel else {
            self.clock.sleep(wait);
            return;
        };
        let deadline = self.clock.now() + wait;
        loop {
            if cancel() {
                return;
            }
            // `checked_duration_since` rather than a subtraction: a backward
            // clock jump must not panic here. Nothing remaining ends the wait
            // too: on a clock that only moves when it is slept on, sleeping
            // zero would spin forever waiting to pass the deadline.
            let remaining = deadline.checked_duration_since(self.clock.now());
            let Some(remaining) = remaining.filter(|r| !r.is_zero()) else {
                return;
            };
            self.clock.sleep(remaining.min(CANCEL_POLL_SLICE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::Ut61PlusProtocol;
    use crate::transport::mock::MockTransport;

    fn build_response(display: &[u8; 7]) -> Vec<u8> {
        let payload: Vec<u8> = vec![
            0x02, // DC V
            0x31, display[0], display[1], display[2], display[3], display[4], display[5],
            display[6], 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        crate::protocol::framing::test_frame_be16(&payload)
    }

    fn new_dmm(responses: Vec<Vec<u8>>) -> Dmm<MockTransport> {
        let mock = MockTransport::new(responses);
        let protocol = Box::new(Ut61PlusProtocol::new());
        Dmm::new(mock, protocol).unwrap()
    }

    /// A `Dmm` whose session time the test drives. The pacing waits then cost
    /// no wall time and land on exact virtual deltas, so these tests assert
    /// what the pacing did rather than how long it took.
    fn new_clocked_dmm(responses: Vec<Vec<u8>>, clock: &Clock) -> Dmm<MockTransport> {
        new_dmm(responses).with_clock(clock.clone())
    }

    fn two_readings() -> Vec<Vec<u8>> {
        vec![build_response(b"  1.000"), build_response(b"  2.000")]
    }

    #[test]
    fn measurement_resets_timeout_counter() {
        let mut dmm = new_dmm(vec![build_response(b"  1.000"), build_response(b"  2.000")]);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);

        let e = stream.tick().unwrap();
        assert!(matches!(e, StreamEvent::Measurement(_)));
        assert_eq!(stream.consecutive_timeouts(), 0);
    }

    #[test]
    fn timeout_increments_counter() {
        // MockTransport with no responses returns 0 bytes → Error::Timeout.
        let mut dmm = new_dmm(vec![]);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);

        let e = stream.tick().unwrap();
        assert!(matches!(e, StreamEvent::Timeout { consecutive: 1 }));
        assert_eq!(stream.consecutive_timeouts(), 1);

        let e = stream.tick().unwrap();
        assert!(matches!(e, StreamEvent::Timeout { consecutive: 2 }));
    }

    #[test]
    fn timeout_counter_resets_on_measurement() {
        // Start silent so the first tick times out.
        let mut dmm = new_dmm(vec![]);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);

        let _ = stream.tick();
        assert_eq!(stream.consecutive_timeouts(), 1);

        // The meter answers the next request on the same stream.
        stream
            .dmm
            .transport()
            .push_reply(build_response(b"  3.000"));
        let _ = stream.tick();
        assert_eq!(stream.consecutive_timeouts(), 0);
    }

    #[test]
    fn pacing_sleeps_between_ticks() {
        let clock = Clock::manual();
        let mut dmm = new_clocked_dmm(two_readings(), &clock);
        let tick = Duration::from_millis(50);
        let mut stream = MeasurementStream::new(&mut dmm, tick);

        let start = clock.now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        // The first tick fires immediately, the second one interval later.
        assert_eq!(clock.now().saturating_duration_since(start), tick);
    }

    /// A long sample interval used to hold the device open for the whole tick
    /// after the user asked to stop.
    #[test]
    fn cancel_cuts_the_pacing_sleep_short() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let clock = Clock::manual();
        let mut dmm = new_clocked_dmm(two_readings(), &clock);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(10))
            .with_cancel(move || flag.load(Ordering::Relaxed));

        let _ = stream.tick().unwrap(); // first tick fires immediately
        stop.store(true, Ordering::Relaxed);

        let start = clock.now();
        let _ = stream.tick().unwrap();
        let waited = clock.now().saturating_duration_since(start);
        assert!(
            waited <= CANCEL_POLL_SLICE,
            "cancelled sleep should give up within a poll slice of the 10 s tick, waited {waited:?}"
        );
    }

    /// The predicate must not shorten a wait nobody asked to cancel.
    #[test]
    fn cancel_predicate_that_stays_false_still_paces() {
        let clock = Clock::manual();
        let mut dmm = new_clocked_dmm(two_readings(), &clock);
        // Not a multiple of CANCEL_POLL_SLICE: the sliced wait must still add
        // up to exactly one interval.
        let tick = Duration::from_millis(120);
        let mut stream = MeasurementStream::new(&mut dmm, tick).with_cancel(|| false);

        let start = clock.now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        assert_eq!(clock.now().saturating_duration_since(start), tick);
    }

    #[test]
    fn zero_tick_disables_pacing() {
        let clock = Clock::manual();
        let mut dmm = new_clocked_dmm(two_readings(), &clock);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);

        let start = clock.now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        // Nothing waited, so nothing advanced the session clock.
        assert_eq!(clock.now().saturating_duration_since(start), Duration::ZERO);
    }

    /// A streaming meter on the session clock: frame `k` is sent at
    /// `offsets[k]` and read at once, its payload the frame's index, or a
    /// corrupt frame where `corrupt` says so.
    struct TimedMeter {
        clock: Clock,
        start: Instant,
        offsets: Vec<Duration>,
        corrupt: Vec<usize>,
        next: usize,
    }

    const TIMED_METER: crate::protocol::DeviceProfile = crate::protocol::DeviceProfile {
        family_name: "test",
        model_name: "timed meter",
        stability: crate::protocol::Stability::Verified,
        supported_commands: &[],
        max_aux_values: 0,
        verification_issue: None,
        meter_keys: crate::protocol::MeterKeys::NONE,
    };

    impl crate::protocol::Protocol for TimedMeter {
        fn init(&mut self, _t: &dyn Transport) -> Result<()> {
            Ok(())
        }
        fn request_measurement(&mut self, _t: &dyn Transport) -> Result<Measurement> {
            let k = self.next;
            let due = self.start + self.offsets[k];
            if let Some(wait) = due.checked_duration_since(self.clock.now()) {
                self.clock.sleep(wait);
            }
            self.next += 1;
            if self.corrupt.contains(&k) {
                return Err(Error::invalid_response_msg("corrupt frame"));
            }
            Ok(Measurement::from_payload(&(k as u32).to_le_bytes()))
        }
        fn delivery(&self) -> Delivery {
            Delivery::Streamed
        }
        fn discard_input(&mut self, _t: &dyn Transport) -> Result<()> {
            Ok(())
        }
        fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
            Ok(Measurement::from_payload(payload))
        }
        fn profile(&self) -> &crate::protocol::DeviceProfile {
            &TIMED_METER
        }
    }

    /// A meter sending every `period`, each frame off by up to ±5 ms, in a
    /// fixed pseudo-random pattern.
    fn jittered(period_ms: u64, frames: usize) -> Vec<Duration> {
        const WANDER_US: [i64; 7] = [4_000, -3_000, 1_000, -5_000, 2_000, 5_000, -2_000];
        (0..frames)
            .map(|k| {
                let us = (k as u64 * period_ms * 1000) as i64 + WANDER_US[k % WANDER_US.len()];
                Duration::from_micros(us.max(0) as u64)
            })
            .collect()
    }

    fn timed_dmm(
        offsets: Vec<Duration>,
        corrupt: Vec<usize>,
    ) -> (Dmm<crate::transport::NullTransport>, Clock) {
        timed_dmm_on(crate::transport::NullTransport, offsets, corrupt)
    }

    fn timed_dmm_on<T: Transport>(
        transport: T,
        offsets: Vec<Duration>,
        corrupt: Vec<usize>,
    ) -> (Dmm<T>, Clock) {
        let clock = Clock::manual();
        let meter = TimedMeter {
            start: clock.now(),
            clock: clock.clone(),
            offsets,
            corrupt,
            next: 0,
        };
        let dmm = Dmm::new(transport, Box::new(meter))
            .unwrap()
            .with_clock(clock.clone());
        (dmm, clock)
    }

    /// The frame index and session time of each of `n` readings kept at
    /// `tick`.
    fn kept<T: Transport>(dmm: &mut Dmm<T>, tick: Duration, n: usize) -> Vec<(u32, Instant)> {
        let mut stream = MeasurementStream::new(dmm, tick);
        (0..n)
            .map(|_| match stream.tick().unwrap() {
                StreamEvent::Measurement(m) => {
                    let k = u32::from_le_bytes(m.raw_payload[..4].try_into().unwrap());
                    (k, m.timestamp)
                }
                other => panic!("expected a reading, got {other:?}"),
            })
            .collect()
    }

    fn spacings_ms(readings: &[(u32, Instant)]) -> Vec<u128> {
        readings
            .windows(2)
            .map(|w| (w[1].1 - w[0].1).as_millis())
            .collect()
    }

    #[test]
    fn a_zero_interval_keeps_every_frame_at_its_own_time() {
        let offsets = jittered(100, 20);
        let (mut dmm, clock) = timed_dmm(offsets.clone(), vec![]);
        let start = clock.now();
        let readings = kept(&mut dmm, Duration::ZERO, 20);
        for (k, (index, at)) in readings.iter().enumerate() {
            assert_eq!(*index as usize, k);
            assert_eq!(*at, start + offsets[k]);
        }
    }

    #[test]
    fn a_long_interval_keeps_the_frame_just_sent() {
        // A ZOTEK's 385 ms frames at a 2 s interval: each reading kept is
        // the frame nearest its tick, never one from further back.
        let (mut dmm, clock) = timed_dmm(jittered(385, 200), vec![]);
        let start = clock.now();
        let readings = kept(&mut dmm, Duration::from_secs(2), 30);
        for (k, (_, at)) in readings.iter().enumerate().skip(1) {
            let tick = start + Duration::from_secs(2 * k as u64);
            let off = if *at > tick { *at - tick } else { tick - *at };
            assert!(
                off <= Duration::from_millis(200),
                "reading {k} is {off:?} off its tick"
            );
        }
    }

    #[test]
    fn an_interval_a_multiple_of_the_frame_period_keeps_a_steady_spacing() {
        // 1 s over a 2 Hz meter whose frames wander: without the tolerance
        // the spacings jump between 0.5, 1 and 1.5 s.
        let (mut dmm, _clock) = timed_dmm(jittered(500, 200), vec![]);
        let readings = kept(&mut dmm, Duration::from_secs(1), 40);
        for gap in &spacings_ms(&readings)[4..] {
            assert!((990..=1010).contains(gap), "spacing {gap} ms");
        }
    }

    #[test]
    fn an_interval_equal_to_the_frame_period_drops_nothing() {
        let (mut dmm, _clock) = timed_dmm(jittered(385, 200), vec![]);
        let readings = kept(&mut dmm, Duration::from_millis(385), 60);
        for w in readings[4..].windows(2) {
            assert_eq!(w[1].0, w[0].0 + 1, "a frame was dropped after {}", w[0].0);
        }
    }

    #[test]
    fn any_other_interval_keeps_one_reading_per_interval_on_average() {
        let (mut dmm, _clock) = timed_dmm(jittered(385, 400), vec![]);
        let readings = kept(&mut dmm, Duration::from_secs(1), 100);
        let span = readings[99].1 - readings[0].1;
        let mean = span / 99;
        assert!(
            mean.abs_diff(Duration::from_secs(1)) < Duration::from_millis(10),
            "mean spacing {mean:?}"
        );
    }

    /// A new interval takes over at the next reading: kept at once, then
    /// one per new tick.
    #[test]
    fn a_new_tick_starts_its_schedule_at_the_next_reading() {
        let (mut dmm, _clock) = timed_dmm(jittered(100, 200), vec![]);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(10));
        let first = match stream.tick().unwrap() {
            StreamEvent::Measurement(m) => m.timestamp,
            other => panic!("{other:?}"),
        };
        stream.set_tick(Duration::from_millis(500));
        let mut stamps = vec![first];
        for _ in 0..4 {
            match stream.tick().unwrap() {
                StreamEvent::Measurement(m) => stamps.push(m.timestamp),
                other => panic!("{other:?}"),
            }
        }
        // The frame right after the first, not ten seconds on.
        assert!(stamps[1] - stamps[0] < Duration::from_millis(150));
        for w in stamps[1..].windows(2) {
            let gap = w[1] - w[0];
            assert!(
                gap.abs_diff(Duration::from_millis(500)) <= Duration::from_millis(10),
                "{gap:?}"
            );
        }
    }

    #[test]
    fn a_cold_start_still_keeps_one_reading_per_tick() {
        // Before three spacings are known there is no tolerance: the first
        // frame at or after each tick counts.
        let (mut dmm, _clock) = timed_dmm(jittered(100, 100), vec![]);
        let readings = kept(&mut dmm, Duration::from_millis(450), 4);
        for gap in spacings_ms(&readings) {
            assert!((350..=550).contains(&gap), "spacing {gap} ms");
        }
    }

    #[test]
    fn a_corrupt_frame_between_ticks_is_dropped() {
        let (mut dmm, _clock) = timed_dmm(jittered(100, 100), vec![3, 4, 12]);
        let readings = kept(&mut dmm, Duration::from_secs(1), 5);
        assert_eq!(readings.len(), 5);
    }

    /// A corrupt frame at the tick loses to a good one read before it in
    /// the same window.
    #[test]
    fn a_good_frame_before_a_corrupt_one_stands_for_the_tick() {
        // Frame 10 lands on the 1 s tick, corrupt; frame 9 came 100 ms before.
        let (mut dmm, _clock) = timed_dmm(jittered(100, 40), vec![10]);
        let readings = kept(&mut dmm, Duration::from_secs(1), 2);
        assert_eq!(readings[1].0, 9, "{readings:?}");
    }

    #[test]
    fn a_tick_with_only_corrupt_frames_reports_the_error() {
        // Frames 1 to 12 are all corrupt: nothing good comes between the
        // first reading and the tick at 1 s, nor just after it.
        let (mut dmm, _clock) = timed_dmm(jittered(100, 40), (1..=12).collect());
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(1));
        assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
        let err = stream.tick().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Protocol);
        assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
    }

    #[test]
    fn a_cancelled_wait_hands_over_the_newest_frame_read() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let (mut dmm, clock) = timed_dmm(jittered(100, 100), vec![]);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(60))
            .with_cancel(move || flag.load(Ordering::Relaxed));
        assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
        stop.store(true, Ordering::Relaxed);
        let start = clock.now();
        assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
        assert!(clock.now() - start < Duration::from_secs(1));
    }

    /// A Bluetooth link that hands readings over two at a time when it is
    /// slower than the meter, as the UT-D07B's does.
    struct SlowLink;

    impl Transport for SlowLink {
        fn write(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
        fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            Ok(0)
        }
        fn link(&self) -> Option<crate::transport::Link> {
            Some(crate::transport::Link::Bluetooth)
        }
        fn late_readings(&self) -> Option<LateReadings> {
            Some(LateReadings { command: None })
        }
    }

    /// The UT-D07B's meter: a frame every 310.15 ms, each off by up to ±5 ms
    /// (adapter spec §5).
    fn adapter_meter(frames: usize) -> Vec<Duration> {
        const WANDER_US: [i64; 7] = [4_000, -3_000, 1_000, -5_000, 2_000, 5_000, -2_000];
        (0..frames)
            .map(|k| {
                let us = k as i64 * 310_150 + WANDER_US[k % WANDER_US.len()];
                Duration::from_micros(us.max(0) as u64)
            })
            .collect()
    }

    /// Frames sent at `offsets`, each carried at the first connection event
    /// of a link at `interval` at or after it.
    fn on_events(offsets: Vec<Duration>, interval: Duration) -> Vec<Duration> {
        let step = interval.as_nanos();
        offsets
            .into_iter()
            .map(|o| Duration::from_nanos((o.as_nanos().div_ceil(step) * step) as u64))
            .collect()
    }

    /// How many notices a stream at `tick` hands out over `ticks` ticks.
    fn notices<T: Transport>(dmm: &mut Dmm<T>, tick: Duration, ticks: usize) -> usize {
        let mut stream = MeasurementStream::new(dmm, tick);
        let mut notices = 0;
        for _ in 0..ticks {
            assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
            notices += usize::from(stream.take_late_readings().is_some());
        }
        notices
    }

    #[test]
    fn a_link_slower_than_the_meter_gives_the_notice_once() {
        let frames = on_events(adapter_meter(400), Duration::from_millis(315));
        let (mut dmm, _clock) = timed_dmm_on(SlowLink, frames.clone(), vec![]);
        assert_eq!(notices(&mut dmm, Duration::ZERO, 400), 1);
        // At any interval: the stream sees every frame either way.
        let (mut dmm, _clock) = timed_dmm_on(SlowLink, frames.clone(), vec![]);
        assert_eq!(notices(&mut dmm, Duration::from_secs(1), 100), 1);
        // A link with nothing to say about it stays quiet.
        let (mut dmm, _clock) = timed_dmm(frames, vec![]);
        assert_eq!(notices(&mut dmm, Duration::ZERO, 400), 0);
    }

    #[test]
    fn a_short_interval_gives_no_notice() {
        let frames = on_events(adapter_meter(400), Duration::from_millis(45));
        let (mut dmm, _clock) = timed_dmm_on(SlowLink, frames, vec![]);
        assert_eq!(notices(&mut dmm, Duration::ZERO, 400), 0);
    }

    /// A backlog handed over at once after the host stalled is one burst,
    /// not a link that bunches.
    #[test]
    fn one_backlog_burst_gives_no_notice() {
        let mut frames = jittered(310, 100);
        for k in 51..54 {
            frames[k] = frames[50];
        }
        let (mut dmm, _clock) = timed_dmm_on(SlowLink, frames, vec![]);
        assert_eq!(notices(&mut dmm, Duration::ZERO, 100), 0);
    }

    #[test]
    fn two_pairs_give_no_notice() {
        let mut frames = jittered(310, 200);
        frames[60] = frames[59] + Duration::from_millis(1);
        frames[140] = frames[139] + Duration::from_millis(1);
        let (mut dmm, _clock) = timed_dmm_on(SlowLink, frames, vec![]);
        assert_eq!(notices(&mut dmm, Duration::ZERO, 200), 0);
    }
}
