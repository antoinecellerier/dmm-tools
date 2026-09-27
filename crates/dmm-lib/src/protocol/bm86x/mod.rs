//! Brymen's meters on the BU-86X cable: the BM860s (BM869s, BM867s), the
//! BM820s (BM829s, BM827s, BM822s, BM821s) and the BM520s (BM525s, BM521s)
//! (`docs/research/bm86x/reverse-engineered-protocol.md`).
//!
//! Each reading over the cable (`transport/bu86x.rs`) is one request naming
//! the series, `00 cc 66`, and one reply of 24 data bytes that map the LCD's
//! segments (spec §3, §4). The BM860s has a map of its own; the BM820s and
//! BM520s share the other (spec §5, §6).
//!
//! - `reply.rs`: the request, and finding the reply
//! - `map.rs`: where each segment sits in a reply
//! - `glyph.rs`: the seven-segment characters and what a row shows
//! - `decode.rs`: the lit segments → `Measurement`
//! - `capture.rs`: the capture steps
//! - `devices.rs`: the registry entries

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
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Protocol, Stability,
};
use crate::transport::Transport;
use log::debug;

/// A series on the cable: its code, which a request names and a reply
/// carries in its model bytes (spec §1.1, §4.2), and its LCD map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Series {
    /// The BM860s: BM869s, BM867s.
    Bm86x,
    /// The BM820s: BM829s, BM827s, BM822s, BM821s.
    Bm82x,
    /// The BM520s, the logging models: BM525s, BM521s.
    Bm52x,
}

impl Series {
    /// Every series, in registry order.
    pub(super) const ALL: [Series; 3] = [Series::Bm86x, Series::Bm82x, Series::Bm52x];

    /// The series code (spec §3.1).
    pub(super) const fn code(self) -> u8 {
        match self {
            Series::Bm86x => 0x86,
            Series::Bm82x => 0x82,
            Series::Bm52x => 0x52,
        }
    }

    /// The series' LCD map (spec §5.1, §6.1).
    fn map(self) -> &'static map::Map {
        match self {
            Series::Bm86x => &map::BM860,
            Series::Bm82x | Series::Bm52x => &map::BM820,
        }
    }

    /// The registry id, which the report hint names as `--device`.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Series::Bm86x => "bm86x",
            Series::Bm82x => "bm82x",
            Series::Bm52x => "bm52x",
        }
    }

    const fn family_name(self) -> &'static str {
        match self {
            Series::Bm86x => "BM86x",
            Series::Bm82x => "BM82x",
            Series::Bm52x => "BM52x",
        }
    }

    /// EF detection: "E.F." and dashes on the main display, on the BM827s
    /// and BM829s (spec §7.3, §11.1).
    const fn detects_fields(self) -> bool {
        matches!(self, Series::Bm82x)
    }

    /// Data logging and Recall, on the BM525s and BM521s (spec §7.3,
    /// §11.1, §11.4).
    const fn logs(self) -> bool {
        matches!(self, Series::Bm52x)
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
                family_name: series.family_name(),
                model_name,
                stability: Stability::Experimental,
                // The meter takes no keys over the cable (spec §1.2, §11.4).
                supported_commands: &[],
                // The secondary display (spec §5.2, §6.2).
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
            Series::Bm82x => capture::bm82x(),
            Series::Bm52x => capture::bm52x(),
        }
    }
}

/// Detection, one fingerprint per series: the reading request, the same
/// bytes a reading sends, so a meter that answers it is left as opening it
/// would. No checksum: four model bytes are all a reply has (spec §4.2).
pub(crate) static FINGERPRINT_86: Fingerprint = Fingerprint {
    family: DeviceFamily::Bm86x,
    label: "bm86x request",
    trigger: Some(|transport| trigger(transport, Series::Bm86x)),
    send_after: &[],
    checksummed: false,
    recognise: |buf, _| recognise(buf, Series::Bm86x),
};

pub(crate) static FINGERPRINT_82: Fingerprint = Fingerprint {
    family: DeviceFamily::Bm82x,
    label: "bm82x request",
    trigger: Some(|transport| trigger(transport, Series::Bm82x)),
    send_after: &[],
    checksummed: false,
    recognise: |buf, _| recognise(buf, Series::Bm82x),
};

pub(crate) static FINGERPRINT_52: Fingerprint = Fingerprint {
    family: DeviceFamily::Bm52x,
    label: "bm52x request",
    trigger: Some(|transport| trigger(transport, Series::Bm52x)),
    send_after: &[],
    checksummed: false,
    recognise: |buf, _| recognise(buf, Series::Bm52x),
};

fn trigger(transport: &dyn Transport, series: Series) -> Result<()> {
    transport.write(&reply::request(series.code()))
}

