//! Wire-byte recorder wrapped around a transport for `dmm-cli capture`.
//!
//! Records what actually crossed the USB link, including bytes the framing
//! layer rejected, so a step that decoded nothing still carries evidence.

use dmm_lib::error::Result;
use dmm_lib::transport::Transport;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// Bound on buffered wire events. A capture drains after every step, so this
/// only has to cover one step; oldest events are dropped and counted.
pub(crate) const MAX_WIRE_EVENTS: usize = 4096;

/// How long a stream of received bytes stays one wire event. The CP2110
/// hands up one UART byte per HID report, so a 19-byte frame arrives as 19
/// reads and would otherwise flood the buffer and every per-step cap.
pub(crate) const RX_COALESCE_GAP_MS: u64 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Direction {
    Tx,
    Rx,
}

/// One transfer over the wire, tagged with the capture step it belongs to.
pub(crate) struct WireEvent {
    pub at_ms: u64,
    pub dir: Direction,
    pub step: Option<String>,
    pub bytes: Vec<u8>,
    /// HID feature report rather than an interrupt write.
    pub feature: bool,
}

/// Shared with the transport wrapper, which records from wherever it runs.
pub(crate) type SharedRecorder = Arc<Mutex<Recorder>>;

pub(crate) struct Recorder {
    start: Instant,
    events: VecDeque<WireEvent>,
    dropped: u64,
    current_step: Option<String>,
    /// When the last Rx byte arrived, for the coalescing gap.
    last_rx_ms: Option<u64>,
}

impl Recorder {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            events: VecDeque::new(),
            dropped: 0,
            current_step: None,
            last_rx_ms: None,
        }
    }

    /// Tag subsequent events with the step being captured (`None` for the
    /// init handshake and anything between steps).
    pub(crate) fn set_step(&mut self, id: Option<&str>) {
        self.current_step = id.map(str::to_string);
    }

    pub(crate) fn drain(&mut self) -> Vec<WireEvent> {
        self.events.drain(..).collect()
    }

    /// Events dropped because the buffer was full, over the whole session.
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped
    }

    fn push(&mut self, dir: Direction, feature: bool, bytes: &[u8]) {
        // `checked_duration_since`: a backward clock jump must not panic
        // mid-capture.
        let at_ms = Instant::now()
            .checked_duration_since(self.start)
            .unwrap_or_default()
            .as_millis() as u64;
        self.push_at(at_ms, dir, feature, bytes);
    }

    /// Consecutive reads on the same step within [`RX_COALESCE_GAP_MS`] are
    /// appended to the previous event rather than filed as new ones. Anything
    /// else — a write in between, a step change, a longer gap — splits.
    fn push_at(&mut self, at_ms: u64, dir: Direction, feature: bool, bytes: &[u8]) {
        if dir == Direction::Rx && !feature {
            let within_gap = self
                .last_rx_ms
                .is_some_and(|last| at_ms.saturating_sub(last) <= RX_COALESCE_GAP_MS);
            self.last_rx_ms = Some(at_ms);
            if within_gap
                && let Some(prev) = self.events.back_mut()
                && prev.dir == Direction::Rx
                && !prev.feature
                && prev.step == self.current_step
            {
                prev.bytes.extend_from_slice(bytes);
                return;
            }
        }
        if self.events.len() >= MAX_WIRE_EVENTS {
            self.events.pop_front();
            self.dropped += 1;
        }
        self.events.push_back(WireEvent {
            at_ms,
            dir,
            step: self.current_step.clone(),
            bytes: bytes.to_vec(),
            feature,
        });
    }
}

/// Lock the recorder without panicking on a poisoned mutex — a capture that
/// lost a thread should still write out the events it has.
pub(crate) fn lock(recorder: &SharedRecorder) -> MutexGuard<'_, Recorder> {
    recorder.lock().unwrap_or_else(|e| e.into_inner())
}

/// Transport wrapper that records every byte it passes through.
pub(crate) struct RecordingTransport {
    inner: Box<dyn Transport>,
    recorder: SharedRecorder,
}

impl RecordingTransport {
    /// Returns the wrapper and the recorder it feeds.
    pub(crate) fn new(inner: Box<dyn Transport>) -> (Self, SharedRecorder) {
        let recorder = Arc::new(Mutex::new(Recorder::new()));
        (
            Self {
                inner,
                recorder: Arc::clone(&recorder),
            },
            recorder,
        )
    }
}

impl Transport for RecordingTransport {
    fn write(&self, data: &[u8]) -> Result<()> {
        lock(&self.recorder).push(Direction::Tx, false, data);
        self.inner.write(data)
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        let n = self.inner.read_timeout(buf, timeout_ms)?;
        if n > 0 {
            lock(&self.recorder).push(Direction::Rx, false, &buf[..n]);
        }
        Ok(n)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        lock(&self.recorder).push(Direction::Tx, true, data);
        self.inner.send_feature_report(data)
    }

