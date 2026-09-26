//! The EEVblog 121GW, Bluetooth LE built in
//! (`docs/research/121gw/reverse-engineered-protocol.md`).
//!
//! The meter streams a 19-byte binary packet about twice a second once a
//! client subscribes (spec §3, §15.4), carrying the main display, the
//! secondary display, the bar graph and the annunciators. Keys go the other
//! way as short ASCII-hex frames behind `F4` (spec §11.1).
//!
//! - `packet.rs`: finding packets in the byte stream
//! - `tables.rs`: the mode and range tables
//! - `decode.rs`: packet → `Measurement`
//! - `capture.rs`: the capture steps
//! - `devices.rs`: the registry entry

mod capture;
mod decode;
pub(crate) mod devices;
mod packet;
mod tables;

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKey, MeterKeys, Probing,
    Protocol, Stability,
};
use crate::transport::Transport;
use log::debug;

/// The registry id, which the report hint names as `--device`.
pub(super) const ID: &str = "121gw";

/// Log label for the read loop.
const LOG: &str = "121gw";

/// Report what in packet `p` no spec section covers.
fn report(p: &[u8], what: &'static str) {
    report_unknown(ID, what, format_args!("packet {p:02X?}"));
}

/// What a read that timed out on an older firmware's ASCII packets says
/// (spec §12): the owner can update the firmware from the SD card (manual
/// p.72), and current firmware sends the binary packet.
const OLDER_FIRMWARE: &str = "the 121GW sends the data format of older firmware, which this tool \
     does not read; update the meter's firmware from its SD card (121GW manual p.72)";

/// The remote keys, by command: the code a short press sends, or with bit
/// 7 set a long one (spec §11.1). EEVblog's app sends range, hold, rel and
/// select; UEi's app sends every code (spec §11.1), of which these are the
/// ones the manual documents and that leave the link and the SD card
/// alone. Never offered: long 1ms PEAK (`84`) switches Bluetooth off,
/// MEM and SETUP (`07`, `87`, `08`, `88`) reach memory, logging and
/// menus, long RANGE and HOLD (`81`, `82`) do nothing the manual names,
/// and the buzzer (`09`) is UEi's alone.
const KEYS: [(&str, u8); 9] = [
    ("range", 0x01),
    ("hold", 0x02),
    ("rel", 0x03),
    ("select", 0x05),
    ("minmax", 0x06),
    // Long MIN/MAX leaves MIN/MAX (manual p.58).
    ("exit_minmax", 0x86),
    // Short 1ms PEAK: 1 ms peak capture in AC V (manual p.33).
    ("peak", 0x04),
    // Long MODE: the backlight (manual p.33).
    ("light", 0x85),
    // Long REL in an AC mode: the 1 kHz low-pass filter (manual p.33).
    ("lpf", 0x83),
];

/// [`KEYS`]' commands, for the profile.
const COMMANDS: [&str; KEYS.len()] = {
    let mut commands = [""; KEYS.len()];
    let mut i = 0;
    while i < KEYS.len() {
        commands[i] = KEYS[i].0;
        i += 1;
    }
    commands
};

/// The key frame for `code`: `F4`, then the code twice as upper-case ASCII
/// hex (spec §11.1).
fn key_frame(code: u8) -> [u8; 5] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let hi = HEX[usize::from(code >> 4)];
    let lo = HEX[usize::from(code & 0x0F)];
    [0xF4, hi, lo, hi, lo]
}

/// The 1 kHz filter key, offered while the reading is in an AC mode the
/// manual names it for.
static LPF_KEY: MeterKey = MeterKey {
    command: "lpf",
    label: "1kHz",
    // A long REL, not a key of its own (manual p.33).
    hover: Some("Hold the meter's REL key (1 kHz low-pass filter)"),
    applies: |m| {
        u8::try_from(m.mode_raw).is_ok_and(|mode| tables::lpf_name(mode).is_some())
            && m.mode != "AC+DC V"
    },
};

