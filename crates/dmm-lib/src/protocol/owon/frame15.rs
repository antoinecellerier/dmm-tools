//! Finding 15-byte frames in the byte stream
//! (`docs/research/owon/reverse-engineered-protocol.md` §10.2, §10.9).
//!
//! A frame has no header, length or checksum (spec §10.2). What marks one is
//! its main function/range word's high byte, bits 16-23 of the G24: `F0`,
//! the offline dump's marker for a G24 word (spec §10.9), in every VC871
//! frame (spec §14.2). The sub-display's G24 carries it too, six bytes on
//! (spec §14.4), so a start six bytes late also has `F0` at byte 2; there
//! the next frame's byte 2 lands at byte 11, so a start whose byte 11 is
//! `F0` is that one. In a true frame byte 11 is the sub-display reading's
//! top byte, which is `F0` only for a negative reading with status code 7
//! (spec §10.4). A reading whose low or top byte is `F0` puts the marker
//! at a false start too; such a start lands a reading byte on the main
//! word's bits 8-15, whose bits 13-15 are clear in every VC871 frame
//! (spec §14.5), so a start with them set is not a frame. A negative
//! status-7 reading (top byte `F0`) with a magnitude below 0x2000 still
//! passes: off a mid-frame start the read then stays a few bytes late for
//! as long as that reading lasts.

use crate::error::Result;
use log::debug;

/// Five 24-bit words (spec §10.2).
pub(super) const FRAME_LEN: usize = 15;

/// A G24 word's high byte (spec §10.9).
pub(super) const MARKER: u8 = 0xF0;

/// Where the first frame in `buf` starts: the first offset whose byte 2 is
/// the marker and, unless `aligned` and at 0, whose byte 1 has bits 5-7
/// (G24 bits 13-15) clear and whose byte 11 is not the marker. A start
/// whose byte 11 has not arrived yet is taken, to wait for the rest.
fn first_start(buf: &[u8], aligned: bool) -> Option<usize> {
    (0..buf.len().saturating_sub(2)).find(|&start| {
        buf[start + 2] == MARKER
            && ((aligned && start == 0)
                || (buf[start + 1] & 0xE0 == 0 && buf.get(start + 11).is_none_or(|&b| b != MARKER)))
    })
}

/// Whether `buf` reads as a 15-byte stream: two frames in a row whose
/// 15-byte steps run whole to the end of the buffer, every step with the
/// marker at byte 2, the first also passing [`first_start`]'s checks off an
/// unaligned start. Over Bluetooth the frames arrive whole (spec §14.2), so
/// a 15-byte stream passes from its second frame on. A 6-byte stream seldom
/// does. Steps 15 bytes apart land on its function word's high byte and its
/// reading's low byte by turns (spec §5), and the first step's byte 11 on
/// the other of the two, so it passes only where a reading's low byte of
/// `F0` meets a change of reading or range between frames.
pub(super) fn tiles(buf: &[u8]) -> bool {
    (0..(buf.len() + 1).saturating_sub(2 * FRAME_LEN)).any(|start| {
        (buf.len() - start).is_multiple_of(FRAME_LEN)
            && first_start(&buf[start..], false) == Some(0)
            && buf[start..].chunks(FRAME_LEN).all(|step| step[2] == MARKER)
    })
}

/// Whether `buf` holds a frame's worth of bytes and no marker anywhere:
/// something that is not this frame.
pub(super) fn lacks_marker(buf: &[u8]) -> bool {
    buf.len() >= FRAME_LEN && !buf.contains(&MARKER)
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
            "owon: dropping {start} bytes before a 15-byte frame: {:02X?}",
            &buf[..start]
        );
    }
    Ok(Some((buf[start..end].to_vec(), end)))
}