    fn transport_info(&self) -> Result<String> {
        self.inner.transport_info()
    }

    fn transport_status(&self) -> Result<String> {
        self.inner.transport_status()
    }

    fn transport_name(&self) -> &'static str {
        self.inner.transport_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canned transport: echoes a fixed response on every read.
    struct FakeTransport {
        response: Vec<u8>,
    }

    impl Transport for FakeTransport {
        fn write(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let n = self.response.len().min(buf.len());
            buf[..n].copy_from_slice(&self.response[..n]);
            Ok(n)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    fn recording(response: Vec<u8>) -> (RecordingTransport, SharedRecorder) {
        RecordingTransport::new(Box::new(FakeTransport { response }))
    }

    #[test]
    fn records_both_directions_with_the_active_step() {
        let (t, rec) = recording(vec![0xAB, 0xCD]);
        lock(&rec).set_step(Some("dcv"));
        t.write(&[0x01]).unwrap();
        let mut buf = [0u8; 8];
        t.read_timeout(&mut buf, 100).unwrap();
        lock(&rec).set_step(None);
        t.write(&[0x02]).unwrap();

        let events = lock(&rec).drain();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].dir, Direction::Tx);
        assert_eq!(events[0].step.as_deref(), Some("dcv"));
        assert_eq!(events[1].dir, Direction::Rx);
        assert_eq!(events[1].bytes, vec![0xAB, 0xCD]);
        assert_eq!(events[1].step.as_deref(), Some("dcv"));
        assert_eq!(events[2].step, None);
        assert!(lock(&rec).drain().is_empty(), "drain must empty the buffer");
    }

    /// A silent read is not a wire event — recording it would bury the real
    /// traffic under one entry per poll.
    #[test]
    fn empty_reads_are_not_recorded() {
        let (t, rec) = recording(vec![]);
        let mut buf = [0u8; 8];
        assert_eq!(t.read_timeout(&mut buf, 100).unwrap(), 0);
        assert!(lock(&rec).drain().is_empty());
    }

    #[test]
    fn feature_reports_are_marked() {
        let (t, rec) = recording(vec![]);
        t.send_feature_report(&[0x41, 0x01]).unwrap();
        let events = lock(&rec).drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].dir, Direction::Tx);
        assert!(events[0].feature);
    }

    /// The CP2110 delivers one UART byte per HID report, so a frame arrives
    /// as one read per byte; the trace must show the frame, not the reports.
    #[test]
    fn consecutive_reads_within_the_gap_are_one_event() {
        let mut r = Recorder::new();
        for (i, b) in [0xAB, 0xCD, 0x10].iter().enumerate() {
            r.push_at(i as u64 * 10, Direction::Rx, false, &[*b]);
        }
        let events = r.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].bytes, vec![0xAB, 0xCD, 0x10]);
        assert_eq!(events[0].at_ms, 0, "the event keeps its first byte's time");
    }

    #[test]
    fn a_write_between_reads_splits_the_event() {
        let mut r = Recorder::new();
        r.push_at(0, Direction::Rx, false, &[0xAB]);
        r.push_at(1, Direction::Tx, false, &[0x01]);
        r.push_at(2, Direction::Rx, false, &[0xCD]);
        let events = r.drain();
        assert_eq!(events.len(), 3);
        assert_eq!(events[2].bytes, vec![0xCD]);
    }

    #[test]
    fn a_gap_over_the_limit_splits_the_event() {
        let mut r = Recorder::new();
        r.push_at(0, Direction::Rx, false, &[0xAB]);
        r.push_at(RX_COALESCE_GAP_MS, Direction::Rx, false, &[0xCD]);
        r.push_at(RX_COALESCE_GAP_MS * 2 + 1, Direction::Rx, false, &[0xEF]);
        let events = r.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].bytes, vec![0xAB, 0xCD]);
        assert_eq!(events[1].bytes, vec![0xEF]);
    }

    #[test]
    fn a_step_change_splits_the_event() {
        let mut r = Recorder::new();
        r.set_step(Some("dcv"));
        r.push_at(0, Direction::Rx, false, &[0xAB]);
        r.set_step(Some("acv"));
        r.push_at(1, Direction::Rx, false, &[0xCD]);
        let events = r.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].step.as_deref(), Some("acv"));
    }

    #[test]
    fn the_buffer_is_bounded_and_drops_are_counted() {
        let (t, rec) = recording(vec![]);
        for _ in 0..MAX_WIRE_EVENTS + 5 {
            t.write(&[0x01]).unwrap();
        }
        assert_eq!(lock(&rec).dropped(), 5);
        assert_eq!(lock(&rec).drain().len(), MAX_WIRE_EVENTS);
    }
}
