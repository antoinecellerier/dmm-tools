//! Asking for a reading and finding the reply
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §3, §4).
//!
//! One request, `00 cc 66`, draws one reply: three 8-byte reports, 24 data
//! bytes, with no checksum (spec §3.1, §4.1). The series code in byte 23 is
//! the only thing that tells a reply from anything else, and it sits in the
//! last report, so a reply is looked for at the end of what has arrived,
//! after each report.

use super::Series;
use super::map::MODEL;
use crate::error::{Error, Result};
use crate::protocol::framing;
use crate::transport::Transport;
use log::debug;
use std::time::{Duration, Instant};

/// A reply's data bytes (spec §4.1).
pub(super) const REPLY_LEN: usize = 24;

/// The real-time request for `code`: "Command 1" `00`, the series code, and
/// "Command 3" `66` (spec §3.1). The cable adds the report ID.
pub(super) const fn request(code: u8) -> [u8; 3] {
    [0x00, code, 0x66]
}

/// How long a reply may take: Brymen's programs wait 4 s for the first
/// report (spec §3.3).
const REPLY_WAIT: Duration = Duration::from_millis(4000);

/// The most reports a drain drops before a request.
const MAX_DRAIN: usize = 16;

/// The receive buffer's bound: two replies.
const MAX_BUF: usize = 2 * REPLY_LEN;

/// The model bytes' first index: bytes 20-23 (spec §4.2).
const MODEL_RUN: usize = MODEL - 3;

/// The reply `buf` ends in, if it ends in one of `series`.
///
/// The BM860 sheet names byte 23 alone, bytes 20-22 being "don't care"
/// (spec §4.2), so only byte 23 is required.
pub(super) fn find_reply(buf: &[u8], series: Series) -> Option<[u8; REPLY_LEN]> {
    let start = buf.len().checked_sub(REPLY_LEN)?;
    let reply: [u8; REPLY_LEN] = buf[start..].try_into().ok()?;
    (reply[MODEL] == series.code()).then_some(reply)
}

/// Where four model bytes of `series` stand anywhere in `buf`: what
/// detection takes as a reply, its buffer spanning several windows.
pub(super) fn model_run_at(buf: &[u8], series: Series) -> Option<usize> {
    let run = [series.code(); 4];
    buf.windows(run.len()).position(|w| w == run)
}

/// Drop what arrived before the request: a late reply to an earlier one
/// would otherwise be taken for this one's.
fn drain(transport: &dyn Transport) -> Result<()> {
    let mut chunk = [0u8; 64];
    for _ in 0..MAX_DRAIN {
        let n = transport.read_timeout(&mut chunk, 0)?;
        if n == 0 {
            break;
        }
        debug!("bm86x: dropped a stale report {:02X?}", &chunk[..n]);
    }
    Ok(())
}

