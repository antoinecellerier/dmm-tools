//! Finding UT8802 frames in the byte stream
//! (`docs/research/uci-bench-family/reverse-engineered-protocol.md` §3).

use crate::error::{Error, Result};
use crate::protocol::framing::locate;
use log::trace;

/// Header byte for UT8802 frames.
pub(super) const UT8802_HEADER: [u8; 1] = [0xAC];

/// Fixed frame length for UT8802: header(1) + position(1) + digits(3) + dp_flags(1) + status(1) + sign(1) = 8.
pub(super) const UT8802_FRAME_LEN: usize = 8;

/// Valid BCD nibble values: 0x0-0x9 (digits), 0x0A (treated as zero), 0x0C (overload 'L').
fn is_valid_bcd_nibble(nibble: u8) -> bool {
    nibble <= 0x0A || nibble == 0x0C
}

/// Valid position codes for the UT8802 (§3.3: the programming manual's
/// table, which the vendor DLL matches).
/// Gaps: 0x00, 0x02, 0x07, 0x08, 0x0F, 0x15, 0x17, 0x1E, 0x20, 0x21, 0x26, and anything > 0x2D.
fn is_valid_ut8802_position(pos: u8) -> bool {
    matches!(
        pos,
        0x01 | 0x03..=0x06
            | 0x09..=0x0E
            | 0x10..=0x14
            | 0x16
            | 0x18..=0x1D
            | 0x1F
            | 0x22..=0x25
            | 0x27..=0x2D
    )
}

/// Extract a frame using UT8802 format: `0xAC` header, fixed 8-byte frame, no checksum.
///
/// The UT8802 wire protocol has no checksum field (all 8 bytes are data).
/// To compensate, we validate the position code and BCD nibbles — this is
/// stricter than the vendor parser, which only checks the header byte.
///
/// Returns `Ok(Some((payload, consumed)))` where payload is bytes 1..8 (7 bytes),
/// `Ok(None)` if incomplete, `Err` on validation failure (invalid position code
/// or BCD nibble).
///
/// See docs/research/uci-bench-family/reverse-engineered-protocol.md section 3.
pub(super) fn extract_frame_ut8802(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    let Some((start, remaining)) = locate(buf, &UT8802_HEADER, UT8802_FRAME_LEN) else {
        return Ok(None);
    };

    let frame = &remaining[..UT8802_FRAME_LEN];
    trace!("framing: ut8802 raw frame: {:02X?}", frame);

    // Validate position code (byte 1)
    let position = frame[1];
    if !is_valid_ut8802_position(position) {
        trace!(
            "framing: ut8802 invalid position code {:#04x}, frame={frame:02X?}",
            position
        );
        return Err(Error::invalid_response(
            format!("ut8802 invalid position code {position:#04x}"),
            frame,
        ));
    }

    // Validate the 5 display nibbles from bytes 2-4. Display order is
    // MSD = byte 4 low nibble … LSD = byte 2 low nibble (see
    // ut8802::parse_measurement); the order is irrelevant for validation.
    let nibbles = [
        frame[4] & 0x0F, // digit 1 (MSD)
        frame[3] >> 4,   // digit 2
        frame[3] & 0x0F, // digit 3
        frame[2] >> 4,   // digit 4
        frame[2] & 0x0F, // digit 5 (LSD)
    ];
    for (i, &nibble) in nibbles.iter().enumerate() {
        if !is_valid_bcd_nibble(nibble) {
            trace!(
                "framing: ut8802 invalid BCD nibble {:#04x} at digit {}, frame={frame:02X?}",
                nibble,
                i + 1
            );
            return Err(Error::invalid_response(
                format!("ut8802 invalid BCD nibble {nibble:#04x} at digit {}", i + 1),
                frame,
            ));
        }
    }

    // Validate decimal point position (byte 5 low nibble, must be 0-4)
    let dp_pos = frame[5] & 0x0F;
    if dp_pos > 4 {
        trace!("framing: ut8802 invalid decimal point position {dp_pos}, frame={frame:02X?}");
        return Err(Error::invalid_response(
            format!("ut8802 invalid decimal point position {dp_pos}"),
            frame,
        ));
    }

    // Payload = bytes 1..8 (everything after the header)
    let payload = frame[1..UT8802_FRAME_LEN].to_vec();
    let consumed = start + UT8802_FRAME_LEN;

    trace!("framing: ut8802 valid frame, position={position:#04x}, consumed={consumed}");
    Ok(Some((payload, consumed)))
}

