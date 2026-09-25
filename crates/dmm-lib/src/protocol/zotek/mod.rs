//! ZOTEK Bluetooth LE meters, sold as ZOYI, ZOTEK, BSIDE and ANENG
//! (`docs/research/zotek/reverse-engineered-protocol.md`).
//!
//! The meters stream unprompted over the FFF0/FFF4 GATT profile, which the
//! Bluetooth transport turns into a byte stream (spec §2, §3). Every packet
//! is XOR-scrambled with a fixed key and, descrambled, starts `5A A5 <type>`;
//! the type byte picks one of four layouts of LCD segments and annunciator
//! bits (spec §1, §4-§7). No packet names the model or brand, only its
//! layout, so the registry has one entry per layout, and a packet is always
//! decoded by its own type byte whichever entry was opened.
//!
//! - `frame.rs`: the key, the per-type lengths and the stream extractor
//! - `glyph.rs`: the seven-segment glyphs and the words they spell
//! - `layout.rs`: each layout's bit table, and packet → `Measurement`
//! - `capture.rs`: the capture steps per layout

mod capture;
mod frame;
mod glyph;
mod layout;

use crate::error::Result;
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Stability,
};
use crate::transport::Transport;
use layout::Layout;
use log::{debug, warn};

/// The packet being decoded, for reporting what in it no spec section
/// covers, under the registry id of the layout it was decoded with.
#[derive(Clone, Copy)]
pub(super) struct Unrecognised<'a> {
    id: &'static str,
    packet: &'a [u8],
}

impl Unrecognised<'_> {
    fn report(self, what: &'static str) {
        report_unknown(self.id, what, format_args!("packet {:02X?}", self.packet));
    }
}

/// Log label for the read loop.
const LOG: &str = "zotek";

/// A ZOTEK meter, opened through the registry entry for one layout.
pub(crate) struct ZotekProtocol {
    rx_buf: Vec<u8>,
    /// The layout the entry is named for.
    layout: &'static Layout,
    profile: DeviceProfile,
    /// Whether this connection has already said the meter sends another
    /// layout than the entry's.
    warned_layout: bool,
}

impl ZotekProtocol {
    fn new(layout: &'static Layout) -> Self {
        Self {
            rx_buf: Vec::with_capacity(64),
            layout,
            profile: DeviceProfile {
                family_name: "ZOTEK",
                model_name: layout.name,
                stability: Stability::Experimental,
                // Key presses (spec §8.2) are a later addition.
                supported_commands: &[],
                max_aux_values: layout.max_aux_values,
                verification_issue: None,
            },
            warned_layout: false,
        }
    }

    pub(crate) fn new_zt300ab() -> Self {
        Self::new(&layout::ZT300AB)
    }

    pub(crate) fn new_zt5566se() -> Self {
        Self::new(&layout::ZT5566SE)
    }

    /// The layout a packet of `type_byte` is in, the first time it is not
    /// the entry's on this connection; `None` every other time.
    fn other_layout(&mut self, type_byte: u8) -> Option<&'static Layout> {
        if self.warned_layout || type_byte == self.layout.type_byte {
            return None;
        }
        let other = layout::for_type(type_byte)?;
        self.warned_layout = true;
        Some(other)
    }
}

/// Whether the read loop takes `packet`: a layout that is not implemented
/// yet is reported and skipped.
fn implemented(entry: &'static str, packet: &[u8]) -> bool {
    let type_byte = packet[frame::TYPE_AT];
    let known = layout::for_type(type_byte).is_some();
    if !known {
        report_unknown(
            entry,
            "packet type",
            format_args!("{type_byte}, whose layout is not supported yet"),
        );
    }
    known
}