/// Whether OWON's app reads `frame`: it skips one whose byte 14 is `FF`
/// (spec §10.2), which no VC871 capture holds (spec §14.2).
pub(super) fn kept(frame: &[u8]) -> bool {
    frame.last() != Some(&0xFF)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Spec §14.5's whole VC871 frames, in its order.
    pub(crate) const VECTORS: [[u8; 15]; 28] = [
        [
            0x76, 0x01, 0xF0, 0xF6, 0x00, 0x00, 0xA1, 0x09, 0xF0, 0xF6, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x24, 0x00, 0xF0, 0x46, 0x27, 0x80, 0xA1, 0x09, 0xF0, 0x46, 0x27, 0x80, 0x05, 0x00,
            0x00,
        ],
        [
            0x1F, 0x00, 0xF0, 0x19, 0x11, 0x11, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x21, 0x12, 0xF0, 0xF9, 0x00, 0x00, 0x61, 0x1A, 0xF0, 0x00, 0x03, 0x00, 0x00, 0x00,
            0x00,
        ],
        [
            0xA1, 0x13, 0xF0, 0x00, 0x00, 0x00, 0xE1, 0x1B, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x20,
            0x00,
        ],
        [
            0x20, 0x15, 0xF0, 0x00, 0x00, 0x00, 0xE0, 0x1C, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x80,
            0x00,
        ],
        [
            0x24, 0x00, 0xF0, 0x21, 0x15, 0x00, 0xA1, 0x09, 0xF0, 0x21, 0x15, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x1A, 0x00, 0xF0, 0x3A, 0xD3, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x59, 0x10, 0xF0, 0x3B, 0x02, 0x00, 0xA2, 0x19, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ],
        [
            0x29, 0x01, 0xF0, 0x71, 0x0E, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xA4, 0x02, 0xF0, 0x83, 0x15, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ],
        [
            0x4C, 0x01, 0xF0, 0xB6, 0x02, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xA3, 0x11, 0xF0, 0x53, 0xC3, 0x00, 0xE2, 0x19, 0xF0, 0x77, 0x11, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xE2, 0x11, 0xF0, 0x7A, 0x14, 0x00, 0xA3, 0x19, 0xF0, 0x96, 0xC6, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x21, 0x12, 0xF0, 0xE2, 0x00, 0x00, 0x61, 0x1A, 0xF0, 0xD8, 0x02, 0x00, 0x00, 0x00,
            0x00,
        ],
        [
            0x61, 0x12, 0xF0, 0x0F, 0x03, 0x00, 0x21, 0x1A, 0xF0, 0x01, 0x01, 0x00, 0x00, 0x00,
            0x00,
        ],
        [
            0x91, 0x00, 0xF0, 0x36, 0x3E, 0x80, 0x21, 0x0A, 0xF0, 0x7E, 0x0A, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xD1, 0x10, 0xF0, 0x88, 0x06, 0x00, 0xA2, 0x19, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0x9B, 0x00, 0xF0, 0x90, 0x06, 0x80, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xDA, 0x10, 0xF0, 0x62, 0x00, 0x00, 0xA2, 0x19, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xA4, 0x00, 0xF0, 0x4D, 0x0B, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xE3, 0x10, 0xF0, 0x4E, 0x00, 0x00, 0xA2, 0x19, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ],
        [
            0xA1, 0x13, 0xF0, 0x82, 0x12, 0x00, 0xE1, 0x1B, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x20,
            0x00,
        ],
        [
            0x23, 0x14, 0xF0, 0x01, 0x00, 0x80, 0xA1, 0x19, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x30,
            0x00,
        ],
        [
            0x62, 0x15, 0xF0, 0x00, 0x00, 0x00, 0xA2, 0x1D, 0xF0, 0x1D, 0x00, 0x00, 0x00, 0x80,
            0x00,
        ],
        [
            0x98, 0x14, 0xF0, 0x00, 0x00, 0x00, 0xE0, 0x1C, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x80,
            0x00,
        ],
        [
            0x61, 0x15, 0xF0, 0x04, 0x00, 0x00, 0xE1, 0x18, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x20,
            0x00,
        ],
        [
            0xA3, 0x00, 0xF0, 0x00, 0x00, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x01,
        ],
    ];

    /// Every vector, back to back.
    pub(crate) fn stream() -> Vec<u8> {
        VECTORS.concat()
    }

    #[test]
    fn every_vector_is_one_frame() {
        for v in VECTORS {
            assert_eq!(extract(&v, false).unwrap(), Some((v.to_vec(), FRAME_LEN)));
            assert!(kept(&v));
        }
    }

    /// Joined anywhere in a frame, the next whole frame is the first taken,
    /// the sub-display's `F0` at byte 8 included.
    #[test]
    fn resync_from_every_offset() {
        let stream = stream();
        for skip in 1..FRAME_LEN {
            let (frame, used) = extract(&stream[skip..], false).unwrap().unwrap();
            assert_eq!(frame, VECTORS[1], "{skip}");
            assert_eq!(used, 2 * FRAME_LEN - skip, "{skip}");
        }
        // Not yet whole: wait.
        assert_eq!(extract(&VECTORS[0][..14], false).unwrap(), None);
        assert_eq!(extract(&[], false).unwrap(), None);
    }

    /// The frames taken from `buf`, read on as `framing::read_frame` does:
    /// unaligned at first, aligned after each frame.
    fn frames_from(buf: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        let mut rest = buf;
        while let Some((frame, used)) = extract(rest, !frames.is_empty()).unwrap() {
            frames.push(frame);
            rest = &rest[used..];
        }
        frames
    }

    /// A steady reading of count 496 has `F0` as its low byte, so a start
    /// one byte early has the marker at byte 2 and none at byte 11; its
    /// byte 1, the true marker, has bits 5-7 set. Joined anywhere, every
    /// frame taken is the true one.
    #[test]
    fn a_reading_with_f0_as_its_low_byte_does_not_hold_the_read_late() {
        let v = [
            0x24, 0x00, 0xF0, 0xF0, 0x01, 0x00, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ];
        let stream = v.repeat(6);
        for skip in 1..FRAME_LEN {
            let frames = frames_from(&stream[skip..]);
            assert_eq!(frames.len(), 5, "{skip}");
            assert!(frames.iter().all(|f| f[..] == v[..]), "{skip}");
        }
    }

    /// A negative reading with status 7 has `F0` as its top byte, making a
    /// false start at byte 3 (the main reading) or 9 (the sub-display's).
    /// With magnitude bits 13-15 set (here 0x9C40), that start's byte 1 has
    /// them set too, and it is passed over. The sub-display's case takes no
    /// frame while it lasts, its byte 11 being the marker, and reads on from
    /// the next frame.
    #[test]
    fn a_reading_with_f0_as_its_top_byte_is_passed_over() {
        let main = [
            0x24, 0x00, 0xF0, 0x40, 0x9C, 0xF0, 0xA2, 0x09, 0xF0, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00,
        ];
        let stream = main.repeat(6);
        for skip in 1..FRAME_LEN {
            let frames = frames_from(&stream[skip..]);
            assert_eq!(frames.len(), 5, "{skip}");
            assert!(frames.iter().all(|f| f[..] == main[..]), "{skip}");
        }
        let sub = [
            0x24, 0x00, 0xF0, 0x23, 0x01, 0x00, 0xA2, 0x09, 0xF0, 0x40, 0x9C, 0xF0, 0x04, 0x00,
            0x00,
        ];
        let stream = [sub.repeat(4), VECTORS[2].repeat(2)].concat();
        for skip in 1..FRAME_LEN {
            let frames = frames_from(&stream[skip..]);
            assert_eq!(frames, [VECTORS[2].to_vec(), VECTORS[2].to_vec()], "{skip}");
        }
    }

    /// Aligned, byte 2 alone places the frame, whatever bytes 1 and 11 hold.
    #[test]
    fn an_aligned_frame_needs_only_byte_2() {
        let mut v = VECTORS[1];
        v[11] = MARKER;
        assert_eq!(extract(&v, true).unwrap(), Some((v.to_vec(), FRAME_LEN)));
        assert_eq!(extract(&v, false).unwrap(), None);
        let mut v = VECTORS[1];
        v[1] = 0x20;
        assert_eq!(extract(&v, true).unwrap(), Some((v.to_vec(), FRAME_LEN)));
        assert_eq!(extract(&v, false).unwrap(), None);
    }

    /// Two whole frames in a row, from any frame boundary, tile; one frame,
    /// or a buffer ending mid-frame, does not yet.
    #[test]
    fn a_15_byte_stream_tiles_from_its_second_frame() {
        let stream = stream();
        for frames in 2..=VECTORS.len() {
            for first in 0..=VECTORS.len() - frames {
                let buf = &stream[first * FRAME_LEN..(first + frames) * FRAME_LEN];
                assert!(tiles(buf), "{first} {frames}");
                assert!(
                    tiles(&[&[0x01, 0x02][..], buf].concat()),
                    "{first} {frames}"
                );
            }
        }
        assert!(!tiles(&VECTORS[0]));
        for cut in 1..FRAME_LEN {
            assert!(!tiles(&stream[..2 * FRAME_LEN + cut]), "{cut}");
        }
    }

    /// A 6-byte stream does not tile as 15-byte frames from any offset: the
    /// 6-byte vectors, and a steady reading whose low byte is `F0`.
    #[test]
    fn a_6_byte_stream_does_not_tile() {
        let six = super::super::frame::tests::stream();
        let steady = [0x19, 0xF0, 0x04, 0x00, 0xF0, 0x00].repeat(10);
        for buf in [&six[..], &steady[..]] {
            for skip in 0..buf.len() {
                for end in skip..=buf.len() {
                    assert!(!tiles(&buf[skip..end]), "{skip} {end}");
                }
            }
        }
    }

    #[test]
    fn a_frame_ending_in_ff_is_not_kept() {
        let mut v = VECTORS[0];
        assert!(kept(&v));
        v[14] = 0xFF;
        assert!(!kept(&v));
    }

    #[test]
    fn a_stream_with_no_marker_is_told_apart() {
        // The older B35T's 14-byte frame (spec §14.5), twice.
        let ascii = [
            0x2B, 0x33, 0x36, 0x32, 0x33, 0x20, 0x34, 0x31, 0x00, 0x40, 0x80, 0x24, 0x0D, 0x0A,
        ]
        .repeat(2);
        assert!(lacks_marker(&ascii));
        assert!(!lacks_marker(&ascii[..14]));
        assert!(!lacks_marker(&stream()));
    }
}