/// Build a valid UT8802 frame from components.
/// Frame: [0xAC, position, d1d2, d3d4, d5xx, dp_flags, status, sign]
#[cfg(test)]
pub(super) fn test_frame_ut8802(
    position: u8,
    digits: [u8; 5],
    dp_pos: u8,
    acdc_bits: u8,
    status: u8,
    sign_flags: u8,
) -> Vec<u8> {
    vec![
        0xAC,
        position,
        (digits[0] << 4) | digits[1],
        (digits[2] << 4) | digits[3],
        digits[4], // high nibble unused
        (acdc_bits << 4) | dp_pos,
        status,
        sign_flags,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::framing::{FrameErrorRecovery, read_frame};
    use crate::transport::mock::MockTransport;

    #[test]
    fn ut8802_valid_frame() {
        // DC V 200V range, display "12345", decimal pos 1
        let frame = test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let (payload, consumed) = extract_frame_ut8802(&frame).unwrap().unwrap();
        assert_eq!(consumed, 8);
        assert_eq!(payload.len(), 7); // bytes 1..8
        assert_eq!(payload[0], 0x05); // position code
    }

    #[test]
    fn ut8802_leading_garbage() {
        let mut buf = vec![0xFF, 0xFE, 0xFD];
        buf.extend_from_slice(&test_frame_ut8802(
            0x01,
            [0, 0, 2, 0, 0],
            3,
            0x02,
            0x00,
            0x00,
        ));
        let (payload, consumed) = extract_frame_ut8802(&buf).unwrap().unwrap();
        assert_eq!(consumed, 3 + 8); // 3 garbage bytes + 8 frame bytes
        assert_eq!(payload[0], 0x01);
    }

    #[test]
    fn ut8802_incomplete() {
        // Only 5 bytes after header — need 8 total
        let buf = vec![0xAC, 0x01, 0x12, 0x34, 0x05];
        assert!(extract_frame_ut8802(&buf).unwrap().is_none());
    }

    #[test]
    fn ut8802_no_header() {
        let buf = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert!(extract_frame_ut8802(&buf).unwrap().is_none());
    }

    #[test]
    fn ut8802_invalid_position_code() {
        // 0x02 is a gap in the position code space
        let frame = test_frame_ut8802(0x02, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
        assert!(extract_frame_ut8802(&frame).is_err());
    }

    #[test]
    fn ut8802_invalid_bcd_nibble() {
        // 0x0F is not a valid BCD nibble
        let frame = test_frame_ut8802(0x01, [0x0F, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
        assert!(extract_frame_ut8802(&frame).is_err());
    }

    #[test]
    fn ut8802_invalid_decimal_position() {
        // Decimal position 5 is out of range (max 4)
        let frame = test_frame_ut8802(0x01, [1, 2, 3, 4, 5], 5, 0x00, 0x00, 0x00);
        assert!(extract_frame_ut8802(&frame).is_err());
    }

    #[test]
    fn ut8802_overload_nibble_accepted() {
        // 0x0C is a valid BCD nibble (overload indicator 'L')
        let frame = test_frame_ut8802(0x01, [0, 0, 0, 0x0C, 0], 0, 0x00, 0x00, 0x00);
        let result = extract_frame_ut8802(&frame).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn ut8802_nibble_0a_accepted() {
        // 0x0A is treated as '0' — should be accepted
        let frame = test_frame_ut8802(0x01, [0x0A, 0, 0, 0, 0], 0, 0x00, 0x00, 0x00);
        let result = extract_frame_ut8802(&frame).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn ut8802_all_valid_positions() {
        let valid_positions: &[u8] = &[
            0x01, 0x03, 0x04, 0x05, 0x06, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x10, 0x11, 0x12,
            0x13, 0x14, 0x16, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1F, 0x22, 0x23, 0x24, 0x25,
            0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
        ];
        for &pos in valid_positions {
            let frame = test_frame_ut8802(pos, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
            assert!(
                extract_frame_ut8802(&frame).unwrap().is_some(),
                "position {pos:#04x} should be valid"
            );
        }
    }

    #[test]
    fn ut8802_invalid_positions() {
        let invalid_positions: &[u8] = &[
            0x00, 0x02, 0x07, 0x08, 0x0F, 0x15, 0x17, 0x1E, 0x20, 0x21, 0x26, 0x2E, 0xFF,
        ];
        for &pos in invalid_positions {
            let frame = test_frame_ut8802(pos, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
            assert!(
                extract_frame_ut8802(&frame).is_err(),
                "position {pos:#04x} should be invalid"
            );
        }
    }

    #[test]
    fn ut8802_false_header_in_garbage() {
        // Garbage contains a false 0xAC byte followed by an invalid position code,
        // then the real frame. The extractor should error on the false header;
        // read_frame's skip-and-retry should advance past it to the real frame.
        let false_frame = vec![0xAC, 0x00, 0x12, 0x34, 0x50, 0x01, 0x00, 0x00]; // pos 0x00 = invalid
        let real_frame = test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);

        // First: the extractor should error on the false frame
        assert!(extract_frame_ut8802(&false_frame).is_err());

        // Second: in a combined buffer, after the false frame the real one is found
        let mut combined = false_frame.clone();
        combined.extend_from_slice(&real_frame);
        let mock = MockTransport::new(vec![combined]);
        let mut rx_buf = Vec::new();

        let result = read_frame(
            &mut rx_buf,
            &mock,
            extract_frame_ut8802,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "test",
            &UT8802_HEADER,
        )
        .unwrap();
        assert_eq!(result[0], 0x05); // position code of the real frame
    }
}
