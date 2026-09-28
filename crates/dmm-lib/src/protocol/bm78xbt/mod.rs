//! The Brymen BM788BT and BM787BT, Bluetooth LE built in
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md`).
//!
//! The meter streams only once the host has logged in with its connection
//! password, which the Bluetooth transport does at bring-up with the packets
//! `login.rs` builds (spec §3). Then each notification carries an
//! information packet and one reading packet, numeric: a signed count, its
//! decimal point, a metric prefix, a unit and two function IDs (spec §4,
//! §6). Nothing on the wire tells the two models apart (spec §1), so one
//! registry entry serves both.
//!
//! - `packet.rs`: the CRC, and finding readings in the stream
//! - `login.rs`: the login's command packets, and judging its reply
//! - `tables.rs`: the function, unit, prefix and display-word tables
//! - `decode.rs`: packets → `Measurement`
//! - `capture.rs`: the capture steps
//! - `devices.rs`: the registry entry

/// The factory reset of the meter's connection password and Bluetooth name,
/// from the BM788BT manual p.20 (printed 19) (spec §9.2). It is the way out
/// of a refused password, which is the only one this driver sends (0000). A
/// macro, so the activation text and the refused-login error can `concat!`
/// it.
macro_rules! reset_gesture {
    () => {
        "hold the Hz button while turning the dial from OFF to capacitance within 0.6 s: \
         the meter shows \"Org\" and its connection password and Bluetooth name are back to \
         the factory settings"
    };
}

mod capture;
mod decode;
pub(crate) mod devices;
// The Bluetooth transport and the tests are its users.
#[cfg_attr(not(feature = "bluetooth"), allow(dead_code))]
pub(crate) mod login;
pub(crate) mod packet;
mod tables;

use crate::error::Result;
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Probing, Protocol,
    Stability,
};
use crate::transport::Transport;
use log::debug;

/// The registry id, which the report hint names as `--device`.
pub(crate) const ID: &str = "bm78xbt";

/// Log label for the read loop.
const LOG: &str = "bm78xbt";

/// Report what in packet `p` no spec section covers.
fn report(p: &[u8], what: &'static str) {
    report_unknown(ID, what, format_args!("packet {p:02X?}"));
}

/// The Brymen BM788BT and BM787BT.
pub(crate) struct Bm78xbtProtocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Bm78xbtProtocol {
    pub(crate) fn new() -> Self {
        Self {
            // One whole notification and the next's start (spec §4).
            rx_buf: Vec::with_capacity(512),
            profile: DeviceProfile {
                family_name: "BM78xBT",
                model_name: "Brymen BM788BT/BM787BT",
                stability: Stability::Experimental,
                // r4 has no key or function command (spec §7.1).
                supported_commands: &[],
                max_aux_values: 0,
                verification_issue: Some(33),
                meter_keys: MeterKeys::NONE,
            },
        }
    }
}

impl Protocol for Bm78xbtProtocol {
    fn delivery(&self) -> crate::protocol::Delivery {
        crate::protocol::Delivery::Streamed
    }

    fn discard_input(&mut self, transport: &dyn Transport) -> Result<()> {
        self.rx_buf.clear();
        crate::protocol::framing::discard_queued(transport)
    }

    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // The transport logged in before this runs (spec §3.1), and the
        // meter streams from then on; nothing is written here. The app's
        // clock set (spec §7.1) would change the meter's clock, and readings
        // are stamped by the host, so it is not sent.
        debug!("bm78xbt: init (listen only)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractor never fails, so the recovery mode and the skip
        // pattern are never used.
        let payload = framing::read_newest_frame(
            &mut self.rx_buf,
            transport,
            packet::extract,
            |_| true,
            FrameErrorRecovery::Propagate,
            LOG,
            &packet::READING_HEAD,
        )?;
        decode::decode(&payload)
    }

    /// `payload` is the information packet and the reading, 56 bytes, or a
    /// reading alone, 32 bytes, as the stream delivers them and a replay
    /// file stores them.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        decode::decode(payload)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        capture::steps()
    }
}

/// Detection for the BM78xBT: it streams once logged in, so nothing is sent.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Bm78xbt,
    label: "bm78xbt stream",
    trigger: None,
    send_after: &[],
    // A CRC-16 over 26 bytes (spec §4).
    checksummed: true,
    recognise,
};

/// One reading packet anywhere in `buf` whose CRC holds and that says it is
/// a meter's.
fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    packet::first_meter_reading(buf).map(|_| Evidence::Model {
        id: ID,
        reported_name: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::registry;
    use crate::transport::mock::MockTransport;
    use packet::tests::{
        example_info, example_notification, example_reading, notification, sealed,
    };

    fn proto() -> Bm78xbtProtocol {
        Bm78xbtProtocol::new()
    }

    /// A second reading, 12.345 V DC, so two readings in a row tell apart.
    fn second_reading() -> Vec<u8> {
        let mut r = example_reading();
        r[15] = 0x00;
        r[21..24].copy_from_slice(&12345i32.to_le_bytes()[..3]);
        r[24] = 2;
        sealed(r)
    }

    /// Two notifications, as the meter sends them.
    fn stream() -> Vec<u8> {
        [
            example_notification(),
            notification(&example_info(), &second_reading()),
        ]
        .concat()
    }

    /// `bytes` in reads of `size`.
    fn pieces(bytes: &[u8], size: usize) -> Vec<Vec<u8>> {
        bytes.chunks(size).map(<[u8]>::to_vec).collect()
    }

    /// Whole notifications, and the same split into 5-, 20- and 64-byte
    /// pieces (a small MTU, the transport's reads), give their readings;
    /// with both already queued, a request answers with the newer one.
    #[test]
    fn notifications_in_any_pieces_give_their_readings() {
        let later = notification(&example_info(), &second_reading());
        for size in [152, 5, 20, 64] {
            let mock = MockTransport::new(pieces(&example_notification(), size));
            let mut proto = proto();
            let (first, reports) = capture_reports(|| proto.request_measurement(&mock));
            assert!(reports.is_empty(), "{reports:?}");
            let first = first.unwrap();
            assert_eq!(first.display_raw.as_deref(), Some("-1.2345"), "{size}");
            assert_eq!(first.raw_payload.len(), 56, "{size}");
            for piece in pieces(&later, size) {
                mock.push_response(piece);
            }
            let second = proto.request_measurement(&mock).unwrap();
            assert_eq!(second.display_raw.as_deref(), Some("12.345"), "{size}");
            assert_eq!(second.raw_payload.len(), 56, "{size}");

            let mock = MockTransport::new(pieces(&stream(), size));
            let newest = self::proto().request_measurement(&mock).unwrap();
            assert_eq!(newest.display_raw.as_deref(), Some("12.345"), "{size}");
        }
    }

    /// Joining mid-notification, past the information packet, the first
    /// reading comes alone, silently; the next has its information packet.
    #[test]
    fn joining_mid_stream_reads_the_next_whole_reading() {
        let stream = stream();
        let whole = example_notification().len();
        for skip in [3, 30, 60] {
            // What is queued when the read comes: the rest of the first
            // notification, and while it holds no reading, the next one too.
            let queued = if skip < 24 { whole } else { stream.len() };
            let mock = MockTransport::new(pieces(&stream[skip..queued], 20));
            let mut proto = proto();
            let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
            assert!(reports.is_empty(), "{reports:?}");
            let m = m.unwrap();
            if skip < 24 {
                assert_eq!(m.raw_payload, example_reading(), "{skip}");
                for piece in pieces(&stream[whole..], 20) {
                    mock.push_response(piece);
                }
                let next = proto.request_measurement(&mock).unwrap();
                assert_eq!(next.raw_payload[24..], second_reading(), "{skip}");
            } else {
                // The first reading was cut: the second is the first read.
                assert_eq!(m.raw_payload[24..], second_reading(), "{skip}");
            }
        }
    }

    #[test]
    fn a_bad_crc_is_skipped_for_the_next_reading() {
        let mut stream = stream();
        stream[24 + 22] ^= 0x01;
        let mock = MockTransport::new(stream.chunks(20).map(<[u8]>::to_vec).collect());
        let m = proto().request_measurement(&mock).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.345"));
    }

    #[test]
    fn init_writes_nothing() {
        let mock = MockTransport::new(Vec::new());
        proto().init(&mock).unwrap();
        assert!(mock.written.borrow().is_empty());
        assert!(mock.bauds.borrow().is_empty());
    }

    #[test]
    fn no_command_is_offered() {
        let mock = MockTransport::new(Vec::new());
        let mut proto = proto();
        for command in ["hold", "range", "rel", ""] {
            assert!(proto.send_command(&mock, command).is_err(), "{command}");
        }
        assert!(mock.written.borrow().is_empty());
        assert!(proto.profile().supported_commands.is_empty());
    }

    #[test]
    fn the_registry_entry_is_this_familys() {
        let entry = registry::find_device(ID).expect("registry entry");
        assert_eq!(entry.family, DeviceFamily::Bm78xbt);
        assert_eq!(entry.display_name, "Brymen BM788BT/BM787BT");
        assert_eq!(entry.bluetooth_names, ["BM78xBT"]);
        assert_eq!(entry.links, [crate::BLUETOOTH]);
        for alias in ["bm788bt", "BM787BT", "brymen-bm788bt", "brymen-bm787bt"] {
            assert_eq!(
                registry::resolve_device(alias).map(|d| d.id),
                Some(ID),
                "{alias}"
            );
        }
        let proto = (entry.new_protocol)();
        assert_eq!(proto.profile().model_name, entry.display_name);
        assert_eq!(proto.profile().max_aux_values, 0);
        assert_eq!(proto.profile().verification_issue, Some(33));
        assert!(std::ptr::eq(entry.fingerprint.unwrap(), &FINGERPRINT));
        assert!(
            entry.activation_instructions.contains(reset_gesture!()),
            "{}",
            entry.activation_instructions
        );
    }

    fn found(buf: &[u8]) -> bool {
        recognise(buf, &Probing::default()).is_some()
    }

    #[test]
    fn recognise_takes_a_meters_reading() {
        assert_eq!(
            recognise(&example_notification(), &Probing::default()),
            Some(Evidence::Model {
                id: ID,
                reported_name: None
            })
        );
        assert!(found(&example_reading()));
        assert!(found(&stream()[40..]));
        let mut junk_first = vec![0x00, 0xFF, 0x02, 0x20];
        junk_first.extend(example_notification());
        assert!(found(&junk_first));
    }

    #[test]
    fn recognise_declines_what_is_not_a_meters_reading() {
        let reading = example_reading();
        assert!(!found(&reading[..31]));
        let mut bad = reading.clone();
        bad[21] ^= 0x01;
        assert!(!found(&bad));
        let mut sensor = reading.clone();
        sensor[17] = 0x00;
        assert!(!found(&sealed(sensor)));
        assert!(!found(&example_info()));
        assert!(!found(&[0u8; 152]));
    }

    /// Other meters' bytes on the same Bluetooth links: the UT61+ ack and
    /// name frames the detection tests use, the UT-D07B's heartbeat
    /// (`transport/ble/issc.rs`), ZOTEK's worked examples as they arrive on
    /// air (ZOTEK spec §9), and the 121GW's worked example 1 (121GW spec
    /// §13), several times over.
    #[test]
    fn recognise_declines_other_meters() {
        use crate::protocol::framing::test_frame_be16;
        let mut ut61 = test_frame_be16(&[0xFF, 0x00]);
        ut61.extend(test_frame_be16(b"UT61E+"));
        assert!(!found(&ut61));
        let heartbeat = [0xAB, 0xCD, 0x06, 0xAA, 0xAA, 0x6E, 0x67, 0x03, 0xA7];
        assert!(!found(&[heartbeat; 20].concat()));
        let zotek: [&[u8]; 4] = [
            &[
                0x1B, 0x84, 0x70, 0x41, 0x08, 0x5C, 0x7D, 0x7F, 0x66, 0xFA, 0x3A,
            ],
            &[0x1B, 0x84, 0x72, 0xF3, 0x2F, 0x2E, 0xE9, 0xD6, 0x66, 0xAA],
            &[0x1B, 0x84, 0x71, 0x15, 0x3C, 0x2B, 0xD9, 0xFA, 0x66, 0xA9],
            &[
                0x1B, 0x84, 0x77, 0x51, 0xF2, 0x2A, 0xC9, 0x9A, 0xA1, 0x6D, 0x75, 0x5F, 0x5F, 0xA6,
                0x33, 0x14, 0x20, 0x1A, 0xAA,
            ],
        ];
        assert!(!found(&[zotek.concat(), zotek.concat()].concat()));
        let gw121: [u8; 19] = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x01, 0x00, 0x30, 0x39, 0x64, 0x01, 0x00, 0xF3, 0x00,
            0x00, 0x0C, 0x40, 0x00, 0x35,
        ];
        assert!(!found(&[gw121; 8].concat()));
    }
}
