//! OWON's Bluetooth LE meters: the B33, B35T+, B41T+, OW16B, OW18B, OW18E
//! and CM2100B, which send a 6-byte frame, and the CMS101, CMS061, OW65B,
//! OW67B, OW69B and Voltcraft's VC871, VC891, VC915 and VC925 PV, which send
//! a 15-byte one (`docs/research/owon/reverse-engineered-protocol.md`).
//!
//! The meter notifies frames on FFF4 unprompted once connected, with no
//! header or checksum: three little-endian 16-bit words (spec §5, §6), or
//! five 24-bit words with a sub-display (spec §10). The model is FFF2 byte
//! 0, which the Bluetooth transport reads at bring-up and hands on as
//! [`Transport::info_characteristic`] (spec §1, §4): it says which frame
//! comes, and function 13, the RMR bit and the remote keys differ by model,
//! so `init` checks the code against the entry opened, and detection picks
//! the entry by it. Keys are two bytes on FFF3 (spec §7.1, §10.8).
//!
//! - `model.rs`: the models, by model code, and FFF2's value
//! - `frame.rs`, `frame15.rs`: finding 6- and 15-byte frames in the stream
//! - `decode.rs`, `decode15.rs`: frame → `Measurement`
//! - `keys.rs`: the remote keys each model offers, and their bytes
//! - `capture.rs`: the capture steps per model
//! - `devices.rs`: the registry entries

mod capture;
mod decode;
mod decode15;
pub(crate) mod devices;
pub(crate) mod frame;
mod frame15;
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
use model::{FALLBACK, Fff2, FrameKind, Model, Record};
use std::cell::Cell;

/// Log label for the read loop.
const LOG: &str = "owon";

/// What a read says when the meter sends bytes but never a frame: the
/// older B35 and B35T send a 14-byte ASCII frame (spec §11, §14.4).
const NO_MARKER: &str = "the meter sends no OWON 6-byte frames; it may be an older B35 or B35T, \
     whose 14-byte frames this tool does not read yet";

/// The same on a 15-byte entry: no byte 2 of `F0` (spec §10.2, §10.9).
const NO_MARKER_15: &str = "the meter sends no OWON 15-byte frames; choose Auto-detect";

/// What a read says when no known model code was read and the frames tile
/// as neither kind ([`stream_kind`]): status bits no capture has shown, or
/// a stream that is not OWON's.
const UNTOLD: &str = "neither the meter's model code nor its frames show whether it sends \
     OWON's 6- or 15-byte frames; please open an issue with this message";

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
    /// auto-detect that entry is detection's B35T+ guess from 6-byte
    /// frames, so the keys may go to another OWON model. That is safe
    /// because no key that goes out then is a long press of 04 or 05, the
    /// Bluetooth key codes (spec §7.1, §9.1): the one key table that holds
    /// one, REL held on the Voltcraft meters, waits for the code
    /// ([`keys::Key::needs_confirmed`]).
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

/// Why `key` stays off while the model code is unread, when it waits for
/// it: the connect warning. A long press of its code switches Bluetooth on
/// OWON's B series and CM2100B (spec §7.1).
fn unconfirmed(key: &keys::Key) -> String {
    format!(
        "the meter's model code was not read, so {} is off",
        key.command
    )
}

/// The refusal of `key` while the model code is unread: why, and what to
/// do instead. Only `exit_rel` waits for the code.
fn unconfirmed_refusal(key: &keys::Key) -> String {
    format!("{}: hold REL on the meter to leave REL", unconfirmed(key))
}

/// What a read says when the frames are the other kind than the entry's,
/// with no known model code to name the meter.
fn other_frame(opened: &Model, sent: FrameKind) -> String {
    let fix = match sent {
        // Detection opens a 6-byte stream's fallback entry.
        FrameKind::Six => "choose Auto-detect",
        // Nothing in a 15-byte frame names the model (spec §10.2).
        FrameKind::Fifteen => "choose the meter's own model",
    };
    format!(
        "the meter sends OWON's {} frames, not the {} frames of the {} it was opened as; {fix}",
        sent.name(),
        opened.frame.name(),
        opened.name
    )
}

