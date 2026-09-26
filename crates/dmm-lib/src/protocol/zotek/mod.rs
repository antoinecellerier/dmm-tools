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
//! - `keys.rs`: the remote keys each layout offers, and their frames
//! - `capture.rs`: the capture steps per layout
//! - `sim.rs`: a simulated ZT-5B, the `mock-zt5b` device

mod capture;
mod frame;
mod glyph;
mod keys;
mod layout;
pub(crate) mod sim;

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Stability,
};
use crate::transport::Transport;
use layout::{Layout, Showing};
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
    /// The type byte of the last packet decoded on this connection, and
    /// what it showed, which the key codes follow; `None` before the first.
    showing: Option<(u8, Showing)>,
}

impl ZotekProtocol {
    /// `issue` is the entry's verification issue, one per layout.
    fn new(layout: &'static Layout, issue: u16) -> Self {
        Self {
            rx_buf: Vec::with_capacity(64),
            layout,
            profile: DeviceProfile {
                family_name: "ZOTEK",
                model_name: layout.name,
                stability: Stability::Experimental,
                supported_commands: keys::commands(layout),
                max_aux_values: layout.max_aux_values,
                verification_issue: Some(issue),
                meter_keys: keys::meter_keys(layout),
            },
            warned_layout: false,
            showing: None,
        }
    }

    pub(crate) fn new_zt300ab() -> Self {
        Self::new(&layout::ZT300AB, 28)
    }

    pub(crate) fn new_zt5566se() -> Self {
        Self::new(&layout::ZT5566SE, 29)
    }

    pub(crate) fn new_zt5bq() -> Self {
        Self::new(&layout::ZT5BQ, 30)
    }

    pub(crate) fn new_zt5b() -> Self {
        Self::new(&layout::ZT5B, 31)
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

impl Protocol for ZotekProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // The meter streams once connected; no app writes anything first
        // (spec §3).
        debug!("zotek: init ({}, listen only)", self.profile.model_name);
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The extractor never fails, so the recovery mode and the skip
        // pattern are never used, and it only cuts packets of the four
        // types, each with a layout. About 2.6 packets a second arrive
        // (spec §11.4), well inside read_frame's 2 s. A packet with no digit
        // lit on the main display has no reading, so it is reported and
        // skipped for the next one.
        let packet = framing::read_frame(
            &mut self.rx_buf,
            transport,
            frame::extract_packet,
            layout::shows_digits,
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
        let (reading, showing) = layout::decode_showing(&packet)?;
        self.showing = Some((packet[frame::TYPE_AT], showing));
        Ok(reading)
    }

    /// `payload` is one whole descrambled packet, as the stream delivers it
    /// and a replay file stores it; it is decoded by its own type byte.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        layout::decode(payload)
    }

