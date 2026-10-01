//! OWON's Bluetooth LE meters: the B33, B35T+, B41T+, OW16B, OW18B, OW18E
//! and CM2100B (`docs/research/owon/reverse-engineered-protocol.md`).
//!
//! The meter notifies 6-byte frames on FFF4 unprompted once connected, three
//! little-endian words with no header or checksum (spec §5, §6). The model
//! is FFF2 byte 0, which the Bluetooth transport reads at bring-up and hands
//! on as [`Transport::info_characteristic`] (spec §1, §4): the frames are the
//! same on every model, but function 13, the RMR bit and the remote keys are
//! not, so `init` checks the code against the entry opened, and detection
//! picks the entry by it. Keys are two bytes on FFF3 (spec §7.1).
//!
//! - `model.rs`: the six models, by model code, and FFF2's value
//! - `frame.rs`: finding frames in the byte stream
//! - `decode.rs`: frame → `Measurement`
//! - `keys.rs`: the remote keys each model offers, and their bytes
//! - `capture.rs`: the capture steps per model
//! - `devices.rs`: the registry entries

mod capture;
mod decode;
pub(crate) mod devices;
pub(crate) mod frame;
mod keys;
mod model;

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Stability,
};
use crate::transport::Transport;
use log::{debug, warn};
use model::{FALLBACK, Fff2, Model, Record};

/// Log label for the read loop.
const LOG: &str = "owon";

/// What a read says when the meter sends bytes but never a frame: the
/// older B35 and B35T send a 14-byte ASCII frame (spec §11, §14.4).
const NO_MARKER: &str = "the meter sends no OWON 6-byte frames; it may be an older B35 or B35T, \
     whose 14-byte frames this tool does not read yet";

/// The most received bytes a no-marker report and error carry.
const SHOWN_BYTES: usize = 64;

/// What `init` made of FFF2's model code, which decides whether keys go
/// out: a key meant for one model can switch another's Bluetooth (spec
/// §7.1, §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    /// The code is the entry's.
    Matches,
    /// The code is another entry's.
    Other(&'static Model),
    /// No entry carries the code.
    Unknown(u8),
    /// The link read no model code: keys follow the entry opened. Under
    /// auto-detect that entry is detection's B35T+ guess from two frames,
    /// so the keys may go to another OWON model. That is safe because no
    /// model's key table holds a long press of 04 or 05, the Bluetooth key
    /// codes (spec §7.1, §9.1).
    NoInfo,
}

/// How messages name a model code the meter reports.
fn reports_code(code: u8) -> String {
    format!("the meter reports OWON model code {code}")
}

/// The name a model code gives the meter: its entry's, or the bare code.
fn name_for(code: u8) -> String {
    match Model::for_code(code) {
        Some(model) => model.name.to_string(),
        None => format!("OWON model code {code}"),
    }
}

/// Why keys stay off for `confirmed` on an entry opened as `opened`; `None`
/// when they go out. Both the connect warning and every refused key say it.
fn keys_off(opened: &Model, confirmed: Confirmation) -> Option<String> {
    match confirmed {
        Confirmation::Matches | Confirmation::NoInfo => None,
        Confirmation::Other(model) => Some(format!(
            "{} ({}), not the {} it was opened as, so remote keys are off; choose Auto-detect \
             or --device {}",
            reports_code(model.code),
            model.name,
            opened.name,
            model.id
        )),
        Confirmation::Unknown(code) => Some(format!(
            "{}, which this tool does not know yet, so remote keys are off",
            reports_code(code)
        )),
    }
}

/// One of OWON's meters, opened through the registry entry for one model.
pub(crate) struct OwonProtocol {
    rx_buf: Vec<u8>,
    /// The model the entry is named for.
    model: &'static Model,
    profile: DeviceProfile,
    /// Set by `init`, from FFF2.
    confirmed: Confirmation,
    /// Whether `rx_buf` starts where the last frame read ended, rather than
    /// anywhere in a frame ([`frame::extract`]).
    aligned: bool,
}

