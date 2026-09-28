//! Shared framing: the read loop every framed family uses, and the `AB CD`
//! extractors and helpers more than one family shares. A frame shape only
//! one family sends lives in that family.

use crate::error::{Error, Result};
use crate::transport::Transport;
use log::{debug, trace};
use std::time::{Duration, Instant};

/// How to handle frame extraction errors (checksum mismatches).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameErrorRecovery {
    /// Propagate the error immediately (UT61E+ behavior).
    Propagate,
    /// Skip past the current ABCD header and retry (UT8803/UT171/UT181A behavior).
    SkipAndRetry,
}

/// Reads that hand up bytes but no frame before [`read_frame`] gives up,
/// counted again from each frame examined. A little over two of the longest
/// frames read here at a byte per report, as a CP2110 hands them up — a
/// VC-890's live frame is 66 bytes, the longest UT181A reading seen 63 — so
/// a frame joined just after its start still completes. A stream that never
/// frames (a wrong baud rate, an unsupported meter) ends after as many
/// reads, not at the deadline.
const MAX_READS_PER_FRAME: usize = 160;

/// Shared read loop: extract frames from `rx_buf`, reading more data from
/// `transport` when needed.
///
/// - `extract_fn`: protocol-specific frame extractor (e.g. `extract_frame_abcd_be16`).
/// - `accept_fn`: predicate on the extracted payload. Return `true` to accept
///   the frame, `false` to skip it and continue reading (used by UT171/UT181A
///   to filter non-measurement frames).
/// - `recovery`: whether to skip-and-retry on frame errors or propagate them.
/// - `label`: protocol label for log messages (e.g. `"ut8803"`).
/// - `skip_header`: byte pattern to scan for when skipping past a bad frame
///   during error recovery. Typically `&HEADER` (`[0xAB, 0xCD]`) for ABCD
///   protocols, or the family's own header for the others.
///
/// Constants match the values used by all protocol implementations:
/// `READ_TIMEOUT_MS = 2000`, `MAX_FRAMES_EXAMINED = 64`. `READ_TIMEOUT_MS`
/// bounds the total time spent waiting for bytes, not each individual read —
/// see [`read_uart_bytes`] for why a single read can come back empty.
/// `MAX_FRAMES_EXAMINED` counts frames extracted or rejected, not reads: a
/// CP2110 hands up about a byte per report, so a long frame takes more reads
/// than any fixed read cap would allow. Reads are bounded by
/// [`MAX_READS_PER_FRAME`], the deadline and `MAX_RX_BUF`.
pub(crate) fn read_frame<F, A>(
    rx_buf: &mut Vec<u8>,
    transport: &dyn Transport,
    extract_fn: F,
    accept_fn: A,
    recovery: FrameErrorRecovery,
    label: &str,
    skip_header: &[u8],
) -> Result<Vec<u8>>
where
    F: Fn(&[u8]) -> Result<Option<(Vec<u8>, usize)>>,
    A: Fn(&[u8]) -> bool,
{
    const READ_TIMEOUT_MS: i32 = 2000;
    // Guards against an extractor that consumes nothing and against a
    // stream of refused or corrupt frames.
    const MAX_FRAMES_EXAMINED: usize = 64;
    // Bounded growth: the largest legitimate frame we handle is ~21 bytes
    // (UT8803), and we drain on successful extraction or on SkipAndRetry.
    // Cap at 4 KiB so a misbehaving / unsupported protocol family that
    // speaks a stream we can't parse doesn't grow `rx_buf` without bound.
    const MAX_RX_BUF: usize = 4096;

    // Real time, not the session clock: this bounds a USB read, and the wire
    // takes as long as it takes whatever session time is doing.
    let deadline = Instant::now() + Duration::from_millis(READ_TIMEOUT_MS as u64);

    let mut examined = 0;
    let mut reads = 0;
    while examined < MAX_FRAMES_EXAMINED {
        match extract_fn(rx_buf) {
            Ok(Some((payload, consumed))) => {
                examined += 1;
                reads = 0;
                rx_buf.drain(..consumed);
                if accept_fn(&payload) {
                    return Ok(payload);
                }
                debug!(
                    "{label}: skipping non-matching frame ({} bytes): {:02X?}",
                    payload.len(),
                    &payload[..payload.len().min(4)]
                );
            }
            Ok(None) => {
                if rx_buf.len() >= MAX_RX_BUF {
                    // Per buffer while nothing frames (a wrong device, a
                    // wrong baud rate); the timeout it returns is reported.
                    debug!(
                        "{label}: rx_buf hit {MAX_RX_BUF} bytes without a valid frame, clearing"
                    );
                    rx_buf.clear();
                    return Err(Error::Timeout);
                }
                if reads >= MAX_READS_PER_FRAME {
                    debug!("{label}: {reads} reads without a valid frame, giving up");
                    return Err(Error::Timeout);
                }
                let mut tmp = [0u8; 64];
                let n = read_uart_bytes(transport, &mut tmp, deadline)?;
                if n == 0 {
                    return Err(Error::Timeout);
                }
                reads += 1;
                rx_buf.extend_from_slice(&tmp[..n]);
            }
            Err(e) => match recovery {
                FrameErrorRecovery::Propagate => {
                    // Discard the corrupt data so the next request starts
                    // clean — matching the vendor parser's "discard and
                    // clear buffer" on a bad frame (UT61E+ spec §2.1).
                    // Leaving it in place would re-extract the same
                    // corrupt frame on every subsequent request.
                    rx_buf.clear();
                    return Err(e);
                }
                FrameErrorRecovery::SkipAndRetry => {
                    // Per bad frame, e.g. joining a stream mid-frame; a
                    // stream of nothing else ends in a reported timeout.
                    examined += 1;
                    reads = 0;
                    debug!("{label}: frame error: {e}, skipping");
                    if let Some(pos) = rx_buf
                        .windows(skip_header.len())
                        .position(|w| w == skip_header)
                    {
                        rx_buf.drain(..pos + skip_header.len());
                    } else {
                        rx_buf.clear();
                    }
                }
            },
        }
    }

    Err(Error::Timeout)
}