    /// Press a remote key, one of those the entry's profile lists. The codes
    /// follow the layout of the meter's packets, the entry's before the
    /// first.
    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        if !self.profile.supported_commands.contains(&command) {
            return Err(Error::UnsupportedCommand(command.to_string()));
        }
        // A key whose code follows the display needs a packet first when
        // none has arrived on this connection, as when `dmm-cli command`
        // opens the meter and presses at once.
        if self.showing.is_none() && keys::follows_display(command) {
            self.request_measurement(transport)?;
        }
        let (type_byte, showing) = self
            .showing
            .unwrap_or((self.layout.type_byte, Showing::default()));
        let code = keys::code(type_byte, command, showing)?;
        let frame = keys::frame(code);
        debug!("zotek: key {command} ({code:02X}), frame {frame:02X?}");
        // One write, no reply awaited: neither app waits for one, and the
        // stream shows what the key did (spec §8.1).
        transport.write(&frame)
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

/// One whole packet anywhere in `buf`: the on-air header, a known
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
        for (example, id) in [(0, "zt300ab"), (1, "zt5bq"), (2, "zt5b"), (3, "zt5566se")] {
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

    /// A key is one frame, written without waiting for anything: the mock
    /// has nothing to read.
    #[test]
    fn a_key_is_one_write_and_no_read() {
        let mock = MockTransport::new(Vec::new());
        let mut proto = ZotekProtocol::new_zt5b();
        proto.send_command(&mock, "auto_function").unwrap();
        assert_eq!(
            *mock.written.borrow(),
            [[0xEA, 0xEC, 0x70, 0xED, 0xA2, 0xC1, 0x32, 0x71, 0x64, 0x99]],
            "spec §9's AUTO frame"
        );
    }

    /// Spec §9's type-4 packet, on air, with V off, DC kept only if `dc`,
    /// and `bit` of `byte` lit.
    fn type4_showing(byte: usize, bit: u8, dc: bool) -> Vec<u8> {
        let mut plain = EXAMPLES[3].1.to_vec();
        plain[4] &= !0x10;
        if !dc {
            plain[13] &= !0x02;
        }
        plain[byte] |= bit;
        scrambled(&plain)
    }

    /// Before any packet, a key that follows the display reads one; after
    /// it, keys follow that one without reading.
    #[test]
    fn a_display_key_follows_the_last_packet() {
        // A, DC (spec §7.4).
        let mock = MockTransport::new(vec![type4_showing(18, 0x10, true)]);
        let mut proto = ZotekProtocol::new_zt5566se();
        proto.send_command(&mock, "current").unwrap();
        assert_eq!(*mock.written.borrow(), [keys::frame(0xC8)]);

        let err = proto.send_command(&mock, "zero").unwrap_err();
        assert!(matches!(err, Error::CommandRejected(_)), "{err:?}");
        assert_eq!(mock.written.borrow().len(), 1, "nothing sent");
    }

    /// An entry opened on a meter sending another layout: the keys follow
    /// the layout of its packets, here type 4's current key in A DC, not
    /// the ZT-300AB's `C9`.
    #[test]
    fn keys_follow_the_layout_the_meter_sends() {
        let mock = MockTransport::new(vec![type4_showing(18, 0x10, true)]);
        let mut proto = ZotekProtocol::new_zt300ab();
        proto.request_measurement(&mock).unwrap();
        proto.send_command(&mock, "current").unwrap();
        assert_eq!(*mock.written.borrow(), [keys::frame(0xC8)]);
    }

    #[test]
    fn zero_goes_out_while_farads_show() {
        // F (spec §7.4).
        let mock = MockTransport::new(vec![type4_showing(17, 0x01, false)]);
        let mut proto = ZotekProtocol::new_zt5566se();
        assert_eq!(proto.request_measurement(&mock).unwrap().unit, "F");
        proto.send_command(&mock, "zero").unwrap();
        assert_eq!(*mock.written.borrow(), [keys::frame(0xB5)]);
    }

    /// A display key with no packet to read fails as the read does, and
    /// sends nothing.
    #[test]
    fn a_display_key_with_nothing_to_read_times_out() {
        let mock = MockTransport::new(Vec::new());
        let mut proto = ZotekProtocol::new_zt5bq();
        let err = proto.send_command(&mock, "temp_unit").unwrap_err();
        assert!(matches!(err, Error::Timeout), "{err:?}");
        assert!(mock.written.borrow().is_empty());
    }

    /// The ZT-300AB entry refuses the keys its layout goes without, and
    /// no entry takes the other families' `auto`.
    #[test]
    fn keys_outside_the_layout_are_unsupported() {
        let mock = MockTransport::new(Vec::new());
        let mut proto = ZotekProtocol::new_zt300ab();
        for command in ["hold", "auto_function", "auto"] {
            let err = proto.send_command(&mock, command).unwrap_err();
            assert!(matches!(err, Error::UnsupportedCommand(_)), "{err:?}");
        }
        assert!(mock.written.borrow().is_empty());
        assert!(
            ZotekProtocol::new_zt5b()
                .profile()
                .supported_commands
                .contains(&"hold")
        );
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