impl OwonProtocol {
    /// `issue` is the entry's verification issue, one per model.
    fn new(model: &'static Model, issue: u16) -> Self {
        Self {
            rx_buf: Vec::with_capacity(64),
            model,
            profile: DeviceProfile {
                family_name: "OWON",
                model_name: model.name,
                stability: Stability::Experimental,
                supported_commands: model.commands,
                max_aux_values: 0,
                verification_issue: Some(issue),
                meter_keys: model.meter_keys,
            },
            confirmed: Confirmation::NoInfo,
            aligned: false,
        }
    }

    /// Check FFF2's value against the entry, and say what the meter
    /// reports beyond the code.
    fn confirm(&self, value: &[u8]) -> Result<Confirmation> {
        let Some(info) = Fff2::parse(value) else {
            report_unknown(self.model.id, "device information", format_args!("empty"));
            return Ok(Confirmation::NoInfo);
        };
        debug!("owon: device information: {info}");
        if let Some(why) = model::unsupported_format(info.code) {
            return Err(Error::invalid_response(
                format!("{}: {why}", reports_code(info.code)),
                value,
            ));
        }
        match info.record {
            // Spec §4, §8.1: OWON's app offers to stop the recording on
            // connect (B35-UM p.30/25-32/27).
            Some(Record::Recording) => warn!(
                "owon: the meter is recording to its own memory (an offline record started from \
                 OWON's app); stop the recording there if no readings arrive"
            ),
            Some(Record::Other(_)) => report_unknown(
                self.model.id,
                "offline record state",
                format_args!("{value:02X?}"),
            ),
            Some(Record::None | Record::Idle) | None => {}
        }
        Ok(match Model::for_code(info.code) {
            Some(model) if std::ptr::eq(model, self.model) => Confirmation::Matches,
            Some(model) => Confirmation::Other(model),
            None => {
                report_unknown(
                    self.model.id,
                    "model code",
                    format_args!("{} in {value:02X?}", info.code),
                );
                Confirmation::Unknown(info.code)
            }
        })
    }
}

impl Protocol for OwonProtocol {
    fn delivery(&self) -> crate::protocol::Delivery {
        crate::protocol::Delivery::Streamed
    }

    fn discard_input(&mut self, transport: &dyn Transport) -> Result<()> {
        self.rx_buf.clear();
        self.aligned = false;
        framing::discard_queued(transport)
    }

    /// Writes nothing: the meter streams once connected, and neither of
    /// OWON's programs needs more than the FFF2 read the transport made
    /// (spec §3). The challenge OWON's app writes is not sent: no client
    /// that skips it has gone without readings (spec §3.4, §14.4).
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        self.confirmed = match transport.info_characteristic() {
            Some(value) => self.confirm(value)?,
            None => {
                debug!(
                    "owon: no device information read; keys follow the {}",
                    self.model.name
                );
                Confirmation::NoInfo
            }
        };
        // Once per connect; an unknown code was reported already.
        if let Confirmation::Other(_) = self.confirmed
            && let Some(why) = keys_off(self.model, self.confirmed)
        {
            warn!("owon: {why}");
        }
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractor never fails, so the recovery mode and the skip
        // pattern are never used. About 2 frames a second arrive (spec §5,
        // §14.4), several inside read_frame's 2 s.
        let aligned = self.aligned;
        let read = framing::read_frame(
            &mut self.rx_buf,
            transport,
            |buf| frame::extract(buf, aligned),
            |_| true,
            FrameErrorRecovery::Propagate,
            LOG,
            &[],
        );
        // A frame read leaves the buffer at the next one's start; a failed
        // read may have cleared it anywhere.
        self.aligned = read.is_ok();
        match read {
            Ok(frame) => decode::decode(&frame, self.model),
            Err(Error::Timeout) if frame::lacks_marker(&self.rx_buf) => {
                let shown = &self.rx_buf[..self.rx_buf.len().min(SHOWN_BYTES)];
                report_unknown(
                    self.model.id,
                    "frames without the function-word marker",
                    format_args!("{shown:02X?}"),
                );
                let err = Error::invalid_response(NO_MARKER, shown);
                self.rx_buf.clear();
                Err(err)
            }
            Err(e) => Err(e),
        }
    }

    /// `payload` is one 6-byte frame, as the stream delivers it and a
    /// replay file stores it.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        decode::decode(payload, self.model)
    }

    /// Press a remote key: two bytes, no reply awaited; the stream shows
    /// what the key did (spec §7.1).
    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let Some(key) = self.model.keys.iter().find(|k| k.command == command) else {
            return Err(Error::UnsupportedCommand(command.to_string()));
        };
        if let Some(why) = keys_off(self.model, self.confirmed) {
            return Err(Error::CommandRejected(why));
        }
        let frame = keys::frame(key);
        debug!("owon: key {command}, frame {frame:02X?}");
        transport.write(&frame)
    }

    /// The model FFF2 names, read at bring-up: no wire I/O here.
    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        Ok(transport
            .info_characteristic()
            .and_then(Fff2::parse)
            .map(|info| name_for(info.code)))
    }

    fn name_before_init(&self, transport: &dyn Transport) -> bool {
        transport
            .info_characteristic()
            .and_then(Fff2::parse)
            .is_some()
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        capture::steps(self.model.code)
    }
}