impl Protocol for ZotekProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // The meter streams once connected; no app writes anything first
        // (spec §3).
        debug!("zotek: init ({}, listen only)", self.profile.model_name);
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractor never fails, so the recovery mode and the skip
        // pattern are never used. About 2.6 packets a second arrive
        // (spec §11.4), well inside read_frame's 2 s. A packet with no digit
        // lit on the main display has no reading, so it is reported and
        // skipped for the next one.
        let entry = self.layout.id;
        let packet = framing::read_frame(
            &mut self.rx_buf,
            transport,
            frame::extract_packet,
            |p| implemented(entry, p) && layout::shows_digits(p),
            FrameErrorRecovery::Propagate,
            LOG,
            &frame::RAW_HEADER,
        )?;
        if let Some(other) = self.other_layout(packet[frame::TYPE_AT]) {
            warn!(
                "zotek: the meter sends the {} layout; choose Auto-detect or --device {}",
                other.name, other.id
            );
        }
        self.parse_payload(&packet)
    }

    /// `payload` is one whole descrambled packet, as the stream delivers it
    /// and a replay file stores it; it is decoded by its own type byte.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        layout::decode(payload)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        capture::steps(self.layout)
    }
}

/// Detection for the ZOTEK meters: they stream unprompted, so nothing is
/// sent, and a packet names its layout, which picks the entry.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Zotek,
    label: "zotek stream",
    trigger: None,
    send_after: &[],
    checksummed: false,
    recognise,
};

