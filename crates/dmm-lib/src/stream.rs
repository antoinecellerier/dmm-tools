//! Paced measurement stream.
//!
//! Wraps a [`Dmm`] with absolute-tick pacing and consecutive-timeout counting,
//! so the CLI `read`/`debug` loop and the GUI background thread can share the
//! same acquisition logic.
//!
//! The stream intentionally does not own cancellation — the CLI uses an
//! `AtomicBool` driven by the Ctrl-C handler while the GUI uses an `mpsc`
//! stop channel, and neither fits naturally inside the other. Callers check
//! their own stop signal around each [`MeasurementStream::tick`] call, and
//! can hand the stream a predicate via [`MeasurementStream::with_cancel`] so
//! the pacing sleep gives up on that same signal instead of running to term.

use crate::Dmm;
use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::transport::Transport;
use std::time::{Duration, Instant};

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
}

/// Paced acquisition wrapper around a [`Dmm`].
///
/// Construct with [`MeasurementStream::new`] and drive by calling
/// [`tick`](Self::tick) repeatedly. Each call sleeps until the next
/// scheduled tick boundary (if any) and then requests one measurement.
///
/// The stream borrows the `Dmm` mutably so callers can keep control over
/// ownership (e.g. to call `send_command` between ticks). Absolute-tick
/// pacing means the Nth tick lands at `start + N*tick` regardless of how
/// long the previous request took, so measurement cadence does not drift
/// when `request_measurement` is occasionally slow.
pub struct MeasurementStream<'a, T: Transport> {
    dmm: &'a mut Dmm<T>,
    tick: Duration,
    next_tick: Option<Instant>,
    consecutive_timeouts: u32,
    /// `'static` rather than `'a`: a borrowed predicate would make the struct
    /// invariant in `'a`, which stops callers shortening the `&mut Dmm` borrow
    /// (the GUI reassigns `dmm` after a reconnect). Both callers hand over an
    /// owned `Arc` anyway.
    cancel: Option<Box<dyn Fn() -> bool + Send + 'static>>,
}

/// Longest single `thread::sleep` inside a cancellable pacing wait.
///
/// The wait is split into slices so the cancel predicate is polled about this
/// often. Small enough that shutdown feels immediate, large enough that a slow
/// sample interval doesn't spin.
const CANCEL_POLL_SLICE: Duration = Duration::from_millis(50);

impl<'a, T: Transport> MeasurementStream<'a, T> {
    /// Build a stream around `dmm` targeting one measurement per `tick`.
    /// A zero tick disables pacing — requests fire as fast as the protocol
    /// allows, useful for `count`-limited bulk reads.
    pub fn new(dmm: &'a mut Dmm<T>, tick: Duration) -> Self {
        Self {
            dmm,
            tick,
            next_tick: None,
            consecutive_timeouts: 0,
            cancel: None,
        }
    }

    /// Poll `cancel` during the pacing sleep and cut it short when it returns
    /// true.
    ///
    /// Without this the sleep runs for the whole sample interval no matter
    /// what: at a 2 s interval a shutdown request isn't noticed until the tick
    /// elapses (plus the read timeout that follows), so the caller keeps the
    /// device open long after the user asked it to stop. A hand-edited
    /// interval of minutes wedges it entirely.
    ///
    /// The predicate only shortens the wait — it does not cancel the
    /// measurement request that follows, so `tick` still returns an event and
    /// the caller decides what to do next.
    pub fn with_cancel(mut self, cancel: impl Fn() -> bool + Send + 'static) -> Self {
        self.cancel = Some(Box::new(cancel));
        self
    }

