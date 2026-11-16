//! How a HID report in hidapi's form goes over plain USB transfers, for
//! `hid_android.rs`: the same bytes the kernel's hidraw puts on the wire, so
//! the bridges see on Android what they see on Linux. Kept apart from the
//! USB calls so that it is tested where the tests run.

/// `wValue`'s high byte in `SET_REPORT` and `GET_REPORT`: the report type.
pub(crate) const OUTPUT_REPORT: u16 = 0x0200;
pub(crate) const FEATURE_REPORT: u16 = 0x0300;

/// `wValue` for a report of `kind` whose buffer is `report`: the type and
/// the report ID, which hidapi keeps in the first byte.
pub(crate) fn report_value(kind: u16, report: &[u8]) -> u16 {
    kind | u16::from(report.first().copied().unwrap_or(0))
}

/// What of `report` is sent. hidapi's buffers start with the report ID, and
/// an ID of zero says the device numbers no reports: the byte is not sent.
/// A numbered report goes out whole, its ID first.
pub(crate) fn wire_bytes(report: &[u8]) -> &[u8] {
    match report {
        [0, rest @ ..] => rest,
        whole => whole,
    }
}

/// How many bytes to ask the device for to fill `buf` with a feature report:
/// an unnumbered one leaves the first byte to the zero ID.
pub(crate) fn feature_request_len(buf: &[u8]) -> u16 {
    let wanted = match buf {
        [0, rest @ ..] => rest.len(),
        whole => whole.len(),
    };
    u16::try_from(wanted).unwrap_or(u16::MAX)
}

/// Put the feature report the device `sent` into `buf`, whose first byte
/// holds the ID that was asked for, and return the length hidapi reports.
/// A numbered report arrives with its ID first and replaces the buffer from
/// the start; an unnumbered one goes after the zero ID. What does not fit is
/// dropped.
pub(crate) fn place_feature(buf: &mut [u8], sent: &[u8]) -> usize {
    let start = usize::from(buf.first() == Some(&0));
    let room = buf.len().saturating_sub(start);
    let n = sent.len().min(room);
    buf[start..start + n].copy_from_slice(&sent[..n]);
    start + n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_report_id_is_not_sent() {
        // CH9329, CH9325 and BU-86X: unnumbered reports.
        assert_eq!(wire_bytes(&[0x00, 0x03, 0xAB, 0xCD]), [0x03, 0xAB, 0xCD]);
        // Only the first zero is the ID.
        assert_eq!(wire_bytes(&[0x00, 0x00, 0x01]), [0x00, 0x01]);
    }

    #[test]
    fn a_numbered_report_is_sent_whole() {
        // CP2110: the ID of a data report is its payload length.
        assert_eq!(wire_bytes(&[0x02, 0xAB, 0xCD]), [0x02, 0xAB, 0xCD]);
        assert_eq!(wire_bytes(&[0x41, 0x01]), [0x41, 0x01]);
    }

    #[test]
    fn a_report_of_the_zero_id_alone_sends_nothing() {
        assert!(wire_bytes(&[0x00]).is_empty());
        assert!(wire_bytes(&[]).is_empty());
    }

    #[test]
    fn report_value_carries_type_and_id() {
        assert_eq!(report_value(FEATURE_REPORT, &[0x41, 0x01]), 0x0341);
        assert_eq!(report_value(FEATURE_REPORT, &[0x00, 0x25]), 0x0300);
        assert_eq!(report_value(OUTPUT_REPORT, &[0x00, 0x01]), 0x0200);
        assert_eq!(report_value(OUTPUT_REPORT, &[]), 0x0200);
    }

    #[test]
    fn a_numbered_feature_report_fills_the_buffer_from_its_id() {
        // CP2110 version report: 3 bytes asked, the ID comes back first.
        let mut buf = [0x46, 0, 0];
        assert_eq!(feature_request_len(&buf), 3);
        assert_eq!(place_feature(&mut buf, &[0x46, 0x0A, 0x01]), 3);
        assert_eq!(buf, [0x46, 0x0A, 0x01]);
    }

    #[test]
    fn an_unnumbered_feature_report_goes_after_the_zero_id() {
        let mut buf = [0u8; 5];
        assert_eq!(feature_request_len(&buf), 4);
        assert_eq!(place_feature(&mut buf, &[0xA0, 0xB1]), 3);
        assert_eq!(buf, [0x00, 0xA0, 0xB1, 0, 0]);
    }

    #[test]
    fn a_feature_report_longer_than_the_buffer_is_cut() {
        let mut numbered = [0x42, 0];
        assert_eq!(place_feature(&mut numbered, &[0x42, 1, 2, 3]), 2);
        assert_eq!(numbered, [0x42, 1]);

        let mut unnumbered = [0u8; 2];
        assert_eq!(place_feature(&mut unnumbered, &[7, 8, 9]), 2);
        assert_eq!(unnumbered, [0, 7]);

        let mut empty: [u8; 0] = [];
        assert_eq!(feature_request_len(&empty), 0);
        assert_eq!(place_feature(&mut empty, &[1]), 0);
    }
}
