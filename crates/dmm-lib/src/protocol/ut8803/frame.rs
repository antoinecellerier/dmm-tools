//! Finding UT8803 frames in the byte stream
//! (`docs/research/ut8803/reverse-engineered-protocol.md` §2.3).

use crate::error::{Error, Result};
use crate::protocol::framing::{HEADER, checksum_ok, locate, sum16};
use crate::protocol::unrecognised::report_unknown;
use log::trace;

/// Extract a frame using UT8803 format: AB CD byte2 0x02 payload chk_hi chk_lo.
///
/// Fixed 21-byte frame. Checksum is alternating-byte sum, stored BE at bytes 19-20.
///
/// Returns `Ok(Some((payload, consumed)))` where payload is bytes 2..19 (17 bytes),
/// `Ok(None)` if incomplete.
pub(crate) fn extract_frame_ut8803(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    const FRAME_LEN: usize = 21;

    let Some((start, remaining)) = locate(buf, &HEADER, FRAME_LEN) else {
        return Ok(None);
    };

    // Checksum: sum of bytes 0..19, stored BE at bytes 19-20.
    // The RE spec describes this as an "alternating-byte sum" (even/odd
    // accumulators), but that's equivalent to a straight sequential sum.
    let computed = sum16(&remaining[..19]);
    let received = u16::from_be_bytes([remaining[19], remaining[20]]);

    // Byte 3 must be 0x02 (measurement response type). Error out rather
    // than returning Ok(None): Ok(None) means "need more data" and never
    // consumes, so one non-measurement frame at the buffer head would
    // block extraction until the buffer cap clears everything. The
    // family's SkipAndRetry recovery drains past this header instead.
    if remaining[3] != 0x02 {
        // The spec documents no other type
        // (docs/research/ut8803/reverse-engineered-protocol.md §2.3). Only a
        // frame whose checksum holds is the meter's: a false AB CD while
        // syncing stays silent.
        if computed == received {
            report_unknown(
                "ut8803",
                "frame type",
                format_args!("{:#04x}", remaining[3]),
            );
        }
        trace!("framing: ut8803 byte3={:#04x}, expected 0x02", remaining[3]);
        return Err(Error::invalid_response(
            format!("ut8803 frame type {:#04x}, expected 0x02", remaining[3]),
            remaining,
        ));
    }

    let frame = &remaining[..FRAME_LEN];
    trace!("framing: ut8803 raw frame: {:02X?}", frame);
    checksum_ok("ut8803", computed, received, frame)?;

    // Payload = bytes 2..19 (everything between header and checksum)
    let payload = frame[2..19].to_vec();
    let consumed = start + FRAME_LEN;

    trace!("framing: ut8803 valid frame, consumed={consumed}");
    Ok(Some((payload, consumed)))
}

/// Build a valid 21-byte UT8803 frame; `body` becomes bytes 2..19, so
/// `body[1]` is the frame-type byte the extractor requires to be 0x02.
#[cfg(test)]
pub(crate) fn test_frame_ut8803(body: &[u8; 17]) -> Vec<u8> {
    let mut frame = vec![0xAB, 0xCD];
    frame.extend_from_slice(body);
    let sum = sum16(&frame);
    frame.push((sum >> 8) as u8);
    frame.push((sum & 0xFF) as u8);
    frame
}