/// Ask `series`' meter for a reading and read its reply.
pub(super) fn read(transport: &dyn Transport, series: Series) -> Result<[u8; REPLY_LEN]> {
    drain(transport)?;
    transport.write(&request(series.code()))?;
    let deadline = Instant::now() + REPLY_WAIT;
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_BUF + 64);
    let mut chunk = [0u8; 64];
    loop {
        let n = framing::read_uart_bytes(transport, &mut chunk, deadline)?;
        if n == 0 {
            debug!("bm86x: no reply in {REPLY_WAIT:?}; received {buf:02X?}");
            return Err(Error::Timeout);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(reply) = find_reply(&buf, series) {
            let run = &reply[MODEL_RUN..MODEL];
            if run.iter().any(|b| *b != series.code()) {
                debug!("bm86x: model bytes 20-22 read {run:02X?}");
            }
            return Ok(reply);
        }
        if buf.len() > MAX_BUF {
            buf.drain(..buf.len() - MAX_BUF);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// The sheet's example (spec §9.1) as the cable delivers it: three
    /// 8-byte reports, the "don't care" bytes `00` and 20-22 `86`, as
    /// community captures show them (spec §13.3).
    pub(crate) fn example_reports() -> Vec<Vec<u8>> {
        vec![
            vec![0x00, 0x01, 0x11, 0xF8, 0xA0, 0xDA, 0xA9, 0xA0],
            vec![0x00, 0x00, 0x7E, 0xBF, 0xA0, 0xA0, 0x04, 0x00],
            vec![0x86, 0x86, 0x86, 0x86, 0x00, 0x00, 0x00, 0x00],
        ]
    }

    /// A cable that answers a request with its reports: `stale` wait in it
    /// before any request, and each write of `request` queues `replies`.
    pub(crate) struct ScriptedCable {
        request: [u8; 3],
        replies: Vec<Vec<u8>>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        pub(crate) written: RefCell<Vec<Vec<u8>>>,
    }

    impl ScriptedCable {
        pub(crate) fn new(request: [u8; 3], replies: Vec<Vec<u8>>, stale: Vec<Vec<u8>>) -> Self {
            Self {
                request,
                replies,
                queued: RefCell::new(stale.into()),
                written: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for ScriptedCable {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.written.borrow_mut().push(data.to_vec());
            if data == self.request {
                self.queued
                    .borrow_mut()
                    .extend(self.replies.iter().cloned());
            }
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(report) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = report.len().min(buf.len());
            buf[..len].copy_from_slice(&report[..len]);
            Ok(len)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    fn example_reply() -> Vec<u8> {
        example_reports().concat()
    }

    fn bm86x_cable(stale: Vec<Vec<u8>>) -> ScriptedCable {
        ScriptedCable::new(request(0x86), example_reports(), stale)
    }

    #[test]
    fn the_request_names_the_series() {
        assert_eq!(request(0x86), [0x00, 0x86, 0x66]);
        assert_eq!(Series::Bm86x.code(), 0x86);
    }

    #[test]
    fn a_reply_is_read_at_report_three() {
        let cable = bm86x_cable(Vec::new());
        assert_eq!(
            read(&cable, Series::Bm86x).unwrap().to_vec(),
            example_reply()
        );
        assert_eq!(cable.written.borrow().as_slice(), [request(0x86).to_vec()]);
    }

    /// What was waiting before the request is dropped, even a whole reply.
    #[test]
    fn stale_reports_are_drained() {
        let mut stale = example_reports();
        stale[0][3] = 0x00;
        stale.insert(0, vec![0x55; 8]);
        let cable = bm86x_cable(stale);
        assert_eq!(
            read(&cable, Series::Bm86x).unwrap().to_vec(),
            example_reply()
        );
    }

    /// Bytes 20-22 are "don't care" on the BM860 sheet (spec §4.2).
    #[test]
    fn a_bm86x_reply_takes_any_bytes_20_to_22() {
        for run in [[0x00; 3], [0xFF; 3], [0x86, 0x00, 0x42]] {
            let mut reply = example_reply();
            reply[16..19].copy_from_slice(&run);
            assert_eq!(
                find_reply(&reply, Series::Bm86x),
                reply.clone().try_into().ok()
            );
        }
        let mut other = example_reply();
        other[MODEL] = 0x82;
        assert_eq!(find_reply(&other, Series::Bm86x), None);
    }

    /// Two reports of three, or nothing at all, time out.
    #[test]
    fn a_short_reply_times_out() {
        let short = ScriptedCable::new(request(0x86), example_reports()[..2].to_vec(), Vec::new());
        assert!(matches!(read(&short, Series::Bm86x), Err(Error::Timeout)));
        let silent = ScriptedCable::new(request(0x86), Vec::new(), Vec::new());
        assert!(matches!(read(&silent, Series::Bm86x), Err(Error::Timeout)));
    }

    /// Reports that never end in a reply keep the buffer at two replies'
    /// worth, and a reply after them is still found.
    #[test]
    fn the_buffer_stays_bounded() {
        let mut reports = vec![vec![0x11; 8]; 40];
        reports.extend(example_reports());
        let cable = ScriptedCable::new(request(0x86), reports, Vec::new());
        assert_eq!(
            read(&cable, Series::Bm86x).unwrap().to_vec(),
            example_reply()
        );
    }

    /// A report split in two, or two joined, still ends where the reply
    /// ends.
    #[test]
    fn report_boundaries_do_not_matter() {
        let reply = example_reply();
        let pieces = vec![
            reply[..5].to_vec(),
            reply[5..20].to_vec(),
            reply[20..].to_vec(),
        ];
        let cable = ScriptedCable::new(request(0x86), pieces, Vec::new());
        assert_eq!(read(&cable, Series::Bm86x).unwrap().to_vec(), reply);
    }

    #[test]
    fn a_model_run_is_found_anywhere() {
        let mut buf = vec![0x42; 5];
        buf.extend(example_reply());
        assert_eq!(model_run_at(&buf, Series::Bm86x), Some(5 + 16));
        let mut three = example_reply();
        three[16] = 0x00;
        assert_eq!(model_run_at(&three, Series::Bm86x), None);
    }
}