/// The EEVblog 121GW.
pub(crate) struct Eevblog121gwProtocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Eevblog121gwProtocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(64),
            profile: DeviceProfile {
                family_name: "121GW",
                model_name: "EEVblog 121GW",
                stability: Stability::Experimental,
                supported_commands: &COMMANDS,
                // The secondary display, or in the VA modes its two
                // operands and then whatever else it shows (see
                // `decode::secondary`).
                max_aux_values: 3,
                verification_issue: None,
                meter_keys: MeterKeys {
                    functions: &[],
                    context: std::slice::from_ref(&LPF_KEY),
                },
            },
        }
    }
}

impl Protocol for Eevblog121gwProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // The meter streams once subscribed; nothing is written first
        // (spec §3). UEi's app sends a clock set, which would change the
        // meter's clock, so it is not sent.
        debug!("121gw: init (listen only)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractor never fails, so the recovery mode and the skip
        // pattern are never used. About 2 packets a second arrive (spec
        // §15.4), several inside read_frame's 2 s.
        let read = framing::read_frame(
            &mut self.rx_buf,
            transport,
            packet::extract_packet,
            |_| true,
            FrameErrorRecovery::Propagate,
            LOG,
            &[packet::START],
        );
        match read {
            Ok(p) => decode::decode(&p),
            Err(Error::Timeout) if packet::holds_ascii_packet(&self.rx_buf) => {
                let err = Error::invalid_response(OLDER_FIRMWARE, &self.rx_buf);
                self.rx_buf.clear();
                Err(err)
            }
            Err(e) => Err(e),
        }
    }

    /// `payload` is one 19-byte packet, `F2` first, as the stream delivers
    /// it and a replay file stores it.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        decode::decode(payload)
    }

    /// Press a remote key: one frame, no reply awaited. The stream shows
    /// what the key did (spec §11.1).
    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let Some(&(_, code)) = KEYS.iter().find(|(name, _)| *name == command) else {
            return Err(Error::UnsupportedCommand(command.to_string()));
        };
        let frame = key_frame(code);
        debug!("121gw: key {command} ({code:02X}), frame {frame:02X?}");
        transport.write(&frame)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        capture::steps()
    }
}

/// Detection for the 121GW: it streams unprompted, so nothing is sent.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Eevblog121gw,
    label: "121gw stream",
    trigger: None,
    send_after: &[],
    // The extractor checks an 8-bit XOR, which a chance window passes one
    // time in 256: too weak to rank with a 16-bit checksum
    // (docs/detection-design.md).
    checksummed: false,
    recognise,
};