/// A UT8803 body whose type byte is set; the rest is filler.
#[cfg(test)]
pub(crate) fn test_ut8803_body() -> [u8; 17] {
    let mut body = [0u8; 17];
    body[1] = 0x02; // frame type = measurement
    body[2] = 0x01; // mode
    body[3] = 0x31; // range
    body[6..11].copy_from_slice(b"12.34");
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut8803_non_measurement_frame_errors_for_resync() {
        // A frame with byte3 != 0x02 must produce Err (so SkipAndRetry
        // drains past it), not Ok(None) which would pin it at the buffer
        // head and stall extraction forever.
        let mut buf = vec![0xAB, 0xCD, 0x00, 0x05];
        buf.resize(21, 0x00);
        assert!(extract_frame_ut8803(&buf).is_err());
    }

    /// A checksummed frame of an undocumented type is reported and still
    /// errors; with a bad checksum (a false AB CD) it errors silently.
    #[test]
    fn ut8803_unknown_frame_type_is_reported_only_when_checksummed() {
        let mut body = test_ut8803_body();
        body[1] = 0x05;
        let mut frame = test_frame_ut8803(&body);
        let (result, reports) = crate::protocol::capture_reports(|| extract_frame_ut8803(&frame));
        assert!(result.is_err());
        assert_eq!(reports, ["ut8803: unrecognised frame type: 0x05"]);

        frame[20] ^= 0xFF;
        let (result, reports) = crate::protocol::capture_reports(|| extract_frame_ut8803(&frame));
        assert!(result.is_err());
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A measurement frame passes the extractor without a report.
    #[test]
    fn ut8803_measurement_frame_reports_nothing() {
        let frame = test_frame_ut8803(&test_ut8803_body());
        let (result, reports) = crate::protocol::capture_reports(|| extract_frame_ut8803(&frame));
        assert!(matches!(result, Ok(Some((_, 21)))));
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn ut8803_valid_frame() {
        // Construct a minimal valid 21-byte UT8803 frame
        let mut frame = vec![
            0xAB, 0xCD, // header
            0x00, // byte 2
            0x02, // type = measurement
            0x01, // mode
            0x31, // range (with 0x30 prefix)
            0x00, // padding
            b'1', b'2', b'.', b'3', b'4', // display (5 bytes)
            0x00, 0x00, // flags0
            0x00, 0x00, // flags1
            0x00, 0x00, // flags2
            0x00, // flags3
        ];
        // Compute checksum: sum of bytes 0..19
        let sum: u16 = frame.iter().map(|&b| b as u16).sum();
        frame.push((sum >> 8) as u8);
        frame.push((sum & 0xFF) as u8);
        assert_eq!(frame.len(), 21);

        let (payload, consumed) = extract_frame_ut8803(&frame).unwrap().unwrap();
        assert_eq!(consumed, 21);
        assert_eq!(payload.len(), 17); // bytes 2..19
    }

    #[test]
    fn ut8803_incomplete() {
        let buf = vec![0xAB, 0xCD, 0x00, 0x02, 0x01]; // too short
        assert!(extract_frame_ut8803(&buf).unwrap().is_none());
    }

    /// 20 of the 21 bytes: still incomplete, and `consumed` stays 0.
    #[test]
    fn ut8803_one_byte_short_is_incomplete() {
        let frame = test_frame_ut8803(&test_ut8803_body());
        let truncated = &frame[..frame.len() - 1];
        assert!(extract_frame_ut8803(truncated).unwrap().is_none());
    }

    /// The UT8803 header can arrive mid-stream; `consumed` has to cover the
    /// bytes before it, or the read loop re-scans them forever.
    #[test]
    fn ut8803_leading_garbage() {
        let frame = test_frame_ut8803(&test_ut8803_body());
        let mut buf = vec![0xFF, 0xFE, 0xFD];
        buf.extend_from_slice(&frame);
        let (payload, consumed) = extract_frame_ut8803(&buf).unwrap().unwrap();
        assert_eq!(consumed, 3 + frame.len());
        assert_eq!(payload, frame[2..19].to_vec());
    }

    /// As framing's `extract_bad_checksum`, for the UT8803's own sum.
    #[test]
    fn ut8803_bad_checksum() {
        let mut frame = test_frame_ut8803(&test_ut8803_body());
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        assert!(matches!(
            extract_frame_ut8803(&frame),
            Err(Error::ChecksumMismatch {
                expected: 603,
                actual: 676,
            })
        ));
    }
}