/// One whole packet anywhere in `buf`: the on-air header, an implemented
/// type, that type's length, and every digit a listed glyph. There is no
/// checksum (spec §5), so the glyphs are what stands in for one.
fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    (0..buf.len())
        .filter_map(|start| frame::packet_at(buf, start))
        .find_map(|packet| layout::plausible(&packet))
        .map(|layout| Evidence::Model {
            id: layout.id,
            reported_name: None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::registry;
    use crate::transport::mock::MockTransport;
    use frame::tests::EXAMPLES;

    fn scrambled(plain: &[u8]) -> Vec<u8> {
        let mut raw = plain.to_vec();
        frame::xor_key(&mut raw);
        raw
    }

    /// Each worked example is recognised as its layout's entry.
    #[test]
    fn the_worked_examples_are_recognised_as_their_layouts() {
        for (example, id) in [(0, "zt300ab"), (3, "zt5566se")] {
            let (raw, _) = EXAMPLES[example];
            assert_eq!(
                recognise(raw, &Probing::default()),
                Some(Evidence::Model {
                    id,
                    reported_name: None
                })
            );
        }
    }

    /// An entry opened on a meter that sends another layout decodes it by
    /// its own type byte, and says so once per connection.
    #[test]
    fn another_layout_is_decoded_and_named_once() {
        let (raw, plain) = EXAMPLES[3];
        let mock = MockTransport::new(vec![raw.to_vec(), raw.to_vec()]);
        let mut proto = ZotekProtocol::new_zt300ab();
        assert!(proto.other_layout(3).is_none());
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.raw_payload, plain);
        assert_eq!(m.aux_values.len(), 1, "decoded as type 4");
        assert!(proto.warned_layout);
        assert!(proto.other_layout(4).is_none(), "warned once");
        assert!(proto.request_measurement(&mock).is_ok());

        let mut fresh = ZotekProtocol::new_zt300ab();
        assert_eq!(fresh.other_layout(4).map(|l| l.id), Some("zt5566se"));
        assert!(fresh.other_layout(4).is_none());
    }

    /// Detection scans every offset: a false header in front of a real
    /// packet does not hide it.
    #[test]
    fn recognise_finds_a_packet_after_a_false_header() {
        let (raw, _) = EXAMPLES[0];
        let mut buf = vec![0x1B, 0x84, 0x70, 0x00];
        buf.extend_from_slice(raw);
        assert!(recognise(&buf, &Probing::default()).is_some());
        assert_eq!(recognise(&raw[..raw.len() - 1], &Probing::default()), None);
    }

    /// A glyph the spec does not list is not taken for a packet.
    #[test]
    fn recognise_declines_unlisted_glyphs() {
        let (_, plain) = EXAMPLES[0];
        let mut bad = plain.to_vec();
        bad[5] = 0x01 | (bad[5] & 0xF0);
        bad[4] &= 0x1F;
        assert_eq!(recognise(&scrambled(&bad), &Probing::default()), None);
    }

    #[test]
    fn every_layout_id_is_its_registry_entrys() {
        for layout in layout::LAYOUTS {
            let entry = registry::find_device(layout.id).expect("registry entry");
            assert_eq!(entry.display_name, layout.name);
            assert_eq!(entry.family, DeviceFamily::Zotek);
            let proto = (entry.new_protocol)();
            assert_eq!(proto.profile().model_name, layout.name);
        }
    }

    #[test]
    fn init_sends_nothing() {
        let mock = MockTransport::new(Vec::new());
        ZotekProtocol::new_zt300ab().init(&mock).unwrap();
        assert!(mock.written.borrow().is_empty());
        assert!(mock.feature_reports.borrow().is_empty());
    }

    /// A packet of a layout not implemented yet is reported and skipped.
    #[test]
    fn a_layout_not_implemented_yet_is_skipped() {
        let (raw, plain) = EXAMPLES[0];
        let (other, _) = EXAMPLES[1];
        let mut stream = other.to_vec();
        stream.extend_from_slice(raw);
        let mock = MockTransport::new(vec![stream]);
        let mut proto = ZotekProtocol::new_zt300ab();
        let (m, reports) = crate::protocol::capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().raw_payload, plain);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(reports[0].contains("packet type"), "{reports:?}");
    }

    /// Notifications arrive in pieces and back to back; each read returns
    /// the next whole packet, descrambled.
    #[test]
    fn request_measurement_reads_packets_across_reads() {
        let (raw, plain) = EXAMPLES[0];
        let mut stream = raw[4..].to_vec();
        stream.extend_from_slice(raw);
        stream.extend_from_slice(raw);
        let reports: Vec<Vec<u8>> = stream.chunks(3).map(<[u8]>::to_vec).collect();
        let mock = MockTransport::new(reports);
        let mut proto = ZotekProtocol::new_zt300ab();
        for _ in 0..2 {
            let m = proto.request_measurement(&mock).unwrap();
            assert_eq!(m.raw_payload, plain);
            assert_eq!(m.display_raw.as_deref(), Some("-12.34"));
        }
        assert!(proto.request_measurement(&mock).is_err());
    }

    /// A packet with no digit lit gives no reading: the read reports it and
    /// skips it for the next one.
    #[test]
    fn a_blank_display_is_skipped() {
        let (raw, plain) = EXAMPLES[0];
        let mut blank = plain.to_vec();
        blank[3] &= 0x0F;
        blank[4..7].fill(0);
        blank[7] &= 0xF0;
        let mut stream = scrambled(&blank);
        stream.extend_from_slice(raw);
        let mock = MockTransport::new(vec![stream]);
        let mut proto = ZotekProtocol::new_zt300ab();
        let (m, reports) = crate::protocol::capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().raw_payload, plain);
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// Joining mid-packet onto bytes that happen to read `1B 84 70`: the
    /// false packet they start runs into the real one, draws no listed
    /// glyph, and is skipped, so the real packet is read and nothing is
    /// reported.
    #[test]
    fn a_false_header_in_a_partial_packet_is_skipped() {
        let (raw, plain) = EXAMPLES[0];
        let mut stream = vec![0x55, 0x1B, 0x84, 0x70, 0x00, 0x01];
        stream.extend_from_slice(raw);
        let mock = MockTransport::new(vec![stream]);
        let mut proto = ZotekProtocol::new_zt300ab();
        let (m, reports) = crate::protocol::capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().raw_payload, plain);
        assert!(reports.is_empty(), "{reports:?}");
    }
}
