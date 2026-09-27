//! Brymen's meters on the BU-86X cable: the BM860s series (BM869s, BM867s)
//! (`docs/research/bm86x/reverse-engineered-protocol.md`).
//!
//! Each reading over the cable (`transport/bu86x.rs`) is one request naming
//! the series, `00 cc 66`, and one reply of 24 data bytes that map the LCD's
//! segments (spec §3, §4). The BM820s and BM520s share the cable and the
//! request with codes of their own, and a second map (spec §6); the BM860s is
//! the one served here.
//!
//! - `reply.rs`: the request, and finding the reply
//! - `map.rs`: where each segment sits in a reply
//! - `glyph.rs`: the seven-segment characters and what a row shows
//! - `decode.rs`: the lit segments → `Measurement`
//! - `capture.rs`: the capture steps
//! - `devices.rs`: the registry entry

mod capture;
mod decode;
pub(crate) mod devices;
mod glyph;
mod map;
mod reply;

use crate::error::Result;
use crate::measurement::Measurement;
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Probing, Protocol,
    Stability,
};
use crate::transport::Transport;
use log::debug;

/// A series on the cable: its code, which a request names and a reply
/// carries in its model bytes (spec §1.1, §4.2), and its LCD map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Series {
    /// The BM860s: BM869s, BM867s.
    Bm86x,
}

impl Series {
    /// The series code (spec §3.1).
    pub(super) const fn code(self) -> u8 {
        match self {
            Series::Bm86x => 0x86,
        }
    }

    /// The series' LCD map (spec §5.1).
    fn map(self) -> &'static map::Map {
        match self {
            Series::Bm86x => &map::BM860,
        }
    }

    /// The registry id, which the report hint names as `--device`.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Series::Bm86x => "bm86x",
        }
    }
}

/// Report what in reply `data` no spec section covers.
fn report(series: Series, data: &[u8], what: &'static str) {
    report_unknown(series.id(), what, format_args!("reply {data:02X?}"));
}

/// A Brymen meter on the BU-86X cable, polled one reading per request.
pub(crate) struct Bm86xProtocol {
    series: Series,
    profile: DeviceProfile,
}

impl Bm86xProtocol {
    pub(crate) fn new(series: Series, model_name: &'static str) -> Self {
        Self {
            series,
            profile: DeviceProfile {
                family_name: "BM86x",
                model_name,
                stability: Stability::Experimental,
                // The meter takes no keys over the cable (spec §1.2, §11.4).
                supported_commands: &[],
                // The secondary display (spec §5.2).
                max_aux_values: 1,
                verification_issue: None,
                meter_keys: MeterKeys::NONE,
            },
        }
    }
}

impl Protocol for Bm86xProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // Nothing to set up: the cable answers each request (spec §3.3).
        debug!("{}: init (polled, nothing to send)", self.series.id());
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let data = reply::read(transport, self.series)?;
        decode::decode(&data, self.series)
    }

    /// `payload` is a reply's 24 data bytes, as `raw_payload` carries them.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        decode::decode(payload, self.series)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        match self.series {
            Series::Bm86x => capture::bm86x(),
        }
    }
}

/// Detection for the BM860s: the reading request, the same bytes a reading
/// sends, so a meter that answers it is left as opening it would.
pub(crate) static FINGERPRINT_86: Fingerprint = Fingerprint {
    family: DeviceFamily::Bm86x,
    label: "bm86x request",
    trigger: Some(trigger_86),
    send_after: &[],
    // No checksum: four model bytes are all a reply has (spec §4.2).
    checksummed: false,
    recognise: recognise_86,
};

fn trigger_86(transport: &dyn Transport) -> Result<()> {
    transport.write(&reply::request(Series::Bm86x.code()))
}