    /// Read one measurement, pacing to the tick schedule first.
    ///
    /// Returns `Ok(Measurement)` or `Ok(Timeout)`. Non-timeout transport
    /// errors bubble up as `Err(_)` and leave the counter unchanged; the
    /// caller decides whether to reconnect or abort.
    ///
    /// Named `tick` (not `next`) to avoid colliding with [`Iterator::next`],
    /// which this type deliberately does not implement — iterators can't
    /// return errors without the caller explicitly handling the `Result`.
    pub fn tick(&mut self) -> Result<StreamEvent> {
        self.sleep_until_tick();
        match self.dmm.request_measurement() {
            Ok(m) => {
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

    /// Number of consecutive timeouts since the last successful measurement.
    pub fn consecutive_timeouts(&self) -> u32 {
        self.consecutive_timeouts
    }

    /// Mutable access to the underlying `Dmm`. Useful for sending commands
    /// or reading transport info between ticks.
    pub fn dmm_mut(&mut self) -> &mut Dmm<T> {
        self.dmm
    }

    fn sleep_until_tick(&mut self) {
        if self.tick.is_zero() {
            return;
        }
        let now = Instant::now();
        match self.next_tick {
            Some(target) => {
                if let Some(wait) = target.checked_duration_since(now) {
                    self.sleep_cancellable(wait);
                }
                let mut next = target + self.tick;
                let now2 = Instant::now();
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
    /// With no predicate this is a plain `thread::sleep`; the slicing costs
    /// nothing when nobody is watching for cancellation.
    fn sleep_cancellable(&self, wait: Duration) {
        let Some(cancel) = &self.cancel else {
            std::thread::sleep(wait);
            return;
        };
        let deadline = Instant::now() + wait;
        loop {
            if cancel() {
                return;
            }
            // `checked_duration_since` rather than a subtraction: a backward
            // clock jump must not panic here.
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return;
            };
            std::thread::sleep(remaining.min(CANCEL_POLL_SLICE));
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
        let len_byte = (payload.len() + 2) as u8;
        let mut frame = vec![0xAB, 0xCD, len_byte];
        frame.extend_from_slice(&payload);
        let sum: u16 = frame.iter().map(|&b| b as u16).sum();
        frame.push((sum >> 8) as u8);
        frame.push((sum & 0xFF) as u8);
        frame
    }

    fn new_dmm(responses: Vec<Vec<u8>>) -> Dmm<MockTransport> {
        let mock = MockTransport::new(responses);
        let protocol = Box::new(Ut61PlusProtocol::new());
        Dmm::new(mock, protocol).unwrap()
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

        // The meter answers again on the same stream.
        stream
            .dmm
            .transport()
            .push_response(build_response(b"  3.000"));
        let _ = stream.tick();
        assert_eq!(stream.consecutive_timeouts(), 0);
    }

    #[test]
    fn pacing_sleeps_between_ticks() {
        let mut dmm = new_dmm(vec![build_response(b"  1.000"), build_response(b"  2.000")]);
        let tick = Duration::from_millis(50);
        let mut stream = MeasurementStream::new(&mut dmm, tick);

        let start = Instant::now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        let elapsed = start.elapsed();
        // Second tick should land ~50ms after the first.
        assert!(elapsed >= tick, "expected >= {tick:?}, got {elapsed:?}");
    }

    /// A long sample interval used to hold the device open for the whole tick
    /// after the user asked to stop.
    #[test]
    fn cancel_cuts_the_pacing_sleep_short() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut dmm = new_dmm(vec![build_response(b"  1.000"), build_response(b"  2.000")]);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(10))
            .with_cancel(move || flag.load(Ordering::Relaxed));

        let _ = stream.tick().unwrap(); // first tick fires immediately
        stop.store(true, Ordering::Relaxed);

        let start = Instant::now();
        let _ = stream.tick().unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "cancelled sleep should return promptly, took {:?}",
            start.elapsed()
        );
    }

    /// The predicate must not shorten a wait nobody asked to cancel.
    #[test]
    fn cancel_predicate_that_stays_false_still_paces() {
        let mut dmm = new_dmm(vec![build_response(b"  1.000"), build_response(b"  2.000")]);
        let tick = Duration::from_millis(120);
        let mut stream = MeasurementStream::new(&mut dmm, tick).with_cancel(|| false);

        let start = Instant::now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        assert!(
            start.elapsed() >= tick,
            "expected >= {tick:?}, got {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn zero_tick_disables_pacing() {
        let mut dmm = new_dmm(vec![build_response(b"  1.000"), build_response(b"  2.000")]);
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);

        let start = Instant::now();
        let _ = stream.tick().unwrap();
        let _ = stream.tick().unwrap();
        // No sleep: should be fast.
        assert!(start.elapsed() < Duration::from_millis(50));
    }
}