/// Read UART bytes from `transport`, retrying reports that carry no payload.
///
/// A zero-length result from [`Transport::read_timeout`] does not mean the
/// meter went quiet: the HID bridges deliver a report whenever the host polls
/// them, and that report carries no UART payload if nothing arrived since the
/// last poll. The CH9325 signals this with its `0xF0` header (payload length
/// zero, §4.2) and the CH9329 with a zero length byte. A UT804 delivers one
/// byte per report and about 50 empty reports between its packets (issue
/// #16), so most polls are empty and treating the first one as a timeout
/// would abort every frame read.
///
/// Returns `Ok(0)` once `deadline` has passed, i.e. a genuine timeout.
pub(crate) fn read_uart_bytes(
    transport: &dyn Transport,
    buf: &mut [u8],
    deadline: Instant,
) -> Result<usize> {
    // Every empty report costs a USB poll interval (~10 ms), so the deadline
    // is normally what stops us. The counter is a guard against a transport
    // that returns empty without blocking, which would otherwise busy-spin
    // until the deadline.
    const MAX_EMPTY_READS: usize = 256;

    for _ in 0..MAX_EMPTY_READS {
        // `checked_duration_since` rather than `duration_since`: a backward
        // clock jump must not panic here.
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        let timeout_ms = i32::try_from(remaining.as_millis()).unwrap_or(i32::MAX);
        if timeout_ms == 0 {
            return Ok(0);
        }
        let n = transport.read_timeout(buf, timeout_ms)?;
        if n > 0 {
            return Ok(n);
        }
        trace!("read_frame: report carried no payload, retrying until deadline");
    }

    Ok(0)
}

/// Header bytes the `AB CD` families share, UNI-T's and Voltcraft's.
pub const HEADER: [u8; 2] = [0xAB, 0xCD];

/// Minimum valid response length: header(2) + length(1) + checksum(2) = 5
/// (length byte value must be >= 2 to hold at least the checksum)
const MIN_RESPONSE_LEN: usize = 5;

/// Offsets of every [`HEADER`] in `buf`.
///
/// What the detection recognisers scan: each of them asks its own extractor
/// whether a frame starts here, so a false header costs one rejected frame
/// rather than a resync.
pub(crate) fn abcd_header_offsets(buf: &[u8]) -> impl Iterator<Item = usize> + '_ {
    buf.windows(HEADER.len())
        .enumerate()
        .filter(|(_, w)| *w == HEADER)
        .map(|(i, _)| i)
}

