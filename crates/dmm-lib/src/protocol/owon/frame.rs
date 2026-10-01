//! Finding 6-byte frames in the byte stream
//! (`docs/research/owon/reverse-engineered-protocol.md` §5, §6.1).
//!
//! A frame has no start byte, length or checksum (spec §5). What marks one
//! is its function/range word: bits 10-15 read `111100` in every live frame
//! seen, on all five 6-byte models (spec §6.1, §14.2), so byte 1 masked
//! with `FC` is `F0`. A reading word carries it only with the sign set and
//! a magnitude of 0x7000-0x73FF (spec §8.3), above every model's count.

use crate::error::Result;
use log::debug;

/// Three 16-bit words (spec §5).
pub(super) const FRAME_LEN: usize = 6;

/// Whether `high`, the function word's high byte, carries the marker.
pub(super) fn has_marker(high: u8) -> bool {
    high & 0xFC == 0xF0
}

/// Where the first frame in `buf` starts: the first offset whose byte 1
/// carries the marker.
///
/// Unless `aligned`, the buffer may start mid-frame, and a reading whose low
/// byte is F0-F3 (counts 240-243, 496-499, ...) puts a marker where no frame
/// starts. Such a false start lands status byte 3 on the function word's
/// low byte (spec §6.1), so off an aligned start a frame also needs status
/// byte 3 clear, as in every 6-byte capture (spec §14.4, "Not seen"). A
/// start whose byte 3 has not arrived yet is taken, to wait for the rest.
fn first_start(buf: &[u8], aligned: bool) -> Option<usize> {
    (0..buf.len().saturating_sub(1)).find(|&start| {
        has_marker(buf[start + 1])
            && ((aligned && start == 0) || buf.get(start + 3).is_none_or(|&b| b == 0))
    })
}

/// Whether `buf` holds a frame's worth of bytes and no marker anywhere:
/// something that is not this frame, such as the older B35T's 14-byte ASCII
/// (spec §11).
pub(super) fn lacks_marker(buf: &[u8]) -> bool {
    buf.len() >= FRAME_LEN && !buf.iter().any(|&b| has_marker(b))
}

/// The extractor `framing::read_frame` takes: the first whole frame, and
/// the bytes it used up. What comes before it is dropped. Never fails.
/// `aligned` says `buf` starts where the last frame taken ended.
pub(super) fn extract(buf: &[u8], aligned: bool) -> Result<Option<(Vec<u8>, usize)>> {
    let Some(start) = first_start(buf, aligned) else {
        return Ok(None);
    };
    let end = start + FRAME_LEN;
    if buf.len() < end {
        return Ok(None);
    }
    if start > 0 {
        debug!(
            "owon: dropping {start} bytes before a frame: {:02X?}",
            &buf[..start]
        );
    }
    Ok(Some((buf[start..end].to_vec(), end)))
}

/// Whether `frame` passes detection's stricter test: the marker, and status
/// bits 6-15 clear, as in every 6-byte capture (spec §14.4, "Not seen").
pub(super) fn plausible(frame: &[u8]) -> bool {
    frame.len() == FRAME_LEN && has_marker(frame[1]) && frame[3] == 0 && frame[2] & 0xC0 == 0
}

/// Whether two plausible frames follow each other anywhere in `buf`.
pub(super) fn two_in_a_row(buf: &[u8]) -> bool {
    buf.windows(2 * FRAME_LEN)
        .any(|w| plausible(&w[..FRAME_LEN]) && plausible(&w[FRAME_LEN..]))
}