/// A reply of the series: its four model bytes, bytes 20-23, anywhere in
/// the buffer. The BM860 sheet names byte 23 alone; four is what detection
/// asks of a reply with no checksum (spec §4.2).
fn recognise_86(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    reply::model_run_at(buf, Series::Bm86x).map(|_| Evidence::Model {
        id: Series::Bm86x.id(),
        reported_name: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::protocol::registry;
    use reply::tests::{ScriptedCable, example_reports};

    fn proto() -> Bm86xProtocol {
        Bm86xProtocol::new(Series::Bm86x, "Brymen BM869s/BM867s")
    }

    /// The example with the main V set, as three reports.
    fn reports_with_v() -> Vec<Vec<u8>> {
        let mut reports = example_reports();
        reports[1][0] |= 0x01;
        reports
    }

    #[test]
    fn a_reading_is_one_request_and_its_reply() {
        let cable = ScriptedCable::new([0x00, 0x86, 0x66], reports_with_v(), Vec::new());
        let mut proto = proto();
        proto.init(&cable).unwrap();
        assert!(cable.written.borrow().is_empty(), "init sends nothing");
        let m = proto.request_measurement(&cable).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.display_raw.as_deref(), Some("312.71"));
        assert_eq!(m.raw_payload, reports_with_v().concat());
        assert_eq!(cable.written.borrow().as_slice(), [vec![0x00, 0x86, 0x66]]);
        // Its payload parses back to the same reading.
        let again = proto.parse_payload(&m.raw_payload).unwrap();
        assert_eq!(again.display_raw, m.display_raw);
        assert!(matches!(
            proto.parse_payload(&m.raw_payload[..23]),
            Err(Error::InvalidResponse { .. })
        ));
    }

    #[test]
    fn a_silent_cable_times_out() {
        let cable = ScriptedCable::new([0x00, 0x86, 0x66], Vec::new(), Vec::new());
        assert!(matches!(
            proto().request_measurement(&cable),
            Err(Error::Timeout)
        ));
    }

    #[test]
    fn no_command_is_offered() {
        let cable = ScriptedCable::new([0x00, 0x86, 0x66], Vec::new(), Vec::new());
        let mut proto = proto();
        for command in ["hold", "range", "rel", ""] {
            assert!(proto.send_command(&cable, command).is_err(), "{command}");
        }
        assert!(cable.written.borrow().is_empty());
    }

    #[test]
    fn the_registry_entry_is_this_familys() {
        let entry = registry::find_device("bm86x").expect("registry entry");
        assert_eq!(entry.family, DeviceFamily::Bm86x);
        assert_eq!(entry.display_name, "Brymen BM869s/BM867s");
        assert_eq!(entry.links, [crate::transport::bu86x::NAME]);
        assert!(entry.bluetooth_names.is_empty());
        for alias in ["bm869s", "BM867s", "brymen-bm869s", "brymen-bm867s"] {
            assert_eq!(
                registry::resolve_device(alias).map(|d| d.id),
                Some("bm86x"),
                "{alias}"
            );
        }
        let proto = (entry.new_protocol)();
        let profile = proto.profile();
        assert_eq!(profile.model_name, entry.display_name);
        assert_eq!(profile.max_aux_values, 1);
        assert_eq!(profile.stability, Stability::Experimental);
        assert!(profile.supported_commands.is_empty());
        assert!(std::ptr::eq(entry.fingerprint.unwrap(), &FINGERPRINT_86));
    }

    fn found(buf: &[u8]) -> bool {
        recognise_86(buf, &Probing::default()).is_some()
    }

    #[test]
    fn recognise_takes_four_model_bytes() {
        assert!(found(&reports_with_v().concat()));
        let mut late = vec![0x00; 13];
        late.extend(reports_with_v().concat());
        assert!(found(&late));
        // Byte 23 alone is the sheet's, but too little to detect on.
        let mut one = reports_with_v().concat();
        one[16..19].fill(0x00);
        assert!(!found(&one));
        assert!(!found(&[0x86; 3]));
        assert!(!found(&[]));
    }
}