/// Build the `AB CD <len> <cmd> [data] <BE16 checksum>` command frame the
/// UT61+ family and both Voltcraft meters send — the write side of
/// [`extract_frame_abcd_be16`].
///
/// `N` is the whole frame: header(2) + len(1) + cmd(1) + `data` + checksum(2).
/// Naming it keeps the frame a plain array, so a `const` ack frame is still a
/// `const`; getting it wrong fails to compile rather than putting a
/// wrong-length frame on the wire.
pub(crate) const fn build_abcd_be16<const N: usize>(cmd: u8, data: &[u8]) -> [u8; N] {
    assert!(
        N == data.len() + 6,
        "frame is header(2) + len(1) + cmd(1) + data + checksum(2)"
    );
    let mut frame = [0u8; N];
    frame[0] = HEADER[0];
    frame[1] = HEADER[1];
    // The length byte counts everything after itself: command, data, checksum.
    frame[2] = (data.len() + 3) as u8;
    frame[3] = cmd;
    let mut i = 0;
    while i < data.len() {
        frame[4 + i] = data[i];
        i += 1;
    }
    let mut sum: u16 = 0;
    let mut i = 0;
    while i < N - 2 {
        sum += frame[i] as u16;
        i += 1;
    }
    frame[N - 2] = (sum >> 8) as u8;
    frame[N - 1] = (sum & 0xFF) as u8;
    frame
}

/// Find `header` in `buf` and return `(start, remaining)` — the offset of the
/// header and the slice from it to the end — but only once at least `min_len`
/// bytes are available from that offset.
///
/// `None` covers both "no header yet" and "header found but the frame is still
/// arriving"; every caller turns it into `Ok(None)`, i.e. "read more".
pub(crate) fn locate<'a>(
    buf: &'a [u8],
    header: &[u8],
    min_len: usize,
) -> Option<(usize, &'a [u8])> {
    let start = buf.windows(header.len()).position(|w| w == header)?;
    let remaining = &buf[start..];
    (remaining.len() >= min_len).then_some((start, remaining))
}

/// 16-bit sum of `bytes`, the shape every AB CD checksum takes.
///
/// `wrapping_add` rather than `Iterator::sum`: for the 2-byte-length framing
/// the summed range can be ~4 KiB of attacker-controlled bytes, and a plain
/// `u16` sum would overflow-panic in debug builds on malformed input. The
/// be16 and ut8803 frames cannot overflow (a 1-byte length caps the range at
/// 256 bytes), so wrapping changes nothing there.
pub(crate) fn sum16(bytes: &[u8]) -> u16 {
    bytes
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16))
}

/// Compare a frame's computed checksum against the one it carries, logging
/// and erroring on a mismatch.
///
/// `expected` is the value read off the wire and `actual` the one we computed
/// — pinned by tests, because swapping them makes every bug report read
/// backwards.
pub(crate) fn checksum_ok(label: &str, computed: u16, received: u16, frame: &[u8]) -> Result<()> {
    if computed != received {
        trace!(
            "framing: {label} checksum mismatch: computed={computed:#06x}, received={received:#06x}, frame={frame:02X?}"
        );
        return Err(Error::ChecksumMismatch {
            expected: received,
            actual: computed,
        });
    }
    Ok(())
}

/// Extract a frame using UT61E+ format: AB CD len payload checksum_BE.
///
/// Length byte counts everything after itself (payload + 2-byte checksum).
/// Checksum is 16-bit BE sum of all bytes before the checksum.
///
/// Returns `Ok(Some((payload, consumed)))` if a valid frame is found,
/// `Ok(None)` if incomplete, `Err` on checksum mismatch.
pub fn extract_frame_abcd_be16(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    let Some((start, remaining)) = locate(buf, &HEADER, MIN_RESPONSE_LEN) else {
        return Ok(None);
    };

    // Byte after header is the "length" — counts everything after itself,
    // i.e. payload + 2-byte checksum. Verified against real device traces.
    let len_byte = remaining[2] as usize;
    if len_byte < 2 {
        // A real frame's length always covers at least the checksum.
        // Returning Ok(None) here would leave this header at the buffer
        // head forever (poisoning reads until the buffer cap); error out
        // so the recovery policy can resync.
        return Err(Error::invalid_response(
            format!("abcd_be16 length byte {len_byte} < 2"),
            remaining,
        ));
    }
    let frame_len = 2 + 1 + len_byte; // header + len_byte + (payload + checksum)
    let payload_len = len_byte - 2;

    if remaining.len() < frame_len {
        return Ok(None);
    }

    let frame = &remaining[..frame_len];
    trace!("framing: raw frame: {:02X?}", frame);

    // Checksum: 16-bit BE sum of all bytes except the last two
    let computed = sum16(&frame[..frame_len - 2]);
    let received = u16::from_be_bytes([frame[frame_len - 2], frame[frame_len - 1]]);
    checksum_ok("abcd_be16", computed, received, frame)?;

    let payload = frame[3..3 + payload_len].to_vec();
    let consumed = start + frame_len;

    trace!("framing: valid frame, payload_len={payload_len}, consumed={consumed}");
    Ok(Some((payload, consumed)))
}