/// A reply of `series`: its four model bytes, bytes 20-23, anywhere in the
/// buffer, whichever series' request drew it. The BM860 sheet names byte 23
/// alone; four is what detection asks of a reply with no checksum (spec
/// §4.2).
fn recognise(buf: &[u8], series: Series) -> Option<Evidence> {
    reply::model_run_at(buf, series).map(|_| Evidence::Model {
        id: series.id(),
        reported_name: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::protocol::{Probing, registry};
    use reply::tests::{ScriptedCable, bm820_example, example_reports};

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

    /// The BM820 sheet's example (spec §9.2) through each series that
    /// shares its map, on its own request.
    #[test]
    fn the_bm820_map_serves_two_series() {
        for series in [Series::Bm82x, Series::Bm52x] {
            let reply = bm820_example(series.code());
            let reports: Vec<Vec<u8>> = reply.chunks(8).map(<[u8]>::to_vec).collect();
            let request = reply::request(series.code());
            let cable = ScriptedCable::new(request, reports, Vec::new());
            let mut proto = Bm86xProtocol::new(series, "test");
            let m = proto.request_measurement(&cable).unwrap();
            assert_eq!(m.mode, "AC V", "{series:?}");
            assert_eq!(m.display_raw.as_deref(), Some("380.1"), "{series:?}");
            assert_eq!(cable.written.borrow().as_slice(), [request.to_vec()]);
            assert_eq!(proto.profile().family_name, series.family_name());
        }
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
    fn each_series_has_its_registry_entry() {
        for (series, family, name, aliases, fingerprint) in [
            (
                Series::Bm86x,
                DeviceFamily::Bm86x,
                "Brymen BM869s/BM867s",
                &["bm869s", "BM867s", "brymen-bm869s", "brymen-bm867s"][..],
                &FINGERPRINT_86,
            ),
            (
                Series::Bm82x,
                DeviceFamily::Bm82x,
                "Brymen BM829s/BM827s/BM822s/BM821s",
                &[
                    "bm829s",
                    "BM827s",
                    "bm822s",
                    "bm821s",
                    "brymen-bm829s",
                    "brymen-bm827s",
                    "brymen-bm822s",
                    "brymen-bm821s",
                ],
                &FINGERPRINT_82,
            ),
            (
                Series::Bm52x,
                DeviceFamily::Bm52x,
                "Brymen BM525s/BM521s",
                &["bm525s", "BM521s", "brymen-bm525s", "brymen-bm521s"],
                &FINGERPRINT_52,
            ),
        ] {
            let entry = registry::find_device(series.id()).expect("registry entry");
            assert!(std::ptr::eq(entry, devices::entry(series)));
            assert_eq!(entry.family, family);
            assert_eq!(entry.display_name, name);
            assert_eq!(entry.links, [crate::transport::bu86x::NAME]);
            assert!(entry.bluetooth_names.is_empty());
            for alias in aliases {
                assert_eq!(
                    registry::resolve_device(alias).map(|d| d.id),
                    Some(series.id()),
                    "{alias}"
                );
            }
            let proto = (entry.new_protocol)();
            let profile = proto.profile();
            assert_eq!(profile.model_name, entry.display_name);
            assert_eq!(profile.max_aux_values, 1);
            assert_eq!(profile.stability, Stability::Experimental);
            assert!(profile.supported_commands.is_empty());
            assert!(std::ptr::eq(entry.fingerprint.unwrap(), fingerprint));
            assert_eq!(fingerprint.family, family);
        }
    }

    fn found(buf: &[u8], series: Series) -> bool {
        recognise(buf, series).is_some()
    }

    #[test]
    fn recognise_takes_four_model_bytes() {
        assert!(found(&reports_with_v().concat(), Series::Bm86x));
        let mut late = vec![0x00; 13];
        late.extend(reports_with_v().concat());
        assert!(found(&late, Series::Bm86x));
        // Byte 23 alone is the sheet's, but too little to detect on.
        let mut one = reports_with_v().concat();
        one[16..19].fill(0x00);
        assert!(!found(&one, Series::Bm86x));
        assert!(!found(&[0x86; 3], Series::Bm86x));
        assert!(!found(&[], Series::Bm86x));
    }

    /// Each rule takes its own series' model bytes and no other's.
    #[test]
    fn each_series_recognises_its_own_model_bytes() {
        for series in Series::ALL {
            let run = [series.code(); 4];
            for other in Series::ALL {
                assert_eq!(found(&run, other), other == series, "{series:?} {other:?}");
            }
        }
        for (fingerprint, series) in [
            (&FINGERPRINT_86, Series::Bm86x),
            (&FINGERPRINT_82, Series::Bm82x),
            (&FINGERPRINT_52, Series::Bm52x),
        ] {
            let evidence =
                (fingerprint.recognise)(&bm820_example(series.code()), &Probing::default());
            assert!(
                matches!(evidence, Some(Evidence::Model { id, .. }) if id == series.id()),
                "{series:?}"
            );
        }
    }
}