/// One packet anywhere in `buf` whose XOR holds and whose mode and range
/// decode, with no reserved bit set: stricter than the extractor, which
/// reads a packet with a reserved bit and reports it.
fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    packet::first_decodable(buf).map(|_| Evidence::Model {
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
    use packet::tests::{COMMUNITY, EXAMPLES, all_packets, ascii_stream};

    fn proto() -> Eevblog121gwProtocol {
        Eevblog121gwProtocol::new()
    }

    #[test]
    fn f2_then_the_body_as_two_values_gives_two_readings() {
        let mut reads = Vec::new();
        for p in [EXAMPLES[0], EXAMPLES[3]] {
            reads.push(vec![packet::START]);
            reads.push(p[1..].to_vec());
        }
        let mock = MockTransport::new(reads);
        let mut proto = proto();
        assert_eq!(
            proto.request_measurement(&mock).unwrap().raw_payload,
            EXAMPLES[0]
        );
        assert_eq!(
            proto.request_measurement(&mock).unwrap().raw_payload,
            EXAMPLES[3]
        );
    }

    /// The shape community clients saw on a meter (spec §15.4): 18-byte
    /// values, no `F2`.
    #[test]
    fn eighteen_byte_values_without_f2_give_readings() {
        let reads: Vec<Vec<u8>> = all_packets().iter().map(|p| p[1..].to_vec()).collect();
        let mock = MockTransport::new(reads);
        let mut proto = proto();
        for p in all_packets() {
            let m = proto.request_measurement(&mock).unwrap();
            assert_eq!(m.raw_payload, p, "stored with F2 first");
        }
    }

    #[test]
    fn joining_mid_packet_reads_the_next_whole_one() {
        let mut stream = EXAMPLES[1][7..].to_vec();
        stream.extend_from_slice(&EXAMPLES[2]);
        stream.extend_from_slice(&EXAMPLES[0][1..]);
        let reads: Vec<Vec<u8>> = stream.chunks(5).map(<[u8]>::to_vec).collect();
        let mock = MockTransport::new(reads);
        let mut proto = proto();
        let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().raw_payload, EXAMPLES[2]);
        assert!(reports.is_empty(), "{reports:?}");
        assert_eq!(
            proto.request_measurement(&mock).unwrap().raw_payload,
            EXAMPLES[0]
        );
    }

    /// A meter whose packets the tables do not cover, joined mid-packet:
    /// two back to back bring the read in step, and each packet reads and
    /// is reported rather than the reads timing out.
    #[test]
    fn off_table_packets_joined_mid_packet_read_and_report() {
        let odd = packet::tests::off_table();
        let mut stream = odd[7..].to_vec();
        for _ in 0..3 {
            stream.extend_from_slice(&odd);
        }
        let reads: Vec<Vec<u8>> = stream.chunks(5).map(<[u8]>::to_vec).collect();
        let mock = MockTransport::new(reads);
        let mut proto = proto();
        for _ in 0..3 {
            let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
            assert_eq!(m.unwrap().raw_payload, odd);
            assert!(
                reports.iter().any(|r| r.contains("unrecognised mode code")),
                "{reports:?}"
            );
        }
    }

    /// Older firmware's ASCII packets never read as a packet: the read
    /// times out and says to update the firmware, and reports nothing.
    #[test]
    fn older_firmware_says_to_update_it() {
        let mut stream = ascii_stream();
        stream.extend(ascii_stream());
        let mock = MockTransport::new(vec![stream]);
        let mut proto = proto();
        let (result, reports) = capture_reports(|| proto.request_measurement(&mock));
        let err = result.unwrap_err();
        assert!(matches!(err, Error::InvalidResponse { .. }), "{err:?}");
        assert!(
            err.to_string().contains("update the meter's firmware"),
            "{err}"
        );
        assert!(reports.is_empty(), "{reports:?}");
        assert!(proto.rx_buf.is_empty());

        // Nothing at all is a plain timeout.
        let silent = MockTransport::new(Vec::new());
        assert!(matches!(
            proto.request_measurement(&silent),
            Err(Error::Timeout)
        ));
    }

    #[test]
    fn init_writes_nothing() {
        let mock = MockTransport::new(Vec::new());
        proto().init(&mock).unwrap();
        assert!(mock.written.borrow().is_empty());
        assert!(mock.feature_reports.borrow().is_empty());
    }

    /// A key is one frame, written without waiting for anything: the mock
    /// has nothing to read. The frames are spec §13's and §11.1's.
    #[test]
    fn a_key_is_one_write_and_no_read() {
        for (command, frame) in [
            ("range", [0xF4, 0x30, 0x31, 0x30, 0x31]),
            ("exit_minmax", [0xF4, 0x38, 0x36, 0x38, 0x36]),
            ("lpf", [0xF4, 0x38, 0x33, 0x38, 0x33]),
            ("light", [0xF4, 0x38, 0x35, 0x38, 0x35]),
        ] {
            let mock = MockTransport::new(Vec::new());
            proto().send_command(&mock, command).unwrap();
            assert_eq!(*mock.written.borrow(), [frame], "{command}");
        }
        assert_eq!(key_frame(0x82), [0xF4, 0x38, 0x32, 0x38, 0x32]);
    }

    /// Long 1ms PEAK switches Bluetooth off (manual p.33), which would cut
    /// the link the key went over.
    #[test]
    fn no_key_switches_bluetooth_off() {
        assert!(KEYS.iter().all(|&(_, code)| code != 0x84));
        let codes: Vec<u8> = KEYS.iter().map(|&(_, code)| code).collect();
        for never in [0x07, 0x87, 0x08, 0x88, 0x81, 0x82, 0x09] {
            assert!(!codes.contains(&never), "{never:#04x}");
        }
    }

    #[test]
    fn other_commands_are_refused_unsent() {
        let mock = MockTransport::new(Vec::new());
        let mut proto = proto();
        for command in ["auto", "exit_peak", "save", "mem", "setup", ""] {
            let err = proto.send_command(&mock, command).unwrap_err();
            assert!(matches!(err, Error::UnsupportedCommand(_)), "{err:?}");
        }
        assert!(mock.written.borrow().is_empty());
        assert_eq!(proto.profile().supported_commands, COMMANDS);
    }

    #[test]
    fn the_filter_key_shows_in_the_ac_modes_it_applies_to() {
        let decode = |mode: u8, coupling: u8| {
            let mut p = EXAMPLES[0];
            p[5] = mode;
            p[15] = coupling << 3;
            decode::decode(&packet::tests::sealed(p)).unwrap()
        };
        for mode in [2, 4, 16, 18, 20] {
            assert!((LPF_KEY.applies)(&decode(mode, 2)), "mode {mode}");
        }
        for mode in [1, 3, 5, 17, 19, 21, 13, 15] {
            assert!(!(LPF_KEY.applies)(&decode(mode, 1)), "mode {mode}");
        }
        assert!(!(LPF_KEY.applies)(&decode(2, 3)), "AC+DC V");
    }

    #[test]
    fn the_registry_entry_is_this_familys() {
        let entry = registry::find_device(ID).expect("registry entry");
        assert_eq!(entry.family, DeviceFamily::Eevblog121gw);
        assert_eq!(entry.display_name, "EEVblog 121GW");
        let proto = (entry.new_protocol)();
        assert_eq!(proto.profile().model_name, entry.display_name);
        assert!(std::ptr::eq(entry.fingerprint.unwrap(), &FINGERPRINT));
    }

    fn found(buf: &[u8]) -> bool {
        recognise(buf, &Probing::default()).is_some()
    }

    #[test]
    fn recognise_takes_the_packets_in_either_shape() {
        for p in all_packets() {
            assert_eq!(
                recognise(&p, &Probing::default()),
                Some(Evidence::Model {
                    id: ID,
                    reported_name: None
                })
            );
            for form in [&p[..], &p[1..]] {
                let mut after_junk = vec![0x00, 0x13, 0xFF, 0x42, 0x07];
                after_junk.extend_from_slice(form);
                assert!(found(&after_junk));
                let mut after_false_start = vec![packet::START, 0x01, 0x02];
                after_false_start.extend_from_slice(form);
                assert!(found(&after_false_start));
            }
        }
        assert!(found(&COMMUNITY[1][1..]), "121gwcli's 18-byte value");
    }

    #[test]
    fn recognise_declines_what_is_not_a_packet() {
        let mut truncated = EXAMPLES[0].to_vec();
        truncated.pop();
        assert!(!found(&truncated));
        let mut bad = EXAMPLES[0];
        bad[18] ^= 0x01;
        assert!(!found(&bad));
        let mut mode = EXAMPLES[0];
        mode[5] = 25;
        assert!(!found(&packet::tests::sealed(mode)));
        let mut range = EXAMPLES[0];
        range[6] = 4;
        assert!(!found(&packet::tests::sealed(range)));
        let mut ascii = ascii_stream();
        ascii.truncate(55);
        assert!(!found(&ascii));
    }

    /// Other meters' bytes on the same Bluetooth links: the UT61+ ack and
    /// name frames the detection tests use, the UT-D07B's heartbeat
    /// (`transport/ble/issc.rs`), and ZOTEK's four worked examples as they
    /// arrive on air (ZOTEK spec §9), twice over; the last carries `F2`.
    #[test]
    fn recognise_declines_other_meters() {
        use crate::protocol::framing::test_frame_be16;
        let mut ut61 = test_frame_be16(&[0xFF, 0x00]);
        ut61.extend(test_frame_be16(b"UT61E+"));
        assert!(!found(&ut61));
        let heartbeat = [0xAB, 0xCD, 0x06, 0xAA, 0xAA, 0x6E, 0x67, 0x03, 0xA7];
        assert!(!found(&[heartbeat, heartbeat, heartbeat].concat()));
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
        let twice = [zotek.concat(), zotek.concat()].concat();
        assert!(!found(&twice));
    }
}