/// The frame kind `buf` shows: two whole frames of it in a row, running to
/// the buffer's end (spec §5, §10.2). The 6-byte test goes first, as in
/// detection: it passes on a 15-byte stream only for a reading above every
/// model's count.
fn stream_kind(buf: &[u8]) -> Option<FrameKind> {
    if frame::tiles(buf) {
        Some(FrameKind::Six)
    } else if frame15::tiles(buf) {
        Some(FrameKind::Fifteen)
    } else {
        None
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
    /// Whether the stream's frame kind is known: from the model code, or,
    /// with no known code, from two whole frames of the entry's kind
    /// ([`stream_kind`]). Until then no frame is read, so a meter opened
    /// under an entry of the other kind gives no readings.
    kind_known: bool,
}

impl OwonProtocol {
    /// `issue` is the entry's verification issue, `None` until it is
    /// opened.
    fn new(model: &'static Model, issue: Option<u16>) -> Self {
        Self {
            rx_buf: Vec::with_capacity(64),
            model,
            profile: DeviceProfile {
                family_name: "OWON",
                model_name: model.name,
                stability: Stability::Experimental,
                supported_commands: model.commands,
                max_aux_values: model.frame.max_aux_values(),
                verification_issue: issue,
                meter_keys: model.meter_keys,
            },
            confirmed: Confirmation::NoInfo,
            aligned: false,
            kind_known: false,
        }
    }

    /// The model frames decode as: the one FFF2 names, when that is another
    /// of the entry's frame kind, as function 13, RMR and the sub-display's
    /// labels differ by model (spec §6.2, §6.6, §10.7).
    fn reads_as(&self) -> &'static Model {
        match self.confirmed {
            Confirmation::Other(model) => model,
            _ => self.model,
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
                format!("{}{why}", reports_code(info.code)),
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
            // The other frame would decode to nonsense (spec §10.1).
            Some(model) if model.frame != self.model.frame => {
                return Err(Error::invalid_response(
                    format!(
                        "{} ({}), which sends OWON's {} frame, not the {} it was opened as; \
                         choose Auto-detect or --device {}",
                        reports_code(model.code),
                        model.name,
                        model.frame.name(),
                        self.model.name,
                        model.id
                    ),
                    value,
                ));
            }
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
        // Both are the entry's frame kind: `confirm` refuses the other.
        self.kind_known = matches!(
            self.confirmed,
            Confirmation::Matches | Confirmation::Other(_)
        );
        // Once per connect; an unknown code was reported already.
        if let Confirmation::Other(_) = self.confirmed
            && let Some(why) = keys_off(self.model, self.confirmed)
        {
            warn!("owon: {why}");
        }
        if self.confirmed == Confirmation::NoInfo
            && let Some(key) = self.model.keys.iter().find(|k| k.needs_confirmed)
        {
            warn!("owon: {}", unconfirmed(key));
        }
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractors never fail, so the skip pattern is never used;
        // only the stream check below does, and the error goes out. About 2
        // frames a second arrive (spec §5, §14.4), several inside
        // read_frame's 2 s.
        let (opened, aligned) = (self.model, self.aligned);
        let kind = opened.frame;
        // Checked once per read: a frame `kept` turns down may leave fewer
        // than two behind.
        let kind_known = Cell::new(self.kind_known);
        let read = framing::read_frame(
            &mut self.rx_buf,
            transport,
            |buf| {
                // With no known model code, the frames must show their kind
                // first: each kind's framer finds frames in the other's
                // stream, function words and readings alike holding `F0`.
                if !kind_known.get() {
                    match stream_kind(buf) {
                        None => return Ok(None),
                        Some(sent) if sent != kind => {
                            let shown = &buf[..buf.len().min(SHOWN_BYTES)];
                            return Err(Error::invalid_response(other_frame(opened, sent), shown));
                        }
                        Some(_) => kind_known.set(true),
                    }
                }
                match kind {
                    FrameKind::Six => frame::extract(buf, aligned),
                    FrameKind::Fifteen => frame15::extract(buf, aligned),
                }
            },
            |frame| kind == FrameKind::Six || frame15::kept(frame),
            FrameErrorRecovery::Propagate,
            LOG,
            &[],
        );
        // A frame read leaves the buffer at the next one's start; a failed
        // read may have cleared it anywhere.
        self.aligned = read.is_ok();
        self.kind_known = kind_known.get();
        let (lacks_marker, no_marker) = match kind {
            FrameKind::Six => (frame::lacks_marker(&self.rx_buf), NO_MARKER),
            FrameKind::Fifteen => (frame15::lacks_marker(&self.rx_buf), NO_MARKER_15),
        };
        let untold = !self.kind_known && self.rx_buf.len() >= 2 * frame15::FRAME_LEN;
        match read {
            Ok(frame) => self.parse_payload(&frame),
            Err(Error::Timeout) if lacks_marker || untold => {
                let shown = &self.rx_buf[..self.rx_buf.len().min(SHOWN_BYTES)];
                let (what, message) = if lacks_marker {
                    ("frames without the function-word marker", no_marker)
                } else {
                    ("frames of neither kind", UNTOLD)
                };
                report_unknown(self.model.id, what, format_args!("{shown:02X?}"));
                let err = Error::invalid_response(message, shown);
                self.rx_buf.clear();
                Err(err)
            }
            Err(e) => Err(e),
        }
    }

    /// `payload` is one frame of the model's kind, as the stream delivers
    /// it and a replay file stores it.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        let model = self.reads_as();
        match model.frame {
            FrameKind::Six => decode::decode(payload, model),
            FrameKind::Fifteen => decode15::decode(payload, model),
        }
    }

    /// Press a remote key: two bytes, no reply awaited; the stream shows
    /// what the key did (spec §7.1, §10.8).
    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let Some(key) = self.model.keys.iter().find(|k| k.command == command) else {
            return Err(Error::UnsupportedCommand(command.to_string()));
        };
        if let Some(why) = keys_off(self.model, self.confirmed) {
            return Err(Error::CommandRejected(why));
        }
        if key.needs_confirmed && self.confirmed == Confirmation::NoInfo {
            return Err(Error::CommandRejected(unconfirmed_refusal(key)));
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
/// the link is OWON's profile, so the frames are OWON's. A 15-byte model's
/// code needs only 15 bytes, so that a stream its frame does not fit ends
/// in that entry's no-marker error rather than in no meter found. Without a
/// code, two plausible 6-byte frames in a row with the marker at every
/// later 6-byte step ([`frame::tiles`]), which open the fallback entry; a
/// 15-byte stream opens nothing, as no entry can be told from its frames. A
/// code no entry reads (spec §1, §10.1) opens the fallback entry on any
/// bytes, so that `init` refuses it naming why.
fn recognise(buf: &[u8], probing: &Probing) -> Option<Evidence> {
    match probing.info_characteristic.as_deref().and_then(Fff2::parse) {
        Some(info) => {
            let model = Model::for_code(info.code);
            let whole = match model.map(|m| m.frame) {
                Some(FrameKind::Fifteen) => buf.len() >= frame15::FRAME_LEN,
                Some(FrameKind::Six) | None => frame::holds_frame(buf),
            };
            if model::unsupported_format(info.code).is_none() && !whole {
                return None;
            }
            let id = model.unwrap_or(FALLBACK).id;
            Some(Evidence::Model {
                id,
                reported_name: Some(name_for(info.code)),
            })
        }
        None => frame::tiles(buf).then_some(Evidence::Model {
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
    use frame15::tests::VECTORS as VC871_STREAM;

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
        OwonProtocol::new(model, None)
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

    /// With no model code read, an entry refuses the other frame kind's
    /// stream, both ways round, rather than reading it, an RMR stream
    /// (status bit 7, spec §6.6) included; once its own kind has shown, the
    /// check is off.
    #[test]
    fn an_entry_refuses_the_other_frame_kind() {
        let mut rmr = frame::tests::stream();
        for at in (2..rmr.len()).step_by(frame::FRAME_LEN) {
            rmr[at] |= 0x80;
        }
        for (opened, sent, bytes, size) in [
            (&model::VC871, FrameKind::Six, frame::tests::stream(), 6),
            (&model::VC915, FrameKind::Six, rmr, 6),
            (&model::B35, FrameKind::Fifteen, VC871_STREAM.concat(), 15),
            (&model::OW18B, FrameKind::Fifteen, VC871_STREAM.concat(), 15),
        ] {
            let mut proto = proto(opened);
            let meter = MockTransport::new(pieces(&bytes, size));
            match proto.request_measurement(&meter) {
                Err(Error::InvalidResponse { message, raw }) => {
                    assert_eq!(message, other_frame(opened, sent));
                    assert_eq!(raw.len(), 2 * size, "{}", opened.id);
                }
                other => panic!("{}: {other:?}", opened.id),
            }
            assert!(proto.rx_buf.is_empty());
        }
        let says = |opened, sent| other_frame(opened, sent);
        assert!(
            says(&model::VC871, FrameKind::Six)
                .ends_with("Voltcraft VC871 it was opened as; choose Auto-detect")
        );
        assert!(
            says(&model::B35, FrameKind::Fifteen)
                .contains("OWON's 15-byte frames, not the 6-byte frames of the OWON B35T+")
        );

        let mut vc871 = proto(&model::VC871);
        let first = MockTransport::new(pieces(&VC871_STREAM[..2].concat(), 15));
        assert_eq!(
            vc871.request_measurement(&first).unwrap().raw_payload,
            VC871_STREAM[0]
        );
        let six = MockTransport::new(pieces(&frame::tests::stream(), 6));
        let (read, _) = capture_reports(|| vc871.request_measurement(&six));
        assert!(
            !matches!(read, Err(Error::InvalidResponse { .. })),
            "{read:?}"
        );
    }

    /// With the model code read, the frames are the code's kind: the first
    /// one gives a reading.
    #[test]
    fn with_the_model_code_one_frame_gives_a_reading() {
        for (opened, code, frame) in [
            (&model::VC871, 87, VC871_STREAM[0].to_vec()),
            (&model::B35, 35, VECTORS[0].to_vec()),
        ] {
            let meter = Meter::new(Some(&info(code)), vec![frame.clone()]);
            let mut proto = proto(opened);
            proto.init(&meter).unwrap();
            assert_eq!(
                proto.request_measurement(&meter).unwrap().raw_payload,
                frame
            );
        }
    }

    /// With no model code read, frames that tile as neither kind, here a
    /// 6-byte stream with status bit 8 set throughout, which no capture
    /// shows (spec §14.4), give an error saying so, reported once.
    #[test]
    fn frames_of_neither_kind_say_so() {
        let mut odd = frame::tests::stream();
        for at in (3..odd.len()).step_by(frame::FRAME_LEN) {
            odd[at] |= 0x01;
        }
        let mut proto = proto(&model::B35);
        let mock = MockTransport::new(pieces(&odd, 6));
        let (read, reports) = capture_reports(|| proto.request_measurement(&mock));
        match read {
            Err(Error::InvalidResponse { message, raw }) => {
                assert_eq!(message, UNTOLD);
                assert_eq!(raw.len(), SHOWN_BYTES);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// FFF2 naming another model of the entry's frame kind: its frames read
    /// as that model's. A VC871 opened as the OW67B, its twin, keeps its
    /// sub-display under REL (spec §10.7), and a CMS101 opened as the OW65B
    /// reads NCV (spec §6.2).
    #[test]
    fn frames_read_as_the_model_the_code_names() {
        use decode15::tests::{frame, g24};
        // DC V under REL, its sub word Hz (spec §10.7).
        let rel = frame(g24(0, 4, 3, true), 1234, g24(6, 3, 1, false), 5000, 0x06);
        // NCV, level 2 (spec §6.7).
        let ncv = frame(g24(13, 4, 0, false), 2, 0, 0, 0);
        for (opened, code, payload) in [(&model::OW67B, 87, rel), (&model::OW65B, 101, ncv)] {
            let named = Model::for_code(code).unwrap();
            let meter = Meter::new(Some(&info(code)), Vec::new());
            let mut proto = proto(opened);
            proto.init(&meter).unwrap();
            let shown = |m: Measurement| format!("{} {:?}", m.mode, m.aux_values);
            let (read, reports) = capture_reports(|| proto.parse_payload(&payload));
            assert!(reports.is_empty(), "{}: {reports:?}", opened.id);
            let read = shown(read.unwrap());
            let named = shown(decode15::decode(&payload, named).unwrap());
            let (as_opened, _) = capture_reports(|| decode15::decode(&payload, opened));
            assert_eq!(read, named, "{}", opened.id);
            assert_ne!(read, shown(as_opened.unwrap()), "{}", opened.id);
        }
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

    /// Spec §14.5's VC871 frames, in reads of each size, give their
    /// readings in order with nothing reported; a frame ending in `FF` is
    /// skipped (spec §10.2).
    #[test]
    fn vc871_frames_across_notifications_give_their_readings() {
        let stream = VC871_STREAM.concat();
        for size in [15, 1, 30, 64] {
            let mock = MockTransport::new(pieces(&stream, size));
            let mut proto = proto(&model::VC871);
            for v in VC871_STREAM {
                let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
                assert!(reports.is_empty(), "{size}: {reports:?}");
                assert_eq!(m.unwrap().raw_payload, v, "{size}");
            }
            assert!(proto.request_measurement(&mock).is_err(), "{size}");
        }
        let mut filler = VC871_STREAM[0];
        filler[14] = 0xFF;
        let mock = MockTransport::new(vec![[filler, VC871_STREAM[1]].concat()]);
        let m = proto(&model::VC871).request_measurement(&mock).unwrap();
        assert_eq!(m.raw_payload, VC871_STREAM[1]);
    }

    /// Joined anywhere in a 15-byte frame, the next whole frame is read.
    #[test]
    fn joining_a_15_byte_stream_mid_frame_reads_the_next_whole_frame() {
        let stream = VC871_STREAM.concat();
        for skip in 1..15 {
            let meter = Meter::new(Some(&info(87)), pieces(&stream[skip..], 15));
            let mut proto = proto(&model::VC871);
            proto.init(&meter).unwrap();
            let (m, reports) = capture_reports(|| proto.request_measurement(&meter));
            assert!(reports.is_empty(), "{skip}: {reports:?}");
            assert_eq!(m.unwrap().raw_payload, VC871_STREAM[1], "{skip}");
        }
    }

    /// Bytes with no `F0` on a 15-byte entry say so in the 15-byte words.
    #[test]
    fn a_15_byte_entry_without_its_frames_says_so() {
        let mock = MockTransport::new(vec![vec![0x2B; 64]]);
        let mut proto = proto(&model::VC915);
        let (read, reports) = capture_reports(|| proto.request_measurement(&mock));
        match read {
            Err(Error::InvalidResponse { message, .. }) => {
                assert!(message.contains("15-byte"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// A meter whose code names the other frame is refused, naming the
    /// entry to open, both ways round (spec §10.1).
    #[test]
    fn a_model_code_of_the_other_frame_names_its_entry() {
        for (opened, code, says) in [
            (&model::B35, 87, "--device vc871"),
            (&model::B35, 101, "--device cms101"),
            (&model::VC871, 35, "--device b35t+"),
            (&model::OW65B, 18, "--device ow18b"),
        ] {
            let meter = Meter::new(Some(&info(code)), Vec::new());
            match proto(opened).init(&meter) {
                Err(Error::InvalidResponse { message, .. }) => {
                    assert!(message.contains(says), "{code}: {message}");
                    assert!(message.contains(&format!("model code {code}")), "{message}");
                    assert!(message.contains(opened.name), "{message}");
                }
                other => panic!("{code}: {other:?}"),
            }
        }
        // Another model of the same frame: keys off, as for the 6-byte
        // meters.
        let meter = Meter::new(Some(&info(67)), Vec::new());
        let mut vc871 = proto(&model::VC871);
        vc871.init(&meter).unwrap();
        assert_eq!(vc871.confirmed, Confirmation::Other(&model::OW67B));
    }

    /// Codes 83 and 85 (the VC831 and VC851, which have no Bluetooth) and
    /// series 55 are refused naming why, and detection opens the fallback
    /// entry on any bytes, for init to refuse; 223 is reported, not
    /// refused.
    #[test]
    fn codes_no_entry_reads_are_refused() {
        let frames = VC871_STREAM.concat();
        for (code, says) in [
            (
                83,
                "the meter reports OWON model code 83, which this tool does not read yet; \
                 please open an issue with this message",
            ),
            (85, "model code 85, which this tool does not read yet"),
            (55, "model code 55: it sends"),
        ] {
            let meter = Meter::new(Some(&info(code)), Vec::new());
            match proto(&model::B35).init(&meter) {
                Err(Error::InvalidResponse { message, .. }) => {
                    assert!(message.contains(says), "{code}: {message}");
                    assert!(message.contains(&code.to_string()), "{message}");
                }
                other => panic!("{code}: {other:?}"),
            }
            let fallback = Some(Evidence::Model {
                id: "b35t+",
                reported_name: Some(format!("OWON model code {code}")),
            });
            let probing = probing(Some(&info(code)));
            assert_eq!(recognise(&frames, &probing), fallback, "{code}");
            assert_eq!(recognise(&frames[..1], &probing), fallback, "{code}");
        }
        let meter = Meter::new(Some(&info(223)), Vec::new());
        assert!(proto(&model::B35).init(&meter).is_ok());
    }

    /// Under auto-detect, a VC871's code and frames open the VC871 entry,
    /// which reads them.
    #[test]
    fn auto_detect_on_a_vc871_reads_it() {
        let stream = VC871_STREAM.concat();
        let meter = Meter::new(Some(&info(87)), vec![stream.clone(), stream]);
        let detected = crate::detect::detect_device(&meter, crate::BLUETOOTH).unwrap();
        assert_eq!(detected.device.id, "vc871");
        assert_eq!(detected.reported_name.as_deref(), Some("Voltcraft VC871"));
        let mut dmm = crate::Dmm::from_detected(meter, &detected).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.raw_payload.len(), 15);
    }

    /// REL held is code 4 long, the B series' Bluetooth key: it goes out
    /// only once FFF2 named the model, and init warns of it when nothing
    /// was read.
    #[test]
    fn exit_rel_waits_for_the_model_code() {
        for (info, sent) in [
            (Some(info(87)), true),
            (None, false),
            (Some(info(67)), false),
            (Some(info(223)), false),
        ] {
            let meter = Meter::new(info.as_ref().map(|i| &i[..]), Vec::new());
            let mut proto = proto(&model::VC871);
            proto.init(&meter).unwrap();
            let done = proto.send_command(&meter, "exit_rel");
            assert_eq!(done.is_ok(), sent, "{info:?}: {done:?}");
            if sent {
                assert_eq!(*meter.inner.written.borrow(), [[0x04, 0x00]]);
            } else {
                assert!(matches!(done, Err(Error::CommandRejected(_))));
                assert!(meter.inner.written.borrow().is_empty());
            }
        }
        let meter = Meter::new(None, Vec::new());
        let mut proto = proto(&model::VC925PV);
        proto.init(&meter).unwrap();
        match proto.send_command(&meter, "exit_rel") {
            Err(Error::CommandRejected(why)) => {
                assert!(why.contains("model code was not read"), "{why}");
                assert!(why.contains("hold REL"), "{why}");
            }
            other => panic!("{other:?}"),
        }
        // A tap goes out with no code read.
        proto.send_command(&meter, "rel").unwrap();
        assert_eq!(*meter.inner.written.borrow(), [[0x04, 0x01]]);
    }

    #[test]
    fn every_registry_entry_is_its_models() {
        let mut issues = Vec::new();
        for m in model::MODELS {
            let entry = registry::find_device(m.id).expect("registry entry");
            assert_eq!(entry.display_name, m.name);
            assert_eq!(entry.family, DeviceFamily::Owon);
            assert_eq!(entry.links, [crate::BLUETOOTH]);
            let names: &[&str] = match m.frame {
                FrameKind::Six => &["BDM", "LILLIPUT"],
                FrameKind::Fifteen if m.name.starts_with("Voltcraft") => &["BDM", "VC8", "VC9"],
                FrameKind::Fifteen => &["BDM"],
            };
            assert_eq!(entry.bluetooth_names, names, "{}", m.id);
            assert!(std::ptr::eq(entry.fingerprint.unwrap(), &FINGERPRINT));
            assert!(
                entry
                    .activation_instructions
                    .starts_with("1. Disconnect the meter from any phone app\n"),
                "{}",
                m.id
            );
            let manual = match m.id {
                "ow65b" | "ow67b" | "ow69b" => "http://owon.co.jp/products_info.asp?ProductID=",
                "vc871" | "vc891" | "vc915" | "vc925pv" => {
                    "https://asset.conrad.com/media10/add/160267/c1/-/gl/"
                }
                _ => "https://www.owontech.com/digital-multimeters/",
            };
            assert!(
                entry.manual_url.is_some_and(|u| u.starts_with(manual)),
                "{}",
                m.id
            );
            let proto = (entry.new_protocol)();
            let profile = proto.profile();
            assert_eq!(profile.family_name, "OWON");
            assert_eq!(profile.model_name, m.name);
            assert_eq!(profile.stability, Stability::Experimental);
            assert_eq!(profile.supported_commands, m.commands);
            assert_eq!(profile.max_aux_values, m.frame.max_aux_values());
            issues.push(profile.verification_issue);
            assert_eq!(proto.delivery(), crate::protocol::Delivery::Streamed);
        }
        // The 15-byte entries' issues are still to be opened.
        assert_eq!(
            issues,
            [
                Some(39),
                Some(40),
                Some(41),
                Some(42),
                Some(43),
                Some(44),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None
            ]
        );
        let ids: Vec<&str> = model::MODELS.iter().map(|m| m.id).collect();
        assert_eq!(
            ids,
            [
                "ow18b", "ow18e", "b33", "b35t+", "b41t+", "cm2100b", "cms101", "cms061", "ow65b",
                "ow67b", "ow69b", "vc871", "vc891", "vc915", "vc925pv"
            ]
        );
        let names: Vec<&str> = model::MODELS[6..].iter().map(|m| m.name).collect();
        assert_eq!(
            names,
            [
                "OWON CMS101",
                "OWON CMS061",
                "OWON OW65B",
                "OWON OW67B",
                "OWON OW69B",
                "Voltcraft VC871",
                "Voltcraft VC891",
                "Voltcraft VC915",
                "Voltcraft VC925 PV"
            ]
        );
        for (alias, id) in [
            ("OW16B", "ow18b"),
            ("owon-ow18e", "ow18e"),
            ("b33t+", "b33"),
            ("b35+", "b35t+"),
            ("b41t", "b41t+"),
            ("owon-cm2100b", "cm2100b"),
            ("VC-871", "vc871"),
            ("vc-891", "vc891"),
            ("vc-915", "vc915"),
            ("vc-925pv", "vc925pv"),
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
        let vc871 = VC871_STREAM[0];
        for m in model::MODELS {
            let bytes: &[u8] = match m.frame {
                FrameKind::Six => &frame,
                FrameKind::Fifteen => &vc871,
            };
            let found = recognise(bytes, &probing(Some(&info(m.code))));
            assert_eq!(
                found,
                Some(Evidence::Model {
                    id: m.id,
                    reported_name: Some(m.name.to_string()),
                }),
                "{}",
                m.id
            );
        }
    }

    /// A 15-byte model's code claims once 15 bytes have come, whatever they
    /// are: a stream its frame does not fit then fails in that entry's
    /// read, naming the frame.
    #[test]
    fn a_15_byte_code_claims_after_15_bytes() {
        let probing = probing(Some(&info(87)));
        assert_eq!(recognise(&VC871_STREAM[0][..14], &probing), None);
        let vc871 = Some(Evidence::Model {
            id: "vc871",
            reported_name: Some("Voltcraft VC871".to_string()),
        });
        assert_eq!(recognise(&VC871_STREAM[0], &probing), vc871);
        assert_eq!(recognise(&[0x2B; 15], &probing), vc871);
        // And a 6-byte frame alone is not enough for it.
        assert_eq!(recognise(&VECTORS[0], &probing), None);
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
        let mut unseen = VECTORS[..2].concat();
        unseen[2] |= 0x40;
        assert_eq!(recognise(&unseen, &none), None, "status bit 6");
        // Bit 7 is the B41T+'s RMR (spec §6.6).
        let mut rmr = VECTORS[..2].concat();
        rmr[2] |= 0x80;
        rmr[8] |= 0x80;
        assert!(recognise(&rmr, &none).is_some());
        assert_eq!(
            recognise(&stream(), &probing(Some(&[]))),
            recognise(&stream(), &none)
        );
    }

    /// Without a model code, a 15-byte meter's frames are not taken for
    /// 6-byte ones, alone, repeated or joined anywhere: spec §14.5's VC871
    /// frames.
    #[test]
    fn recognise_without_device_information_declines_15_byte_frames() {
        let none = probing(None);
        for frame in VC871_STREAM {
            for count in 1..=4 {
                let frames = frame.repeat(count);
                for skip in 0..frame.len() {
                    assert_eq!(
                        recognise(&frames[skip..], &none),
                        None,
                        "{frame:02X?} {skip}"
                    );
                }
            }
        }
        let all = VC871_STREAM.concat();
        for skip in 0..15 {
            assert_eq!(recognise(&all[skip..], &none), None, "{skip}");
        }
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