/// Detection for OWON's meters: they stream unprompted, so nothing is sent.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Owon,
    label: "owon stream",
    trigger: None,
    send_after: &[],
    // No checksum at all (spec §5).
    checksummed: false,
    // OWON's GATT profile is the one that reads FFF2 (spec §4).
    claims_info_links: true,
    recognise,
};

/// With FFF2 read, its model code names the entry once one frame has come:
/// the link is OWON's profile, so the frames are OWON's. Without it, two
/// plausible frames in a row, which open the fallback entry. A code the
/// 15-byte or series-55 decoder reads (spec §1, §10.1) opens the fallback
/// entry on any bytes, so that `init` refuses it naming the format.
fn recognise(buf: &[u8], probing: &Probing) -> Option<Evidence> {
    match probing.info_characteristic.as_deref().and_then(Fff2::parse) {
        Some(info) => {
            if model::unsupported_format(info.code).is_none() && !frame::holds_frame(buf) {
                return None;
            }
            let id = Model::for_code(info.code).unwrap_or(FALLBACK).id;
            Some(Evidence::Model {
                id,
                reported_name: Some(name_for(info.code)),
            })
        }
        None => frame::two_in_a_row(buf).then_some(Evidence::Model {
            id: FALLBACK.id,
            reported_name: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::registry;
    use crate::transport::Link;
    use crate::transport::mock::MockTransport;
    use frame::tests::{VECTORS, stream};

    /// A meter on OWON's profile: `info` is what the FFF2 read gave.
    struct Meter {
        inner: MockTransport,
        info: Option<Vec<u8>>,
    }

    impl Meter {
        fn new(info: Option<&[u8]>, responses: Vec<Vec<u8>>) -> Self {
            Self {
                inner: MockTransport::new(responses),
                info: info.map(<[u8]>::to_vec),
            }
        }
    }

    impl Transport for Meter {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.inner.write(data)
        }
        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
            self.inner.read_timeout(buf, timeout_ms)
        }
        fn link(&self) -> Option<Link> {
            self.inner.link()
        }
        fn info_characteristic(&self) -> Option<&[u8]> {
            self.info.as_deref()
        }
    }

    /// Spec §14.5's B41T+ FFF2 value: code 41, firmware 0.1.2.
    const B41_INFO: [u8; 6] = [0x29, 0xFF, 0x00, 0x01, 0x02, 0x00];
    /// The same with code `code`.
    fn info(code: u8) -> [u8; 6] {
        let mut info = B41_INFO;
        info[0] = code;
        info
    }

    fn proto(model: &'static Model) -> OwonProtocol {
        // The issue number plays no part in decoding.
        OwonProtocol::new(model, 0)
    }

    /// `bytes` in reads of `size`.
    fn pieces(bytes: &[u8], size: usize) -> Vec<Vec<u8>> {
        bytes.chunks(size).map(<[u8]>::to_vec).collect()
    }

    /// Whole frames, single bytes and several frames per read each give
    /// every frame's reading, in order, with nothing reported.
    #[test]
    fn frames_across_notifications_give_their_readings() {
        for size in [6, 1, 12, 64] {
            let mock = MockTransport::new(pieces(&stream(), size));
            let mut proto = proto(&model::B35);
            for v in VECTORS {
                let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
                assert!(reports.is_empty(), "{size}: {reports:?}");
                assert_eq!(m.unwrap().raw_payload, v, "{size}");
            }
            assert!(proto.request_measurement(&mock).is_err(), "{size}");
        }
    }

    /// Joining mid-frame, the bytes before the next marker are dropped and
    /// the next whole frame is read.
    #[test]
    fn joining_mid_frame_reads_the_next_whole_frame() {
        let stream = stream();
        for skip in 1..6 {
            let mock = MockTransport::new(pieces(&stream[skip..], 6));
            let (m, reports) = capture_reports(|| proto(&model::B35).request_measurement(&mock));
            assert!(reports.is_empty(), "{skip}: {reports:?}");
            assert_eq!(m.unwrap().raw_payload, VECTORS[1], "{skip}");
        }
    }

    /// A reading whose low byte is a marker (count 241, `F1 00`), joined at
    /// frame byte 1 or 3, still gives the true frame, every time.
    #[test]
    fn a_marker_in_the_reading_does_not_shift_the_frames() {
        let v = [0x19, 0xF0, 0x04, 0x00, 0xF1, 0x00];
        let stream = v.repeat(5);
        for skip in [1, 3] {
            let mock = MockTransport::new(pieces(&stream[skip..], 6));
            let mut proto = proto(&model::B35);
            for _ in 0..3 {
                let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
                assert!(reports.is_empty(), "{skip}: {reports:?}");
                let m = m.unwrap();
                assert_eq!(m.raw_payload, v, "{skip}");
                assert_eq!(m.mode, "DC V", "{skip}");
            }
        }
    }

    /// A stream with bytes but no marker, as the older B35T's 14-byte
    /// frames would be, says so rather than timing out bare.
    #[test]
    fn a_stream_without_the_marker_names_the_older_format() {
        let ascii = [
            0x2B, 0x33, 0x36, 0x32, 0x33, 0x20, 0x34, 0x31, 0x00, 0x40, 0x80, 0x24, 0x0D, 0x0A,
        ];
        let mock = MockTransport::new(vec![ascii.repeat(8)]);
        let mut proto = proto(&model::B35);
        let (read, reports) = capture_reports(|| proto.request_measurement(&mock));
        match read {
            Err(Error::InvalidResponse { message, raw }) => {
                assert!(message.contains("14-byte"), "{message}");
                assert_eq!(raw.len(), SHOWN_BYTES);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(proto.rx_buf.is_empty());
        // Silence stays a plain timeout.
        let silent = MockTransport::new(Vec::new());
        assert!(matches!(
            proto.request_measurement(&silent),
            Err(Error::Timeout)
        ));
    }

    #[test]
    fn init_writes_nothing() {
        for info in [None, Some(&B41_INFO[..])] {
            let meter = Meter::new(info, Vec::new());
            let mut proto = proto(&model::B41);
            let (done, reports) = capture_reports(|| proto.init(&meter));
            done.unwrap();
            assert!(reports.is_empty(), "{reports:?}");
            assert!(meter.inner.written.borrow().is_empty());
            assert!(meter.inner.bauds.borrow().is_empty());
        }
    }

    /// The name is FFF2's model, read at bring-up, with nothing written.
    #[test]
    fn get_name_comes_from_the_device_information() {
        let mut proto = proto(&model::B35);
        let named = |info: Option<&[u8]>, proto: &mut OwonProtocol| {
            let meter = Meter::new(info, Vec::new());
            let asked = proto.name_before_init(&meter);
            let name = proto.get_name(&meter).unwrap();
            assert!(meter.inner.written.borrow().is_empty());
            (asked, name)
        };
        assert_eq!(
            named(Some(&B41_INFO), &mut proto),
            (true, Some("OWON B41T+".to_string()))
        );
        let ow18b = [0x12, 0x63, 0x04, 0x00, 0x09, 0x00];
        assert_eq!(
            named(Some(&ow18b), &mut proto),
            (true, Some("OWON OW18B/OW16B".to_string()))
        );
        assert_eq!(
            named(Some(&info(223)), &mut proto),
            (true, Some("OWON model code 223".to_string()))
        );
        assert_eq!(named(Some(&[]), &mut proto), (false, None));
        assert_eq!(named(None, &mut proto), (false, None));
    }

    /// Keys go out when FFF2 names the entry, and when nothing was read.
    #[test]
    fn keys_go_out_when_the_code_matches_or_none_was_read() {
        for info in [Some(&B41_INFO[..]), None] {
            let meter = Meter::new(info, Vec::new());
            let mut proto = proto(&model::B41);
            proto.init(&meter).unwrap();
            proto.send_command(&meter, "hold").unwrap();
            assert_eq!(*meter.inner.written.borrow(), [[0x03, 0x01]], "{info:?}");
        }
    }

    /// Spec §12.3: Select, short, on a B-series meter, one write and no
    /// read.
    #[test]
    fn a_key_is_one_two_byte_write() {
        let meter = Meter::new(Some(&info(35)), Vec::new());
        let mut proto = proto(&model::B35);
        proto.init(&meter).unwrap();
        proto.send_command(&meter, "select").unwrap();
        assert_eq!(*meter.inner.written.borrow(), [[0x01, 0x01]]);
        let err = proto.send_command(&meter, "zero").unwrap_err();
        assert!(matches!(err, Error::UnsupportedCommand(_)), "{err:?}");
        assert_eq!(meter.inner.written.borrow().len(), 1);
    }

    /// FFF2 naming another of the six: init says which and how to open
    /// it, and every key is refused unsent, saying the same.
    #[test]
    fn a_mismatched_model_code_is_named_and_keys_are_refused() {
        let meter = Meter::new(Some(&B41_INFO), Vec::new());
        let mut proto = proto(&model::B35);
        let (done, reports) = capture_reports(|| proto.init(&meter));
        done.unwrap();
        assert!(reports.is_empty(), "{reports:?}");
        assert_eq!(proto.confirmed, Confirmation::Other(&model::B41));
        for command in model::B35.commands {
            match proto.send_command(&meter, command) {
                Err(Error::CommandRejected(why)) => {
                    assert!(why.contains("OWON B41T+"), "{why}");
                    assert!(why.contains("OWON B35T+"), "{why}");
                    assert!(why.contains("--device b41t+"), "{why}");
                }
                other => panic!("{command}: {other:?}"),
            }
        }
        assert!(meter.inner.written.borrow().is_empty());
    }

    /// A code no entry carries is reported, and keys stay off.
    #[test]
    fn an_unknown_model_code_is_reported_and_keys_are_refused() {
        let meter = Meter::new(Some(&info(223)), Vec::new());
        let mut proto = proto(&model::B35);
        let (done, reports) = capture_reports(|| proto.init(&meter));
        done.unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(reports[0].contains("model code"), "{reports:?}");
        let err = proto.send_command(&meter, "hold").unwrap_err();
        assert!(
            matches!(&err, Error::CommandRejected(why) if why.contains("223")),
            "{err:?}"
        );
        assert!(meter.inner.written.borrow().is_empty());
    }

    /// An empty FFF2 is reported and counts as no information.
    #[test]
    fn an_empty_device_information_is_reported() {
        let meter = Meter::new(Some(&[]), Vec::new());
        let mut proto = proto(&model::OW18B);
        let (done, reports) = capture_reports(|| proto.init(&meter));
        done.unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert_eq!(proto.confirmed, Confirmation::NoInfo);
    }

    /// Byte 5 values OWON's PC software does not list are reported;
    /// recording (`01`) is warned about, not reported.
    #[test]
    fn the_offline_record_byte_is_checked() {
        for (byte, reported) in [(0x00, false), (0xFF, false), (0x01, false), (0x02, true)] {
            let mut value = info(35);
            value[5] = byte;
            let meter = Meter::new(Some(&value), Vec::new());
            let mut proto = proto(&model::B35);
            let (done, reports) = capture_reports(|| proto.init(&meter));
            done.unwrap();
            assert_eq!(!reports.is_empty(), reported, "{byte:02X}: {reports:?}");
            assert_eq!(proto.confirmed, Confirmation::Matches);
        }
    }

    /// Spec §14.5's VC871 frames, one of which holds a plausible 6-byte
    /// window at offset 7.
    const VC871: [[u8; 15]; 6] = [
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
    ];

    /// A meter whose code names the 15-byte frame (VC871, code 87), or
    /// series 55's: init refuses it naming the format, and detection opens
    /// the fallback entry on its frames, for init to refuse.
    #[test]
    fn a_15_byte_model_code_is_refused() {
        let frames = VC871.concat();
        assert!(frame::holds_frame(&frames), "a 6-byte window is there");
        for (code, says) in [(87, "15-byte"), (101, "15-byte"), (55, "sign")] {
            let meter = Meter::new(Some(&info(code)), Vec::new());
            match proto(&model::B35).init(&meter) {
                Err(Error::InvalidResponse { message, .. }) => {
                    assert!(message.contains(says), "{code}: {message}");
                    assert!(message.contains(&code.to_string()), "{message}");
                }
                other => panic!("{code}: {other:?}"),
            }
            let probing = Probing {
                info_characteristic: Some(info(code).to_vec()),
                ..Probing::default()
            };
            let fallback = Some(Evidence::Model {
                id: "b35t+",
                reported_name: Some(format!("OWON model code {code}")),
            });
            assert_eq!(recognise(&frames, &probing), fallback, "{code}");
            assert_eq!(recognise(&frames[..1], &probing), fallback, "{code}");
        }
    }

    /// Under auto-detect, a VC871's frames end in init's refusal, which
    /// names the format, not in a reading.
    #[test]
    fn auto_detect_on_a_15_byte_meter_ends_in_inits_error() {
        let meter = Meter::new(Some(&info(87)), vec![VC871.concat()]);
        let detected = crate::detect::detect_device(&meter, crate::BLUETOOTH).unwrap();
        assert_eq!(detected.device.id, "b35t+");
        match crate::Dmm::from_detected(meter, &detected) {
            Err(Error::InvalidResponse { message, .. }) => {
                assert!(message.contains("15-byte"), "{message}");
                assert!(message.contains("model code 87"), "{message}");
            }
            Err(other) => panic!("{other:?}"),
            Ok(_) => panic!("opened"),
        }
    }

    #[test]
    fn every_registry_entry_is_its_models() {
        let mut issues = Vec::new();
        for m in model::MODELS {
            let entry = registry::find_device(m.id).expect("registry entry");
            assert_eq!(entry.display_name, m.name);
            assert_eq!(entry.family, DeviceFamily::Owon);
            assert_eq!(entry.links, [crate::BLUETOOTH]);
            assert_eq!(entry.bluetooth_names, ["BDM", "LILLIPUT"]);
            assert!(std::ptr::eq(entry.fingerprint.unwrap(), &FINGERPRINT));
            assert!(
                entry
                    .activation_instructions
                    .starts_with("1. Disconnect the meter from any phone app\n"),
                "{}",
                m.id
            );
            assert!(
                entry
                    .manual_url
                    .is_some_and(|u| u.starts_with("https://www.owontech.com/digital-multimeters/")),
                "{}",
                m.id
            );
            let proto = (entry.new_protocol)();
            let profile = proto.profile();
            assert_eq!(profile.family_name, "OWON");
            assert_eq!(profile.model_name, m.name);
            assert_eq!(profile.stability, Stability::Experimental);
            assert_eq!(profile.supported_commands, m.commands);
            assert_eq!(profile.max_aux_values, 0);
            issues.push(profile.verification_issue);
            assert_eq!(proto.delivery(), crate::protocol::Delivery::Streamed);
        }
        let issues: Vec<u16> = issues.into_iter().flatten().collect();
        assert_eq!(issues, [39, 40, 41, 42, 43, 44]);
        let ids: Vec<&str> = model::MODELS.iter().map(|m| m.id).collect();
        assert_eq!(ids, ["ow18b", "ow18e", "b33", "b35t+", "b41t+", "cm2100b"]);
        for (alias, id) in [
            ("OW16B", "ow18b"),
            ("owon-ow18e", "ow18e"),
            ("b33t+", "b33"),
            ("b35+", "b35t+"),
            ("b41t", "b41t+"),
            ("owon-cm2100b", "cm2100b"),
        ] {
            assert_eq!(registry::resolve_device(alias).map(|d| d.id), Some(id));
        }
        // A "+"-less B35 may send the 14-byte frame (spec §11).
        for alias in ["b35", "b35t"] {
            assert!(registry::resolve_device(alias).is_none(), "{alias}");
        }
    }

    fn probing(info: Option<&[u8]>) -> Probing {
        Probing {
            info_characteristic: info.map(<[u8]>::to_vec),
            ..Probing::default()
        }
    }

    /// Spec §14.5's B41T+ FFF2 and one of its frames name the B41T+.
    #[test]
    fn recognise_names_the_model_from_the_device_information() {
        let frame = [0x24, 0xF0, 0x04, 0x00, 0x03, 0x00];
        assert_eq!(
            recognise(&frame, &probing(Some(&B41_INFO))),
            Some(Evidence::Model {
                id: "b41t+",
                reported_name: Some("OWON B41T+".to_string()),
            })
        );
        assert_eq!(recognise(&frame[..5], &probing(Some(&B41_INFO))), None);
        // A frame is a frame with the code read, status bits and all.
        let rmr = [0x24, 0xF0, 0x84, 0x00, 0x03, 0x00];
        assert!(recognise(&rmr, &probing(Some(&B41_INFO))).is_some());
        for m in model::MODELS {
            let found = recognise(&frame, &probing(Some(&info(m.code))));
            assert!(
                matches!(found, Some(Evidence::Model { id, .. }) if id == m.id),
                "{}",
                m.id
            );
        }
    }

    /// Without a model code, one frame is not enough: two plausible ones
    /// in a row open the fallback entry, unnamed.
    #[test]
    fn recognise_without_device_information_needs_two_frames() {
        let none = probing(None);
        assert_eq!(recognise(&VECTORS[0], &none), None);
        assert_eq!(
            recognise(&stream(), &none),
            Some(Evidence::Model {
                id: "b35t+",
                reported_name: None,
            })
        );
        let mut rmr = VECTORS[..2].concat();
        rmr[2] |= 0x80;
        assert_eq!(recognise(&rmr, &none), None, "a high status bit");
        assert_eq!(
            recognise(&stream(), &probing(Some(&[]))),
            recognise(&stream(), &none)
        );
    }

    /// A code no entry carries is still a model code: the fallback entry,
    /// with the code as the name.
    #[test]
    fn an_unknown_model_code_falls_back_named() {
        assert_eq!(
            recognise(&VECTORS[0], &probing(Some(&info(223)))),
            Some(Evidence::Model {
                id: "b35t+",
                reported_name: Some("OWON model code 223".to_string()),
            })
        );
    }

    fn found(buf: &[u8]) -> bool {
        recognise(buf, &Probing::default()).is_some()
    }

    /// Other meters' bytes on the same Bluetooth links: the UT61+ ack and
    /// name frames the detection tests use, the UT-D07B's heartbeat
    /// (`transport/ble/issc.rs`), ZOTEK's worked examples as they arrive on
    /// air (ZOTEK spec §9), the 121GW's worked example 1 (121GW spec §13)
    /// and a BM78xBT notification, several times over.
    #[test]
    fn recognise_declines_other_meters() {
        use crate::protocol::framing::test_frame_be16;
        let mut ut61 = test_frame_be16(&[0xFF, 0x00]);
        ut61.extend(test_frame_be16(b"UT61E+"));
        assert!(!found(&ut61.repeat(4)));
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
        // BM78xBT spec §10's information packet and example 1's reading,
        // padded to a whole notification.
        let mut bm78xbt = vec![
            0xFF, 0x01, 0x18, 0x04, 0x01, 0x02, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x01, 0xFC, 0x94, 0xFF, 0x03, 0xFF, 0x02, 0x20, 0x05,
            0x01, 0x00, 0x00, 0x01, 0xFA, 0x78, 0x85, 0x03, 0x3A, 0x35, 0x10, 0x40, 0x00, 0x01,
            0x03, 0x00, 0x01, 0xC7, 0xCF, 0xFF, 0x01, 0x00, 0x02, 0x05, 0x76, 0xC3, 0xFF, 0x03,
        ];
        bm78xbt.resize(152, 0);
        assert!(!found(&bm78xbt.repeat(4)));
    }
}