/// Whether `buf`, which may start mid-frame, holds one whole frame.
pub(super) fn holds_frame(buf: &[u8]) -> bool {
    first_start(buf, false).is_some_and(|start| buf.len() >= start + FRAME_LEN)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The 6-byte frames of spec §12.1 and §14.5, in order, but for
    /// `2C F1 04 00 0D 7F`, caught mid-range-change (spec §14.5).
    pub(crate) const VECTORS: [[u8; 6]; 18] = [
        [0x19, 0xF0, 0x04, 0x00, 0xBD, 0x09],
        [0x19, 0xF0, 0x04, 0x00, 0xE9, 0x0D],
        [0x20, 0xF2, 0x00, 0x00, 0x1D, 0x00],
        [0x63, 0xF0, 0x04, 0x00, 0x10, 0x00],
        [0x21, 0xF1, 0x04, 0x00, 0x07, 0x00],
        [0xE7, 0xF2, 0x00, 0x00, 0x00, 0x00],
        [0x37, 0xF1, 0x04, 0x00, 0x00, 0x00],
        [0xA7, 0xF2, 0x00, 0x00, 0x00, 0x00],
        [0x19, 0xF0, 0x04, 0x00, 0x2B, 0x89],
        [0x19, 0xF0, 0x04, 0x00, 0x00, 0x80],
        [0x19, 0xF0, 0x04, 0x00, 0x49, 0x04],
        [0x34, 0xF1, 0x04, 0x00, 0x81, 0x2B],
        [0x24, 0xF0, 0x04, 0x00, 0xC6, 0x3A],
        [0x22, 0xF1, 0x04, 0x00, 0x9F, 0x11],
        [0x2B, 0xF1, 0x04, 0x00, 0x56, 0x09],
        [0x22, 0xF0, 0x04, 0x00, 0x00, 0x00],
        [0x24, 0xF0, 0x05, 0x00, 0x1F, 0x00],
        [0x24, 0xF0, 0x04, 0x00, 0x03, 0x00],
    ];

    /// Every vector, back to back.
    pub(crate) fn stream() -> Vec<u8> {
        VECTORS.concat()
    }

    #[test]
    fn every_vector_is_a_plausible_frame() {
        for v in VECTORS {
            assert!(plausible(&v), "{v:02X?}");
            assert_eq!(extract(&v, false).unwrap(), Some((v.to_vec(), FRAME_LEN)));
        }
    }

    /// Bytes before the marker are dropped; a frame cut short waits.
    #[test]
    fn extraction_starts_at_the_marker() {
        let v = VECTORS[0];
        let mut buf = vec![0xBD, 0x09];
        buf.extend_from_slice(&v);
        assert_eq!(extract(&buf, false).unwrap(), Some((v.to_vec(), 8)));
        assert_eq!(extract(&v[..5], false).unwrap(), None);
        assert_eq!(extract(&[0x19], false).unwrap(), None);
        assert_eq!(extract(&[], false).unwrap(), None);
    }

    /// Off an aligned start, a marker in the reading's low byte is no
    /// frame: its byte 3 is the function word's low byte. Aligned, a frame
    /// with status byte 3 set is still taken, for decode to report.
    #[test]
    fn a_marker_in_the_reading_needs_status_byte_3_clear() {
        // Count 241, `F1 00`, joined at frame byte 3: `00 F1 00 19 F0 04`.
        let v = [0x19, 0xF0, 0x04, 0x00, 0xF1, 0x00];
        let buf = [&v[3..], &v[..], &v[..]].concat();
        assert_eq!(extract(&buf, false).unwrap(), Some((v.to_vec(), 9)));
        assert_eq!(extract(&buf[..6], false).unwrap(), None, "byte 3 to come");
        let mut high = v;
        high[3] = 0x01;
        assert_eq!(extract(&high, true).unwrap(), Some((high.to_vec(), 6)));
        assert_eq!(extract(&high, false).unwrap(), None);
    }

    #[test]
    fn plausibility_wants_the_marker_and_clear_high_status_bits() {
        let mut v = VECTORS[0];
        assert!(plausible(&v));
        v[2] = 0x44;
        assert!(!plausible(&v), "bit 6");
        v[2] = 0x84;
        assert!(!plausible(&v), "bit 7");
        v[2] = 0x04;
        v[3] = 0x01;
        assert!(!plausible(&v), "bit 8");
        let mut v = VECTORS[0];
        v[1] = 0xE0;
        assert!(!plausible(&v), "marker");
        assert!(!plausible(&VECTORS[0][..5]));
    }

    #[test]
    fn two_in_a_row_wants_both_frames() {
        let stream = stream();
        assert!(two_in_a_row(&stream));
        assert!(two_in_a_row(&stream[3..]));
        assert!(!two_in_a_row(&stream[..11]));
        assert!(!two_in_a_row(&VECTORS[0]));
        assert!(holds_frame(&VECTORS[0]));
        assert!(!holds_frame(&VECTORS[0][..5]));
    }

    #[test]
    fn a_stream_with_no_marker_is_told_apart() {
        // The older B35T's 14-byte frame (spec §14.5).
        let ascii = [
            0x2B, 0x33, 0x36, 0x32, 0x33, 0x20, 0x34, 0x31, 0x00, 0x40, 0x80, 0x24, 0x0D, 0x0A,
        ];
        assert!(lacks_marker(&ascii));
        assert!(!lacks_marker(&ascii[..5]));
        assert!(!lacks_marker(&stream()));
        // A marker at index 0 counts too: frame byte 1 onwards.
        assert!(!lacks_marker(&[0xF0, 0x04, 0x00, 0xBD, 0x09, 0x19]));
    }
}