/// Extract a frame using UT181A format: AB CD len_lo len_hi payload chk_lo chk_hi.
///
/// Length is 2-byte LE uint16 = payload_size + 2 (includes checksum bytes).
/// Checksum is 16-bit LE sum of bytes from offset 2 through end of payload
/// (covers length field + payload, excludes header and checksum).
pub fn extract_frame_abcd_2byte_le16(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    // 6 = header(2) + length(2) + checksum(2) minimum
    let Some((start, remaining)) = locate(buf, &HEADER, 6) else {
        return Ok(None);
    };

    let len_val = u16::from_le_bytes([remaining[2], remaining[3]]) as usize;
    if len_val < 2 {
        // See abcd_be16: Ok(None) would pin this header at the buffer
        // head until the cap clears; error out so recovery can resync.
        return Err(Error::invalid_response(
            format!("2byte_le16 length field {len_val} < 2"),
            remaining,
        ));
    }

    let payload_len = len_val - 2;
    let frame_len = 2 + 2 + payload_len + 2; // header + length_field + payload + checksum

    if remaining.len() < frame_len {
        return Ok(None);
    }

    let frame = &remaining[..frame_len];
    trace!("framing: 2byte_le16 raw frame: {:02X?}", frame);

    // Checksum: 16-bit LE sum of bytes[2..frame_len-2] (length field +
    // payload), mod 2^16.
    let computed = sum16(&frame[2..frame_len - 2]);
    let received = u16::from_le_bytes([frame[frame_len - 2], frame[frame_len - 1]]);
    checksum_ok("2byte_le16", computed, received, frame)?;

    let payload = frame[4..4 + payload_len].to_vec();
    let consumed = start + frame_len;

    trace!("framing: 2byte_le16 valid frame, payload_len={payload_len}, consumed={consumed}");
    Ok(Some((payload, consumed)))
}

/// Build a valid UT181A/UT171 frame (2-byte LE length, LE checksum) around
/// `payload` — the write side of [`extract_frame_abcd_2byte_le16`].
///
/// Shared with the detection tests, which need the same frames to arrive from
/// a probe rather than from a parser's own transport.
#[cfg(test)]
pub(crate) fn test_frame_le16(payload: &[u8]) -> Vec<u8> {
    let len_val = (payload.len() + 2) as u16;
    let mut frame = vec![0xAB, 0xCD];
    frame.extend_from_slice(&len_val.to_le_bytes());
    frame.extend_from_slice(payload);
    let sum = sum16(&frame[2..]);
    frame.extend_from_slice(&sum.to_le_bytes());
    frame
}

