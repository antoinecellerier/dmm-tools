//! Asking for a reading and finding the reply
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §3, §4).
//!
//! One request, `00 cc 66`, draws one reply: three 8-byte reports, 24 data
//! bytes, with no checksum (spec §3.1, §4.1). The series code in the model
//! bytes, 20-23, is the only thing that tells a reply from anything else,
//! and it sits in the last report, so a reply is looked for at the end of
//! what has arrived, after each report. Another series' code there is a
//! meter the user named wrongly, which the error says (spec §4.2).

use super::map::MODEL;
use super::{Series, devices};
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

/// The receive buffer's bound: two replies.
const MAX_BUF: usize = 2 * REPLY_LEN;

/// The model bytes' first index: bytes 20-23 (spec §4.2).
pub(super) const MODEL_RUN: usize = MODEL - 3;

/// The last 24 bytes of `buf`, where a reply ends once its third report has
/// arrived.
fn last_reply(buf: &[u8]) -> Option<[u8; REPLY_LEN]> {
    let start = buf.len().checked_sub(REPLY_LEN)?;
    buf[start..].try_into().ok()
}

/// Whether `reply`'s model bytes name `series`.
///
/// The BM860 sheet names byte 23 alone, bytes 20-22 being "don't care", so
/// only byte 23 is required of a BM86x; the BM820 and BM520-ML sheets name
/// all four (spec §4.2).
fn names(reply: &[u8; REPLY_LEN], series: Series) -> bool {
    match series {
        Series::Bm86x => reply[MODEL] == series.code(),
        Series::Bm82x | Series::Bm52x => {
            reply[MODEL_RUN..=MODEL].iter().all(|b| *b == series.code())
        }
    }
}

/// The reply `buf` ends in, if it ends in one of `series`.
pub(super) fn find_reply(buf: &[u8], series: Series) -> Option<[u8; REPLY_LEN]> {
    let reply = last_reply(buf)?;
    names(&reply, series).then_some(reply)
}

/// The other series whose model bytes `buf` ends in: a meter of that
/// series is on the cable, answering the request of this one.
fn other_series(buf: &[u8], series: Series) -> Option<Series> {
    let reply = last_reply(buf)?;
    Series::ALL
        .into_iter()
        .find(|other| *other != series && names(&reply, *other))
}

/// The error for a reply of `other` read as `series`: it names the entry
/// to use.
fn wrong_series(series: Series, other: Series, reply: &[u8]) -> Error {
    Error::invalid_response(
        format!(
            "the meter on the cable is a {}, not a {}; choose Auto-detect or --device {}",
            devices::entry(other).display_name,
            devices::entry(series).display_name,
            other.id()
        ),
        reply,
    )
}

/// Where four model bytes of `series` stand anywhere in `buf`: what
/// detection takes as a reply, its buffer spanning several windows.
pub(super) fn model_run_at(buf: &[u8], series: Series) -> Option<usize> {
    let run = [series.code(); 4];
    buf.windows(run.len()).position(|w| w == run)
}

/// Ask `series`' meter for a reading and read its reply.
pub(super) fn read(transport: &dyn Transport, series: Series) -> Result<[u8; REPLY_LEN]> {
    // A late reply to an earlier request would otherwise be taken for
    // this one's.
    framing::discard_queued(transport)?;
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
        if let Some(other) = other_series(&buf, series) {
            return Err(wrong_series(series, other, &buf[buf.len() - REPLY_LEN..]));
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

    /// The BM820 sheet's example (spec §9.2) with digit 5 in byte 11, where
    /// BM820 Table 1 puts it, byte 10 being report II's ID (spec §4.3): the
    /// 24 data bytes, the "don't care" bytes `00`, the model bytes `code`.
    pub(crate) fn bm820_example(code: u8) -> Vec<u8> {
        let printed = [
            0x00, 0x00, 0x10, 0x00, 0xE9, 0xEF, 0xBF, 0xA0, 0x00, 0x6D, 0x00, 0xBF, 0xA0, 0xCB,
            0x00, 0x01, 0x20, 0x00, 0x00, code, code, code, code, 0x10, 0x00, 0x00, 0x00,
        ];
        let mut table_1 = printed;
        table_1.swap(9, 10);
        table_1
            .iter()
            .enumerate()
            .filter(|(i, _)| ![0, 9, 18].contains(i))
            .map(|(_, b)| *b)
            .collect()
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
        fn link(&self) -> Option<crate::transport::Link> {
            None
        }

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
        assert_eq!(Series::Bm82x.code(), 0x82);
        assert_eq!(Series::Bm52x.code(), 0x52);
    }

    /// The BM820 and BM520-ML sheets name all four model bytes (spec §4.2):
    /// byte 23 alone is not a reply of theirs.
    #[test]
    fn a_bm82x_or_bm52x_reply_needs_four_model_bytes() {
        for series in [Series::Bm82x, Series::Bm52x] {
            let reply = bm820_example(series.code());
            assert_eq!(find_reply(&reply, series), reply.clone().try_into().ok());
            for i in 16..19 {
                let mut short = reply.clone();
                short[i] = 0x00;
                assert_eq!(find_reply(&short, series), None, "{series:?} {i}");
            }
        }
    }

    /// A reply naming another series fails at once, naming the entry to
    /// use, instead of waiting out the deadline.
    #[test]
    fn another_series_reply_names_its_entry() {
        for (series, other) in [
            (Series::Bm86x, Series::Bm82x),
            (Series::Bm82x, Series::Bm52x),
            (Series::Bm52x, Series::Bm86x),
            (Series::Bm52x, Series::Bm82x),
        ] {
            let mut reply = bm820_example(other.code());
            reply[MODEL_RUN..=MODEL].fill(other.code());
            let reports = reply.chunks(8).map(<[u8]>::to_vec).collect();
            let cable = ScriptedCable::new(request(series.code()), reports, Vec::new());
            match read(&cable, series) {
                Err(Error::InvalidResponse { message, raw }) => {
                    assert!(
                        message.contains(&format!("--device {}", other.id())),
                        "{message}"
                    );
                    assert!(
                        message.contains(devices::entry(other).display_name),
                        "{message}"
                    );
                    assert_eq!(raw, reply);
                }
                other => panic!("{series:?}: {other:?}"),
            }
        }
        // A BM86x is named by byte 23 alone here too.
        let mut loose: Vec<u8> = example_reports().concat();
        loose[MODEL_RUN..MODEL].fill(0x00);
        let reports = loose.chunks(8).map(<[u8]>::to_vec).collect();
        let cable = ScriptedCable::new(request(0x82), reports, Vec::new());
        match read(&cable, Series::Bm82x) {
            Err(Error::InvalidResponse { message, .. }) => {
                assert!(message.contains("--device bm86x"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        // Three bytes of another series are no reply of it: the read waits.
        let mut three = bm820_example(0x82);
        three[16] = 0x00;
        let reports = three.chunks(8).map(<[u8]>::to_vec).collect();
        let cable = ScriptedCable::new(request(0x86), reports, Vec::new());
        assert!(matches!(read(&cable, Series::Bm86x), Err(Error::Timeout)));
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
