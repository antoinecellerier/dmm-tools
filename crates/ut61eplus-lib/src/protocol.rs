use crate::error::{Error, Result};
use log::{debug, trace};

/// Header bytes for all messages.
pub const HEADER: [u8; 2] = [0xAB, 0xCD];

/// Minimum valid response length: header(2) + length(1) + checksum(2) = 5
/// (length byte value must be >= 2 to hold at least the checksum)
const MIN_RESPONSE_LEN: usize = 5;

/// Expected payload length for a measurement response.
pub const MEASUREMENT_PAYLOAD_LEN: usize = 14;

/// Find a complete framed response in the buffer.
///
/// Scans for `AB CD` header, reads the length byte, validates the checksum,
/// and returns the payload bytes (excluding header, length, and checksum).
///
/// Returns `Ok(Some((payload, consumed)))` if a valid frame is found,
/// where `consumed` is how many bytes to drain from the buffer.
/// Returns `Ok(None)` if the buffer doesn't yet contain a complete frame.
/// Returns `Err` if a frame is found but the checksum is invalid.
pub fn extract_frame(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    // Scan for header
    let Some(start) = buf.windows(2).position(|w| w == HEADER) else {
        return Ok(None);
    };

    let remaining = &buf[start..];
    if remaining.len() < MIN_RESPONSE_LEN {
        return Ok(None);
    }

    // Byte after header is the "length" — counts everything after itself,
    // i.e. payload + 2-byte checksum. Verified against real device traces.
    let len_byte = remaining[2] as usize;
    if len_byte < 2 {
        return Ok(None); // Need at least 2 bytes for checksum
    }
    let frame_len = 2 + 1 + len_byte; // header + len_byte + (payload + checksum)
    let payload_len = len_byte - 2; // subtract the 2 checksum bytes

    if remaining.len() < frame_len {
        return Ok(None);
    }

    let frame = &remaining[..frame_len];
    trace!("protocol: raw frame: {:02X?}", frame);

    // Checksum: 16-bit BE sum of all bytes except the last two
    let data_bytes = &frame[..frame_len - 2];
    let computed: u16 = data_bytes.iter().map(|&b| b as u16).sum();
    let received = u16::from_be_bytes([frame[frame_len - 2], frame[frame_len - 1]]);

    if computed != received {
        debug!("protocol: checksum mismatch: computed={computed:#06x}, received={received:#06x}");
        return Err(Error::ChecksumMismatch {
            expected: received,
            actual: computed,
        });
    }

    let payload = frame[3..3 + payload_len].to_vec();
    let consumed = start + frame_len;

    debug!("protocol: valid frame, payload_len={payload_len}, consumed={consumed}");
    Ok(Some((payload, consumed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid frame from a payload.
    /// Length byte = payload.len() + 2 (includes the checksum).
    fn make_frame(payload: &[u8]) -> Vec<u8> {
        let len_byte = (payload.len() + 2) as u8; // payload + 2 checksum bytes
        let mut frame = vec![0xAB, 0xCD, len_byte];
        frame.extend_from_slice(payload);
        let sum: u16 = frame.iter().map(|&b| b as u16).sum();
        frame.push((sum >> 8) as u8);
        frame.push((sum & 0xFF) as u8);
        frame
    }

    #[test]
    fn extract_valid_frame() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = make_frame(&payload);
        let result = extract_frame(&frame).unwrap().unwrap();
        assert_eq!(result.0, payload);
        assert_eq!(result.1, frame.len());
    }

    #[test]
    fn extract_with_leading_garbage() {
        let payload = vec![0x01, 0x02, 0x03];
        let frame = make_frame(&payload);
        let mut buf = vec![0xFF, 0xFE, 0xFD];
        buf.extend_from_slice(&frame);
        let result = extract_frame(&buf).unwrap().unwrap();
        assert_eq!(result.0, payload);
        assert_eq!(result.1, 3 + frame.len()); // garbage + frame
    }

    #[test]
    fn extract_incomplete() {
        let frame = vec![0xAB, 0xCD, 0x03, 0x01]; // incomplete
        assert!(extract_frame(&frame).unwrap().is_none());
    }

    #[test]
    fn extract_bad_checksum() {
        let mut frame = make_frame(&[0x01, 0x02, 0x03]);
        let last = frame.len() - 1;
        frame[last] ^= 0xFF; // corrupt checksum
        assert!(extract_frame(&frame).is_err());
    }

    #[test]
    fn extract_no_header() {
        let buf = vec![0x00, 0x01, 0x02, 0x03];
        assert!(extract_frame(&buf).unwrap().is_none());
    }

    #[test]
    fn extract_real_device_frame() {
        // Real frame captured from UT61E+ on DC mV mode, reading " 0.0004"
        let frame = vec![
            0xAB, 0xCD, 0x10, // header + length (16 = 14 payload + 2 checksum)
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, // mode, range, display
            0x00, 0x02, // progress (no 0x30 prefix on these)
            0x30, 0x30, 0x30, // flags
            0x03, 0x8E, // checksum
        ];
        let (payload, consumed) = extract_frame(&frame).unwrap().unwrap();
        assert_eq!(consumed, 19);
        assert_eq!(payload.len(), 14);
        assert_eq!(payload[0], 0x02); // mode byte is raw (no 0x30 prefix)
        assert_eq!(payload[1] & 0x0F, 0x00); // range: 0
        assert_eq!(&payload[2..9], b" 0.0004");
    }
}