/// Build a valid AB CD BE16 frame (UT61E+ wire format) around `payload`:
/// length byte = payload + 2 checksum bytes, checksum = 16-bit BE sum.
///
/// Shared with the `lib` and `stream` tests, which drive the same framing
/// through `Dmm`.
#[cfg(test)]
pub(crate) fn test_frame_be16(payload: &[u8]) -> Vec<u8> {
    let len_byte = (payload.len() + 2) as u8;
    let mut frame = vec![0xAB, 0xCD, len_byte];
    frame.extend_from_slice(payload);
    let sum = sum16(&frame);
    frame.push((sum >> 8) as u8);
    frame.push((sum & 0xFF) as u8);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;

    #[test]
    fn extract_valid_frame() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        let result = extract_frame_abcd_be16(&frame).unwrap().unwrap();
        assert_eq!(result.0, payload);
        assert_eq!(result.1, frame.len());
    }

    #[test]
    fn extract_with_leading_garbage() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        let mut buf = vec![0xFF, 0xFE, 0xFD];
        buf.extend_from_slice(&frame);
        let result = extract_frame_abcd_be16(&buf).unwrap().unwrap();
        assert_eq!(result.0, payload);
        assert_eq!(result.1, 3 + frame.len());
    }

    #[test]
    fn extract_incomplete() {
        let frame = vec![0xAB, 0xCD, 0x03, 0x01]; // incomplete
        assert!(extract_frame_abcd_be16(&frame).unwrap().is_none());
    }

    /// A corrupted checksum reports the frame's own value as `expected` and
    /// the sum we computed as `actual` — both pinned, because a swap makes
    /// every bug report read backwards.
    #[test]
    fn extract_bad_checksum() {
        let mut frame = test_frame_be16(&[0x01, 0x02, 0x03]);
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        assert!(matches!(
            extract_frame_abcd_be16(&frame),
            Err(Error::ChecksumMismatch {
                expected: 380,
                actual: 387,
            })
        ));
    }

    /// A frame that is complete except for its last byte is incomplete, not
    /// an error: the extractor must wait for the rest rather than consume it.
    #[test]
    fn abcd_be16_one_byte_short_is_incomplete() {
        let frame = test_frame_be16(&[0x01, 0x02, 0x03]);
        let truncated = &frame[..frame.len() - 1];
        assert!(extract_frame_abcd_be16(truncated).unwrap().is_none());
    }

    #[test]
    fn extract_no_header() {
        let buf = vec![0x00, 0x01, 0x02, 0x03];
        assert!(extract_frame_abcd_be16(&buf).unwrap().is_none());
    }

    #[test]
    fn extract_real_device_frame() {
        // Real frame captured from UT61E+ on DC mV mode, reading " 0.0004"
        let frame = vec![
            0xAB, 0xCD, 0x10, 0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, 0x00, 0x02,
            0x30, 0x30, 0x30, 0x03, 0x8E,
        ];
        let (payload, consumed) = extract_frame_abcd_be16(&frame).unwrap().unwrap();
        assert_eq!(consumed, 19);
        assert_eq!(payload.len(), 14);
        assert_eq!(payload[0], 0x02);
        assert_eq!(payload[1] & 0x0F, 0x00);
        assert_eq!(&payload[2..9], b" 0.0004");
    }

    #[test]
    fn abcd_be16_short_length_errors_for_resync() {
        let buf = vec![0xAB, 0xCD, 0x01, 0x00, 0x00, 0x00];
        assert!(extract_frame_abcd_be16(&buf).is_err());
    }

    #[test]
    fn abcd_2byte_le16_short_length_errors_for_resync() {
        let buf = vec![0xAB, 0xCD, 0x01, 0x00, 0x00, 0x00];
        assert!(extract_frame_abcd_2byte_le16(&buf).is_err());
    }

    #[test]
    fn abcd_2byte_le16_checksum_overflow_does_not_panic() {
        // ~4 KiB of 0xFF after a large length field: a plain u16 Sum over
        // the checksum range would overflow-panic in debug builds.
        let len: u16 = 4000;
        let mut buf = vec![0xAB, 0xCD];
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend(std::iter::repeat_n(0xFFu8, len as usize + 2));
        // Wrong checksum is fine — it must reject, not panic.
        assert!(extract_frame_abcd_2byte_le16(&buf).is_err());
    }

    #[test]
    fn le16_frame_ut181a() {
        // Build a valid UT181A frame (2-byte LE length = payload + 2)
        let payload = vec![0x02, 0x00, 0x11, 0x31]; // type + some data
        let len_val = (payload.len() + 2) as u16; // payload + checksum
        let mut frame = vec![0xAB, 0xCD];
        frame.push((len_val & 0xFF) as u8);
        frame.push((len_val >> 8) as u8);
        frame.extend_from_slice(&payload);
        // Checksum over bytes[2..frame.len()] = length + payload
        let sum: u16 = frame[2..].iter().map(|&b| b as u16).sum();
        frame.push((sum & 0xFF) as u8);
        frame.push((sum >> 8) as u8);

        let (p, consumed) = extract_frame_abcd_2byte_le16(&frame).unwrap().unwrap();
        assert_eq!(p, payload);
        assert_eq!(consumed, frame.len());
    }

    #[test]
    fn le16_frame_ut171() {
        // UT171 shares the UT181A framing: 2-byte LE length = payload +
        // checksum. This is the real connect command capture.
        let frame = [0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x01, 0x0F, 0x00];
        let (p, consumed) = extract_frame_abcd_2byte_le16(&frame).unwrap().unwrap();
        assert_eq!(p, vec![0x0A, 0x01]);
        assert_eq!(consumed, frame.len());
    }

    /// As `extract_with_leading_garbage`, for the 2-byte-length framing.
    #[test]
    fn le16_leading_garbage() {
        let payload = vec![0x02, 0x00, 0x11, 0x31];
        let frame = test_frame_le16(&payload);
        let mut buf = vec![0xFF, 0xFE, 0xFD];
        buf.extend_from_slice(&frame);
        let (p, consumed) = extract_frame_abcd_2byte_le16(&buf).unwrap().unwrap();
        assert_eq!(p, payload);
        assert_eq!(consumed, 3 + frame.len());
    }

    /// As `extract_bad_checksum`, for the LE checksum. `expected` is the
    /// little-endian read of the corrupted trailer, not a byte-swap of it.
    #[test]
    fn le16_bad_checksum() {
        let mut frame = test_frame_le16(&[0x02, 0x00, 0x11, 0x31]);
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        assert!(matches!(
            extract_frame_abcd_2byte_le16(&frame),
            Err(Error::ChecksumMismatch {
                expected: 65354,
                actual: 74,
            })
        ));
    }

    /// As `abcd_be16_one_byte_short_is_incomplete`, for the 2-byte length.
    #[test]
    fn le16_one_byte_short_is_incomplete() {
        let frame = test_frame_le16(&[0x02, 0x00, 0x11, 0x31]);
        let truncated = &frame[..frame.len() - 1];
        assert!(extract_frame_abcd_2byte_le16(truncated).unwrap().is_none());
    }

    // --- read_frame tests ---

    #[test]
    fn read_frame_single_chunk() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        let mock = MockTransport::new(vec![frame]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, payload);
        assert!(rx_buf.is_empty());
    }

    #[test]
    fn read_frame_split_across_reads() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        // Split the frame into two parts
        let part1 = frame[..3].to_vec();
        let part2 = frame[3..].to_vec();
        let mock = MockTransport::new(vec![part1, part2]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, payload);
    }

    /// A CP2110 hands up about a byte per report, so a VC-890's 66-byte live
    /// frame (61-byte payload) takes 66 reads. The cap counts frames, not
    /// reads, or the frame would time out a few bytes short.
    #[test]
    fn read_frame_takes_a_long_frame_a_byte_at_a_time() {
        let payload: Vec<u8> = (0..61).collect();
        let frame = test_frame_be16(&payload);
        let mock = MockTransport::new(frame.iter().map(|&b| vec![b]).collect());
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, payload);
    }

    /// A frame joined just after its start, a byte per report, completes:
    /// the rest of the one in flight, then a whole one.
    #[test]
    fn read_frame_takes_a_long_frame_joined_mid_way_a_byte_at_a_time() {
        let payload: Vec<u8> = (0..61).collect();
        let frame = test_frame_be16(&payload);
        let mut stream = frame[1..].to_vec();
        stream.extend(&frame);
        let mock = MockTransport::new(stream.iter().map(|&b| vec![b]).collect());
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "test",
            &HEADER,
        );
        assert_eq!(result.unwrap(), payload);
    }

    /// A byte per report that never frames — a wrong baud rate, an
    /// unsupported meter — ends after [`MAX_READS_PER_FRAME`] reads, not at
    /// the deadline or the buffer cap.
    #[test]
    fn read_frame_gives_up_on_noise_after_a_bounded_number_of_reads() {
        struct Noise(std::cell::Cell<usize>);
        impl Transport for Noise {
            fn write(&self, _data: &[u8]) -> Result<()> {
                Ok(())
            }
            fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
                self.0.set(self.0.get() + 1);
                buf[0] = 0x55;
                Ok(1)
            }
            fn link(&self) -> Option<crate::transport::Link> {
                None
            }
        }
        let noise = Noise(std::cell::Cell::new(0));
        let mut rx_buf = Vec::new();
        let start = Instant::now();

        let result = read_frame(
            &mut rx_buf,
            &noise,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        );
        assert!(matches!(result, Err(Error::Timeout)));
        assert_eq!(noise.0.get(), MAX_READS_PER_FRAME);
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "ended before the deadline"
        );
    }

    /// The cap still ends a stream of frames the filter refuses, after 64 of
    /// them, while more are queued.
    #[test]
    fn read_frame_gives_up_on_a_stream_of_refused_frames() {
        let frames = (0..70).map(|i| test_frame_be16(&[0x01, i])).collect();
        let mock = MockTransport::new(frames);
        let mut rx_buf = Vec::new();
        let refused = std::cell::Cell::new(0);

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| {
                refused.set(refused.get() + 1);
                false
            },
            FrameErrorRecovery::SkipAndRetry,
            "test",
            &HEADER,
        );
        assert!(matches!(result, Err(Error::Timeout)));
        assert_eq!(refused.get(), 64);
    }

    /// HID bridges answer a poll with an empty payload whenever the meter has
    /// sent nothing since the last one. Those must not end the frame read —
    /// a UT804 sends about 50 empty reports between its packets (issue #16).
    #[test]
    fn read_frame_survives_empty_reports_between_data() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        let part1 = frame[..3].to_vec();
        let part2 = frame[3..].to_vec();
        let mock = MockTransport::new(vec![Vec::new(), Vec::new(), part1, Vec::new(), part2]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, payload);
    }

    #[test]
    fn read_frame_timeout_when_no_data() {
        let mock = MockTransport::new(vec![]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        );
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[test]
    fn read_frame_propagate_error() {
        // Build a frame with a corrupted checksum
        let mut frame = test_frame_be16(&[0x01, 0x02, 0x03]);
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        let mock = MockTransport::new(vec![frame]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        );
        assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
    }

    #[test]
    fn read_frame_skip_and_retry_on_error() {
        // First frame has bad checksum, second is valid
        let mut bad_frame = test_frame_be16(&[0x01, 0x02, 0x03]);
        let last = bad_frame.len() - 1;
        bad_frame[last] ^= 0xFF;

        let good_payload = vec![0x04, 0x05, 0x06];
        let good_frame = test_frame_be16(&good_payload);

        // Concatenate bad + good into one response chunk so the retry finds the good frame
        let mut combined = bad_frame;
        combined.extend_from_slice(&good_frame);
        let mock = MockTransport::new(vec![combined]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, good_payload);
    }

    #[test]
    fn read_frame_accept_filter_skips_non_matching() {
        // First frame has payload starting with 0x01 (rejected), second with 0x02 (accepted)
        let rejected_payload = vec![0x01, 0xAA, 0xBB];
        let accepted_payload = vec![0x02, 0xCC, 0xDD];
        let frame1 = test_frame_be16(&rejected_payload);
        let frame2 = test_frame_be16(&accepted_payload);

        let mut combined = frame1;
        combined.extend_from_slice(&frame2);
        let mock = MockTransport::new(vec![combined]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |p| !p.is_empty() && p[0] == 0x02,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, accepted_payload);
    }

    #[test]
    fn read_frame_existing_data_in_rx_buf() {
        // Frame data is already in rx_buf before read_frame is called
        let payload = vec![0x01, 0x02, 0x03];
        let frame = test_frame_be16(&payload);
        let mock = MockTransport::new(vec![]); // no transport reads needed
        let mut rx_buf = frame;

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        )
        .unwrap();
        assert_eq!(result, payload);
        assert!(rx_buf.is_empty());
    }

    #[test]
    fn read_frame_caps_rx_buf_when_garbage_spans_calls() {
        // rx_buf is persistent across `read_frame` calls, so a previous
        // call can leave it partially filled. Simulate that: preload it
        // with 4000 bytes of junk, then give the transport 64-byte reads
        // of more junk. The cap should fire on the first iteration where
        // `rx_buf.len() >= 4096` and clear the buffer before returning.
        let garbage = vec![vec![0x55u8; 64]; 10];
        let mock = MockTransport::new(garbage);
        let mut rx_buf = vec![0x55u8; 4000];
        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "test",
            &HEADER,
        );
        assert!(matches!(result, Err(Error::Timeout)));
        // Cap path clears rx_buf; the MAX_FRAMES_EXAMINED exit path does not.
        assert!(
            rx_buf.is_empty(),
            "rx_buf should have been cleared by the 4 KiB cap (got {} bytes)",
            rx_buf.len()
        );
    }
}
