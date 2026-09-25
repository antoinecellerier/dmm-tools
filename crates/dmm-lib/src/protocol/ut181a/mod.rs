//! UT181A protocol.
//!
//! Streaming protocol: user must manually enable "Communication ON" on the meter.
//! Device streams measurement packets (type 0x02) continuously.
//!
//! Frame format: AB CD len_lo len_hi payload chk_lo chk_hi
//! Length = payload_size + 2 (includes checksum bytes).
//! Checksum = 16-bit LE sum of length + payload bytes.
//!
//! Values are IEEE 754 float32 (LE) with device-sent unit strings.
//! 79 mode words (uint16 LE) with structured nibble encoding.
//!
//! Based on 3 independent community implementations:
//! antage/ut181a (Rust), loblab/ut181a (C++), sigrok uni-t-ut181a (C).
//! See docs/research/ut181/reverse-engineered-protocol.md
//!
//! ## Not Implemented
//!
//! - Recording protocol (commands 0x0A-0x0F): start/stop/retrieve/delete recordings
//! - Saved measurement retrieval (commands 0x07-0x09): get/delete saved readings
//! - SET_REFERENCE command (0x03): setting relative reference value
//! - Saved measurement packet parsing (response type 0x03)
//! - Recording info/data packet parsing (response types 0x04, 0x05)
//! - Reply data parsing (response type 0x72)
//! - Timestamp decoding (packed 32-bit format, protocol spec Section 9)
//! - Bargraph value extraction (detected but not exposed)

mod command;
pub(crate) mod mode;
// `parse` is `pub(crate)` only so `crate::detect`'s tests can reuse the real
// frames pinned in this module's own tests; every item inside it stays
// `pub(super)`.
pub(crate) mod parse;
mod specs;

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::cycle::FlagSetting;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    Choice, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Setting,
    Stability, unsupported_setting,
};
use crate::specs::{ModeSpecInfo, SpecInfo, SpecSheetTable};
use crate::transport::Transport;
pub(crate) use command::set_monitor_frame;
use command::{UT181A_COMMANDS, build_command, build_set_mode};
use log::debug;
use parse::{decode_mode_word, parse_measurement};

/// Protocol implementation for the UT181A.
pub(crate) struct Ut181aProtocol {
    rx_buf: Vec<u8>,
    /// Mode word of the last measurement parsed. SET_MODE, REL and SET_RANGE
    /// are all relative to it — the meter never tells us its dial position
    /// except through the stream, so a command issued before the first
    /// reading takes one itself (`require_last_mode`).
    last_mode_raw: Option<u16>,
    /// Range byte of the last measurement parsed, so "range" steps the ladder
    /// from where the meter actually is rather than restarting at 1.
    last_range_raw: Option<u8>,
    profile: DeviceProfile,
}

impl Default for Ut181aProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut181aProtocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(256),
            last_mode_raw: None,
            last_range_raw: None,
            profile: DeviceProfile {
                family_name: "UT181A",
                model_name: "UNI-T UT181A",
                // Two reporters have confirmed V DC, V AC + Hz and dual-thermocouple
                // temperature on a real meter (issue #5), but the REL / MIN/MAX /
                // Peak / COMP formats and the CP2110 cable have never run against
                // one. The remote commands — SET_MODE, SET_RANGE, REL, MIN/MAX —
                // are traced from the vendor Windows app rather than guessed
                // (research spec §6.1), but no meter has answered one yet, and the
                // reply frame that would say whether it did is itself unverified.
                // PartlyVerified keeps the warning and the badge linking to the
                // verification issue while listing the meter apart from ones
                // nobody has run; README and docs/supported-devices.md say the
                // same.
                stability: Stability::PartlyVerified,
                supported_commands: UT181A_COMMANDS,
                // aux1 + aux2 + COMP High + COMP Low.
                max_aux_values: 4,
                verification_issue: Some(5),
            },
        }
    }

    /// The mode word the last measurement reported, taking a reading first if
    /// none has arrived yet.
    ///
    /// Every mode-relative command needs the dial position, and a fresh
    /// process has none: opening the device only runs `init`, which starts
    /// the stream without reading from it. So a one-shot `dmm-cli command
    /// rel` has to consume one frame itself before it can build the command.
    /// A meter that does not answer surfaces as the read's own error — a
    /// `Timeout` already says the stream is silent, which is the point where
    /// "enable Communication on the meter" is the useful advice.
    fn require_last_mode(&mut self, transport: &dyn Transport) -> Result<u16> {
        match self.last_mode_raw {
            Some(mode) => Ok(mode),
            // `request_measurement` records `last_mode_raw` from every frame
            // it parses, so a successful read always yields a mode word.
            None => Ok(self.request_measurement(transport)?.mode_raw),
        }
    }
}

impl Protocol for Ut181aProtocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // User must enable "Communication ON" on the meter; SET_MONITOR
        // starts the measurement stream.
        debug!("ut181a: sending start-stream command (CMD_CONT_DATA)");
        transport.write(&set_monitor_frame())?;
        debug!("ut181a: init (streaming, manual enable required)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_2byte_le16,
            // Only accept measurement frames (type 0x02)
            |p| {
                report_unknown_frame_type(p);
                p.first() == Some(&parse::RESPONSE_MEASUREMENT)
            },
            FrameErrorRecovery::SkipAndRetry,
            "ut181a",
            &framing::HEADER,
        )?;
        let measurement = parse_measurement(&payload)?;
        // The stream is the only place the meter states its dial position, so
        // record it for the mode-relative commands.
        self.last_mode_raw = Some(measurement.mode_raw);
        self.last_range_raw = Some(measurement.range_raw);
        Ok(measurement)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let frame = match command {
            // 0x12 = button-press command, 0x5A = HOLD button code.
            // antage (the only reference implementation that transmits
            // this) sends the two-byte payload; sending bare [0x12] is
            // untested. Hardware check pending.
            "hold" => build_command(&[0x12, 0x5A]),
            // REL is not its own opcode: it is SET_MODE with nibble 0 flipped
            // between 1 (plain) and 2 (relative), which is also how it turns
            // back off (research spec §6.1).
            "rel" => {
                let current = self.require_last_mode(transport)?;
                if !mode::rel_supported(current) {
                    return Err(Error::UnsupportedCommand(format!(
                        "rel in {} ({current:#06x})",
                        decode_mode_word(current)
                    )));
                }
                build_set_mode(current ^ 0x3)
            }
            // Step the dial's manual range ladder from wherever the meter
            // reported it, wrapping back to the first manual range.
            "range" => {
                let current = self.require_last_mode(transport)?;
                let last = self.last_range_raw.unwrap_or(0);
                let Some(next) = mode::next_manual_range(current, last) else {
                    return Err(Error::UnsupportedCommand(format!(
                        "range in {} ({current:#06x}): fixed-range mode",
                        decode_mode_word(current)
                    )));
                };
                build_command(&[0x02, next])
            }
            "auto" => build_command(&[0x02, 0x00]),
            // SET_MIN_MAX takes a single byte, not the uint32 the community
            // specs list — the vendor app sends two-byte frames (§6.1).
            "minmax" => build_command(&[0x04, 0x01]),
            "exit_minmax" => build_command(&[0x04, 0x00]),
            "monitor" => build_command(&[0x05, 0x01]),
            "save" => build_command(&[0x06]),
            _ => return Err(Error::UnsupportedCommand(command.to_string())),
        };
        self.send_frame(transport, &frame, command)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    /// A REL reading takes its function's spec. Range 0 is auto with no
    /// range reported: a table keyed by range byte has no row for it, though
    /// the mode's spec still shows, while a table of one range answers it.
    fn spec_info(&self, m: &Measurement) -> Option<&'static SpecInfo> {
        if !mode::is_known_range(m.mode_raw, m.range_raw) {
            return None;
        }
        let table = specs::table(mode::plain_word(m.mode_raw))?;
        table.row(m.range_raw).map(|row| &row.spec)
    }

    fn mode_spec_info(&self, m: &Measurement) -> Option<&'static ModeSpecInfo> {
        specs::table(mode::plain_word(m.mode_raw)).map(|table| &table.mode)
    }

    fn spec_sheet(&self) -> Vec<SpecSheetTable> {
        specs::ALL.iter().map(|t| t.sheet_table()).collect()
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        match setting {
            Setting::Mode => mode::mode_choices(current.mode_raw),
            Setting::Range => mode::range_choices(
                current.mode_raw,
                current.range_raw,
                current.flags.auto_range,
            ),
            flag => {
                let Some(flag) = FlagSetting::of(flag) else {
                    return Vec::new();
                };
                let live = flag.state(&current.flags);
                Self::flag_states(flag, current.mode_raw)
                    .iter()
                    .map(|&id| Choice {
                        id,
                        label: Self::flag_label(flag, id),
                        current: id == live,
                    })
                    .collect()
            }
        }
    }

    /// Only words from the dial's own family are sent: the vendor app never
    /// crosses a family boundary, and the meter would refuse it anyway.
    /// Ranges are the same story — SET_RANGE takes an index into the
    /// family's own ladder. Validation happens before the write, so a stray
    /// id costs no I/O.
    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        match setting {
            Setting::Mode => {
                let current = self.require_last_mode(transport)?;
                if !mode::mode_choices(current).iter().any(|c| c.id == id) {
                    return Err(Error::UnsupportedCommand(format!(
                        "mode {id:#06x} is not reachable from {} ({current:#06x}) — turn the dial first",
                        decode_mode_word(current)
                    )));
                }
                let frame = build_set_mode(id);
                self.send_frame(transport, &frame, &format!("mode {}", decode_mode_word(id)))
            }
            // The meter takes the range absolutely, so there is nothing to
            // walk: the id is the SET_RANGE byte, and 0 is auto.
            Setting::Range => {
                let current = self.require_last_mode(transport)?;
                // The reading's own range and auto flag only decide which
                // entry is marked current; validation needs the list alone.
                let ladder = mode::range_choices(current, 0, false);
                if ladder.is_empty() {
                    return Err(Error::UnsupportedCommand(format!(
                        "range in {} ({current:#06x}): fixed-range mode",
                        decode_mode_word(current)
                    )));
                }
                let Some(choice) = ladder.iter().find(|c| c.id == id) else {
                    return Err(Error::UnsupportedCommand(format!(
                        "range {id} is not one of the {} {} offers",
                        ladder.len() - 1,
                        decode_mode_word(current)
                    )));
                };
                // The id came from the ladder, so it fits the command's byte.
                let what = format!("range {}", choice.label);
                self.send_frame(transport, &build_command(&[0x02, id as u8]), &what)
            }
            flag => match FlagSetting::of(flag) {
                Some(flag) => self.select_flag(transport, flag, id),
                None => Err(unsupported_setting(setting)),
            },
        }
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::flags::Flag;
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{CaptureStep, Expect, Need, RangeExpect};

        // Only V DC, V AC and °C have been seen on real hardware (issue #5),
        // so the mark sits on the V DC step alone, not on the whole gate.
        let [vdc, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
            Volts::VDc,
            CaptureStep::basic("vdc", "Set meter to V DC").verified(),
            Ohms::Word,
            CaptureStep::basic(
                "ohm",
                "Set meter to Resistance. Leave leads open (should show OL).",
            ),
        );

        // Core UT181A modes
        vec![
            vdc,
            dcv_short,
            dcv_negative,
            // Each dial family reaches several mode words (spec §6.1); the
            // steps below name them as the meter reports them, one per word,
            // because SET_MODE only ever moves inside the family the dial is
            // already on.
            CaptureStep::basic("vdc_acdc", "Set meter to V DC AC+DC")
                .expect(Expect::mode("V DC AC+DC")),
            CaptureStep::basic("vdc_peak", "Set meter to V DC Peak")
                .expect(Expect::mode("V DC Peak")),
            CaptureStep::basic("vac", "Set meter to V AC")
                .verified()
                .expect(Expect::mode("V AC")),
            CaptureStep::basic("vac_hz", "Set meter to V AC Hz").expect(Expect::mode("V AC Hz")),
            CaptureStep::basic("vac_lpf", "Set meter to V AC LPF").expect(Expect::mode("V AC LPF")),
            CaptureStep::basic("vac_dbv", "Set meter to V AC dBV").expect(Expect::mode("V AC dBV")),
            CaptureStep::basic("vac_dbm", "Set meter to V AC dBm").expect(Expect::mode("V AC dBm")),
            CaptureStep::basic("mvdc", "Set meter to mV DC").expect(Expect::mode("mV DC")),
            CaptureStep::basic("mvdc_peak", "Set meter to mV DC Peak")
                .expect(Expect::mode("mV DC Peak")),
            CaptureStep::basic("mvac", "Set meter to mV AC").expect(Expect::mode("mV AC")),
            CaptureStep::basic("mvac_hz", "Set meter to mV AC Hz").expect(Expect::mode("mV AC Hz")),
            CaptureStep::basic("mvac_peak", "Set meter to mV AC Peak")
                .expect(Expect::mode("mV AC Peak")),
            // The vendor UI offers this one but its own decoder has no label
            // for it, so whether the meter has it is open (spec §6.1
            // "Caveats").
            CaptureStep::basic(
                "mvac_acdc",
                "Set meter to mV AC AC+DC (if the meter has it)",
            )
            .expect(Expect::mode("mV AC AC+DC")),
            ohm,
            ohm_body,
            ohm_short,
            CaptureStep::basic("cont", "Set meter to Continuity")
                .expect(Expect::mode("Continuity")),
            // Nibble 0 = 2 is a second function on these two families, not
            // REL: the open-circuit beeper and the diode alarm (spec §6.1).
            CaptureStep::basic("cont_open", "Continuity: switch to the open-circuit beeper")
                .expect(Expect::mode("Continuity (open)")),
            CaptureStep::basic("ns", "Set meter to Conductance (nS)").expect(Expect::mode("nS")),
            CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
            CaptureStep::basic("diode_alarm", "Diode: switch to the alarm function")
                .expect(Expect::mode("Diode Alarm")),
            CaptureStep::basic("cap", "Set meter to Capacitance")
                .expect(Expect::mode("Capacitance")),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Hz")),
            CaptureStep::basic("duty", "Set meter to Duty Cycle (%)")
                .expect(Expect::mode("Duty %")),
            CaptureStep::basic("pulse", "Set meter to Pulse Width (ms)")
                .expect(Expect::mode("Pulse Width")),
            CaptureStep::basic("uadc", "Set meter to µA DC").expect(Expect::mode("µA DC")),
            CaptureStep::basic("uadc_acdc", "Set meter to µA DC AC+DC")
                .expect(Expect::mode("µA DC AC+DC")),
            CaptureStep::basic("uadc_peak", "Set meter to µA DC Peak")
                .expect(Expect::mode("µA DC Peak")),
            CaptureStep::basic("uaac", "Set meter to µA AC").expect(Expect::mode("µA AC")),
            CaptureStep::basic("uaac_hz", "Set meter to µA AC Hz").expect(Expect::mode("µA AC Hz")),
            CaptureStep::basic("uaac_peak", "Set meter to µA AC Peak")
                .expect(Expect::mode("µA AC Peak")),
            CaptureStep::basic("madc", "Set meter to mA DC").expect(Expect::mode("mA DC")),
            CaptureStep::basic("madc_acdc", "Set meter to mA DC AC+DC")
                .expect(Expect::mode("mA DC AC+DC")),
            CaptureStep::basic("madc_peak", "Set meter to mA DC Peak")
                .expect(Expect::mode("mA DC Peak")),
            CaptureStep::basic("maac", "Set meter to mA AC").expect(Expect::mode("mA AC")),
            CaptureStep::basic("maac_hz", "Set meter to mA AC Hz").expect(Expect::mode("mA AC Hz")),
            CaptureStep::basic("maac_peak", "Set meter to mA AC Peak")
                .expect(Expect::mode("mA AC Peak")),
            CaptureStep::basic("adc", "Set meter to A DC").expect(Expect::mode("A DC")),
            CaptureStep::basic("adc_acdc", "Set meter to A DC AC+DC")
                .expect(Expect::mode("A DC AC+DC")),
            CaptureStep::basic("adc_peak", "Set meter to A DC Peak")
                .expect(Expect::mode("A DC Peak")),
            CaptureStep::basic("aac", "Set meter to A AC").expect(Expect::mode("A AC")),
            CaptureStep::basic("aac_hz", "Set meter to A AC Hz").expect(Expect::mode("A AC Hz")),
            CaptureStep::basic("aac_peak", "Set meter to A AC Peak")
                .expect(Expect::mode("A AC Peak")),
            CaptureStep::basic("tempc", "Set meter to Temperature C")
                .verified()
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°C")),
            // Nibble 1 is the probe arrangement on the temperature families,
            // so T2 and the two differentials need the second probe in T2.
            CaptureStep::basic(
                "tempc_t2",
                "Temperature C: switch the main display to T2 (second thermocouple in T2)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C T2")),
            CaptureStep::basic(
                "tempc_t1_t2",
                "Temperature C: switch to the T1-T2 difference (two thermocouples)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C T1-T2")),
            CaptureStep::basic(
                "tempc_t2_t1",
                "Temperature C: switch to the T2-T1 difference (two thermocouples)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C T2-T1")),
            CaptureStep::basic("tempf", "Set meter to Temperature F")
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°F")),
            CaptureStep::basic(
                "tempf_t2",
                "Temperature F: switch the main display to T2 (second thermocouple in T2)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F T2")),
            CaptureStep::basic(
                "tempf_t1_t2",
                "Temperature F: switch to the T1-T2 difference (two thermocouples)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F T1-T2")),
            CaptureStep::basic(
                "tempf_t2_t1",
                "Temperature F: switch to the T2-T1 difference (two thermocouples)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F T2-T1")),
            // Remote command steps
            CaptureStep::with_command("hold", "V DC mode: we will send HOLD.", "hold", 3)
                .expect(Expect::new().flags(&[(Flag::Hold, true)])),
            CaptureStep::with_command(
                "hold_off",
                "We will send HOLD again to turn it off.",
                "hold",
                3,
            )
            .expect(Expect::new().flags(&[(Flag::Hold, false)])),
            CaptureStep::with_command("minmax", "We will enable MIN/MAX.", "minmax", 3)
                .expect(Expect::new().flags(&[(Flag::Min, true), (Flag::Max, true)])),
            CaptureStep::with_command("minmax_off", "We will disable MIN/MAX.", "exit_minmax", 3)
                .expect(Expect::new().flags(&[(Flag::Min, false), (Flag::Max, false)])),
            CaptureStep::with_command("auto", "We will set auto-range.", "auto", 3)
                .expect(Expect::new().range(RangeExpect::Auto)),
            // Format variant verification steps
            CaptureStep::basic(
                "rel",
                "V DC mode: long-press REL to enable relative. \
                              The report should list Reference and Absolute \
                              sub-values under each sample.",
            )
            .expect(Expect::new().flags(&[(Flag::Rel, true)])),
            CaptureStep::basic("rel_off", "Long-press REL again to disable relative mode.")
                .samples(3)
                .expect(Expect::new().flags(&[(Flag::Rel, false)])),
            CaptureStep::basic(
                "peak",
                "V AC mode: enable Peak mode (FUNC button). \
                              The report should list a Peak Min sub-value \
                              under each sample.",
            )
            .expect(Expect::new().flags(&[(Flag::PeakMax, true), (Flag::PeakMin, true)])),
            CaptureStep::basic("peak_off", "Disable Peak mode.")
                .samples(3)
                .expect(Expect::new().flags(&[(Flag::PeakMax, false), (Flag::PeakMin, false)])),
            CaptureStep::basic(
                "manual_range",
                "V DC mode: press RANGE to switch to manual range. \
                              Verify range_label shows the selected range (e.g. 60V).",
            )
            .samples(3)
            .expect(Expect::new().range(RangeExpect::Manual)),
        ]
    }
}

/// Report a checksummed frame whose type research spec §4.1 does not list.
///
/// For the host's own reads only: detection scans bytes other meters sent.
fn report_unknown_frame_type(payload: &[u8]) {
    match payload.first() {
        Some(0x01..=0x05 | 0x72) => {}
        Some(kind) => report_unknown(
            "ut181a",
            "frame type",
            format_args!("{kind:#04x}, {} bytes", payload.len()),
        ),
        None => report_unknown("ut181a", "frame type", format_args!("none, 0 bytes")),
    }
}

/// Detection for the UT181A.
///
/// SET_MONITOR is what makes a UT181A speak at all, and the frames it then
/// streams are framed exactly like a UT171's. Two things claim one: a payload
/// only a UT181A can send, and any measurement frame that arrived while
/// SET_MONITOR was the last trigger out. Everything shorter and later is left
/// to the UT171, which detection consults next.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Ut181a,
    label: "ut181a set monitor",
    trigger: Some(send_set_monitor),
    send_after: &[],
    checksummed: true,
    recognise,
};

/// Start the measurement stream — the same frame [`Protocol::init`] writes.
fn send_set_monitor(transport: &dyn Transport) -> Result<()> {
    transport.write(&set_monitor_frame())
}

fn recognise(buf: &[u8], probing: &Probing) -> Option<Evidence> {
    for start in framing::abcd_header_offsets(buf) {
        let Ok(Some((payload, _))) = framing::extract_frame_abcd_2byte_le16(&buf[start..]) else {
            continue;
        };
        if payload.first() != Some(&parse::RESPONSE_MEASUREMENT) {
            // Logged once here rather than in both LE16 recognisers: an OK/ER
            // reply names no model, but it does say something answered.
            debug!(
                "detect: ignoring LE16 frame of type {:#04x}",
                payload.first().copied().unwrap_or(0)
            );
            continue;
        }
        if payload.len() >= parse::EXCLUSIVE_PAYLOAD_MIN
            || probing.last() == Some(DeviceFamily::Ut181a)
        {
            return Some(Evidence::Model {
                id: "ut181a",
                reported_name: None,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse::tests::{hex, make_payload, make_relative_payload, minmax_payload};
    use super::*;

    #[test]
    fn init_sends_start_stream_command() {
        use crate::transport::mock::MockTransport;
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut181aProtocol::new();
        proto.init(&mock).unwrap();
        let written = mock.written.borrow();
        // CMD_CONT_DATA: AB CD 04 00 05 01 0A 00
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0],
            vec![0xAB, 0xCD, 0x04, 0x00, 0x05, 0x01, 0x0A, 0x00]
        );
    }

    // --- Remote control: mode selection, REL, range, replies ---------------
    //
    // Every command below is traced from the vendor Windows app and has never
    // run against a meter (research spec §6.1), so these tests pin the bytes
    // we chose to send, not behaviour anyone has observed.

    use crate::transport::mock::MockTransport;

    /// A protocol that has already parsed one measurement, so the
    /// mode-relative commands have a dial position to work from.
    pub(super) fn proto_in(mode_word: u16, range: u8) -> (Ut181aProtocol, MockTransport) {
        let mut payload = make_payload(mode_word, 1.0, 0x20, b"VDC\0\0\0\0\0", 0x00, 0x01);
        payload[5] = range;
        let mock = MockTransport::new(vec![build_command(&payload)]);
        let mut proto = Ut181aProtocol::new();
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.mode_raw, mode_word);
        assert_eq!(m.range_raw, range);
        (proto, mock)
    }

    /// The single frame a command wrote.
    fn only_write(mock: &MockTransport) -> Vec<u8> {
        let written = mock.written.borrow();
        assert_eq!(
            written.len(),
            1,
            "expected exactly one write: {written:02X?}"
        );
        written[0].clone()
    }

    #[test]
    fn select_mode_sends_set_mode_with_the_mode_word() {
        let (mut proto, mock) = proto_in(0x1111, 0);
        proto.select(&mock, Setting::Mode, 0x1121).unwrap();
        // AB CD | len 05 00 | 01 (SET_MODE) 21 11 (0x1121 LE) | checksum
        // 05+00+01+21+11 = 0x38.
        assert_eq!(
            only_write(&mock),
            hex("AB CD 05 00 01 21 11 38 00"),
            "SET_MODE 0x1121"
        );
    }

    #[test]
    fn select_mode_refuses_a_word_from_another_family() {
        let (mut proto, mock) = proto_in(0x1111, 0);
        // V DC is a different dial position: the meter can't get there on its own.
        let err = proto.select(&mock, Setting::Mode, 0x3111).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCommand(_)),
            "got {err:?}, want UnsupportedCommand"
        );
        assert!(
            mock.written.borrow().is_empty(),
            "a rejected id must cost no I/O"
        );
    }

    /// With no reading yet the command takes one itself, so a silent stream
    /// surfaces as the read's own timeout — and nothing is sent on a guess.
    #[test]
    fn select_mode_without_a_reading_fails_on_the_read() {
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut181aProtocol::new();
        let err = proto.select(&mock, Setting::Mode, 0x1121).unwrap_err();
        assert!(matches!(err, Error::Timeout), "got {err:?}, want Timeout");
        assert!(mock.written.borrow().is_empty());
    }

    /// REL is SET_MODE with nibble 0 flipped, in both directions.
    #[test]
    fn rel_toggles_nibble_zero_of_the_mode_word() {
        let (mut proto, mock) = proto_in(0x1111, 0);
        proto.send_command(&mock, "rel").unwrap();
        // 05+00+01+12+11 = 0x29.
        assert_eq!(only_write(&mock), hex("AB CD 05 00 01 12 11 29 00"));

        let (mut proto, mock) = proto_in(0x1112, 0);
        proto.send_command(&mock, "rel").unwrap();
        // 05+00+01+11+11 = 0x28.
        assert_eq!(only_write(&mock), hex("AB CD 05 00 01 11 11 28 00"));
    }

    /// A fresh process has never read the stream — opening the device only
    /// runs `init` — so a one-shot `command rel` takes a reading itself to
    /// learn which mode word to toggle.
    #[test]
    fn rel_reads_the_mode_when_none_has_arrived_yet() {
        let frame = build_command(&make_payload(
            0x1111,
            1.0,
            0x20,
            b"VAC\0\0\0\0\0",
            0x00,
            0x01,
        ));
        let mock = MockTransport::new(vec![frame]);
        let mut proto = Ut181aProtocol::new();

        proto.send_command(&mock, "rel").unwrap();

        // 0x1111 -> 0x1112. A fresh protocol has no other source for that
        // word than the queued frame, so the read landed before the write.
        assert_eq!(only_write(&mock), hex("AB CD 05 00 01 12 11 29 00"));
        assert_eq!(proto.last_mode_raw, Some(0x1111));
    }

    /// ...and with the stream silent, the read's timeout is the error: no
    /// frame goes out carrying a guessed mode word.
    #[test]
    fn rel_without_a_reading_fails_on_the_read() {
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut181aProtocol::new();
        let err = proto.send_command(&mock, "rel").unwrap_err();
        assert!(matches!(err, Error::Timeout), "got {err:?}, want Timeout");
        assert!(mock.written.borrow().is_empty());
    }

    /// REL rides on the variant, not only on the plain mode — the vendor app
    /// offers it on LowPass, dB and AC+DC too (research spec §6.1).
    #[test]
    fn rel_works_on_the_other_rel_capable_variants() {
        // V AC LowPass 0x1141 -> 0x1142. 05+00+01+42+11 = 0x59.
        let (mut proto, mock) = proto_in(0x1141, 0);
        proto.send_command(&mock, "rel").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 05 00 01 42 11 59 00"));

        // V DC AC+DC 0x3121 -> 0x3122. 05+00+01+22+31 = 0x59.
        let (mut proto, mock) = proto_in(0x3121, 0);
        proto.send_command(&mock, "rel").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 05 00 01 22 31 59 00"));
    }

    /// ...and is withheld on every Hz and Peak variant, on the differential
    /// temperature arrangements, and on continuity and diode — which spend
    /// nibble 0 = 2 on the open beeper and the alarm instead.
    #[test]
    fn rel_is_unsupported_where_the_vendor_disables_it() {
        for word in [
            0x1121, // V AC Hz
            0x1131, // V AC Peak
            0x4231, // °C T1-T2
            0x8221, // µA AC Hz
            0x5211, // Continuity
            0x6111, // Diode
        ] {
            let (mut proto, mock) = proto_in(word, 0);
            let err = proto.send_command(&mock, "rel").unwrap_err();
            assert!(
                matches!(err, Error::UnsupportedCommand(_)),
                "{word:#06x}: got {err:?}, want UnsupportedCommand"
            );
            assert!(
                mock.written.borrow().is_empty(),
                "{word:#06x} wrote a frame"
            );
        }
    }

    #[test]
    fn range_steps_the_manual_ladder_and_wraps() {
        // V DC has four manual ranges; auto (0) steps to the first.
        let (mut proto, mock) = proto_in(0x3111, 0);
        proto.send_command(&mock, "range").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 02 01 07 00"));

        // The top of the ladder wraps back to 1, not to auto.
        let (mut proto, mock) = proto_in(0x3111, 4);
        proto.send_command(&mock, "range").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 02 01 07 00"));

        // Mid-ladder: 1 -> 2.
        let (mut proto, mock) = proto_in(0x3111, 1);
        proto.send_command(&mock, "range").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 02 02 08 00"));
    }

    #[test]
    fn range_is_unsupported_on_a_fixed_range_mode() {
        let (mut proto, mock) = proto_in(0x4211, 0); // °C
        let err = proto.send_command(&mock, "range").unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCommand(_)),
            "got {err:?}, want UnsupportedCommand"
        );
        assert!(mock.written.borrow().is_empty());
    }

    /// SET_MIN_MAX takes one byte, not the uint32 the community specs list.
    #[test]
    fn minmax_sends_a_single_argument_byte() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        proto.send_command(&mock, "minmax").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 04 01 09 00"));

        let (mut proto, mock) = proto_in(0x3111, 0);
        proto.send_command(&mock, "exit_minmax").unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 04 00 08 00"));
    }

    #[test]
    fn mode_choices_lists_the_dial_family() {
        let (proto, _mock) = proto_in(0x1111, 0);
        let m = make_payload(0x1111, 1.0, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&m).unwrap();
        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(choices.len(), 6, "V AC: plain, Hz, Peak, LPF, dBV, dBm");
        assert_eq!(choices[0].label, "V AC");
        let current: Vec<u16> = choices.iter().filter(|c| c.current).map(|c| c.id).collect();
        assert_eq!(current, vec![0x1111]);
    }

    #[test]
    fn mode_choices_covers_the_temperature_arrangements() {
        let (proto, _mock) = proto_in(0x4211, 0);
        let m = parse_measurement(&make_payload(
            0x4231,
            1.0,
            0x10,
            b"\xB0C\0\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();
        let choices = proto.choices(Setting::Mode, &m);
        let labels: Vec<&str> = choices.iter().map(|c| c.label.as_ref()).collect();
        assert_eq!(labels, vec!["°C", "°C T2", "°C T1-T2", "°C T2-T1"]);
        let current: Vec<u16> = choices.iter().filter(|c| c.current).map(|c| c.id).collect();
        assert_eq!(current, vec![0x4231]);
    }

    #[test]
    fn mode_choices_pairs_continuity_with_its_open_beeper() {
        let (proto, _mock) = proto_in(0x5211, 0);
        let m = parse_measurement(&make_payload(
            0x5212,
            1.0,
            0x20,
            b"~\0\0\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();
        let choices = proto.choices(Setting::Mode, &m);
        let labels: Vec<&str> = choices.iter().map(|c| c.label.as_ref()).collect();
        assert_eq!(labels, vec!["Continuity", "Continuity (open)"]);
        // Nibble 0 = 2 is a second function here, so it is its own choice —
        // not the REL companion of the first.
        let current: Vec<u16> = choices.iter().filter(|c| c.current).map(|c| c.id).collect();
        assert_eq!(current, vec![0x5212]);
    }

    /// With REL on, the meter reports the companion word (0x1112, 0x1142…).
    /// The choice list still has to point at the variant underneath it, or
    /// the UI would show no mode selected.
    #[test]
    fn mode_choices_flags_the_variant_behind_an_active_rel() {
        for (reported, expected) in [(0x1112u16, 0x1111u16), (0x1142, 0x1141)] {
            let (proto, _mock) = proto_in(reported, 0);
            let m = parse_measurement(&make_payload(
                reported,
                1.0,
                0x20,
                b"VAC\0\0\0\0\0",
                0x00,
                0x01,
            ))
            .unwrap();
            let choices = proto.choices(Setting::Mode, &m);
            let current: Vec<u16> = choices.iter().filter(|c| c.current).map(|c| c.id).collect();
            assert_eq!(current, vec![expected], "from {reported:#06x}");
        }
    }

    // --- Range selection --------------------------------------------------

    #[test]
    fn v_ac_offers_auto_and_its_four_ranges() {
        let (proto, _mock) = proto_in(0x1111, 2);
        let m = parse_measurement(&{
            let mut p = make_payload(0x1111, 1.0, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
            p[5] = 2;
            p
        })
        .unwrap();
        let choices = proto.choices(Setting::Range, &m);
        let listed: Vec<(u16, &str)> = choices.iter().map(|c| (c.id, c.label.as_ref())).collect();
        assert_eq!(
            listed,
            vec![
                (0, "Auto"),
                (1, "6V"),
                (2, "60V"),
                (3, "600V"),
                (4, "1000V")
            ]
        );
        // misc2 bit 0 clear = manual, so the reported range byte is current.
        assert_eq!(
            choices
                .iter()
                .filter(|c| c.current)
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn a_fixed_range_family_offers_nothing() {
        let (proto, _mock) = proto_in(0x1111, 0);
        // 0x4211: temperature, whose ranges the meter does not switch.
        let m = parse_measurement(&make_payload(
            0x4211,
            20.0,
            0x20,
            b"C\0\0\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();
        assert!(proto.choices(Setting::Range, &m).is_empty());
    }

    /// Duty cycle has a four-item vendor ladder but no rung labels: a blank
    /// entry can be neither picked nor confirmed, so nothing is offered.
    #[test]
    fn an_unnamed_ladder_offers_nothing() {
        let (proto, _mock) = proto_in(0x7211, 0);
        let m = parse_measurement(&make_payload(
            0x7211,
            50.0,
            0x20,
            b"%\0\0\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();
        assert!(proto.choices(Setting::Range, &m).is_empty());
        assert!(
            mode::range_choices(0x7311, 0, true).is_empty(),
            "pulse width"
        );
    }

    #[test]
    fn selecting_a_range_sends_set_range_with_the_rung() {
        let (mut proto, mock) = proto_in(0x1111, 1);
        proto.select(&mock, Setting::Range, 3).unwrap();
        // AB CD | len 04 00 | 02 (SET_RANGE) 03 | checksum 04+00+02+03 = 0x09.
        assert_eq!(only_write(&mock), hex("AB CD 04 00 02 03 09 00"));
    }

    #[test]
    fn selecting_auto_sends_set_range_zero() {
        let (mut proto, mock) = proto_in(0x1111, 2);
        proto.select(&mock, Setting::Range, 0).unwrap();
        assert_eq!(only_write(&mock), hex("AB CD 04 00 02 00 06 00"));
    }

    #[test]
    fn a_rung_the_family_lacks_costs_no_io() {
        let (mut proto, mock) = proto_in(0x1111, 0);
        let err = proto.select(&mock, Setting::Range, 5).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("not one of the 4")),
            "got {err:?}"
        );
        assert!(mock.written.borrow().is_empty());
    }

    #[test]
    fn a_fixed_range_mode_refuses_a_range_without_sending() {
        let (mut proto, mock) = proto_in(0x4211, 0);
        let err = proto.select(&mock, Setting::Range, 1).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("fixed-range mode")),
            "got {err:?}"
        );
        assert!(mock.written.borrow().is_empty());
    }

    #[test]
    fn an_er_reply_rejects_a_range_selection() {
        let (mut proto, mock) = proto_in(0x1111, 1);
        mock.push_response(build_command(&[0x01, b'E', b'R']));
        let err = proto.select(&mock, Setting::Range, 3).unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(_)),
            "got {err:?}, want CommandRejected"
        );
    }

    // --- Flag-backed settings (HOLD, REL, MIN/MAX) ------------------------

    /// The OK reply the meter answers a command with (spec §4.1).
    fn ok_reply() -> Vec<u8> {
        build_command(&[0x01, b'O', b'K'])
    }

    fn er_reply() -> Vec<u8> {
        build_command(&[0x01, b'E', b'R'])
    }

    /// A plain V DC frame, with `misc` carrying the HOLD bit when set.
    fn plain_frame(mode: u16, misc: u8) -> Vec<u8> {
        build_command(&make_payload(mode, 1.0, 0x20, b"VDC\0\0\0\0\0", misc, 0x01))
    }

    #[test]
    fn hold_and_minmax_are_offered_as_a_plain_switch() {
        let proto = Ut181aProtocol::new();
        let m = parse_measurement(&make_payload(
            0x1111,
            1.0,
            0x20,
            b"VDC\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();

        for setting in [Setting::Hold, Setting::MinMax] {
            let choices = proto.choices(setting, &m);
            assert_eq!(
                choices.iter().map(|c| c.id).collect::<Vec<_>>(),
                vec![0, 1],
                "{setting}"
            );
            assert_eq!(
                choices.iter().map(|c| c.label.as_ref()).collect::<Vec<_>>(),
                vec!["off", "on"],
                "{setting}"
            );
            assert!(choices[0].current, "{setting} is off");
        }
        // Peak is a mode variant on this meter, not a setting of its own.
        assert!(proto.choices(Setting::Peak, &m).is_empty());
    }

    #[test]
    fn a_minmax_reading_marks_the_on_state() {
        let proto = Ut181aProtocol::new();
        let m = parse_measurement(&minmax_payload(0x3111)).unwrap();
        assert!(proto.choices(Setting::MinMax, &m)[1].current);
    }

    /// Continuity is one of the functions the vendor app disables REL on.
    #[test]
    fn rel_is_not_offered_where_the_vendor_disables_it() {
        let proto = Ut181aProtocol::new();
        let cont = parse_measurement(&make_payload(
            0x5121,
            1.0,
            0x20,
            b"\0\0\0\0\0\0\0\0",
            0x00,
            0x01,
        ))
        .unwrap();
        assert!(!mode::rel_supported(cont.mode_raw));
        assert!(proto.choices(Setting::Rel, &cont).is_empty());
    }

    #[test]
    fn selecting_minmax_sends_the_single_argument_byte() {
        let mock = MockTransport::new(vec![
            plain_frame(0x3111, 0x00),
            ok_reply(),
            build_command(&minmax_payload(0x3111)),
        ]);
        let mut proto = Ut181aProtocol::new();
        proto.select(&mock, Setting::MinMax, 1).expect("recording");
        // AB CD | len 04 00 | 04 (SET_MIN_MAX) 01 | checksum 04+00+04+01 = 0x09.
        assert_eq!(
            only_write(&mock),
            vec![0xAB, 0xCD, 0x04, 0x00, 0x04, 0x01, 0x09, 0x00]
        );
    }

    #[test]
    fn selecting_hold_sends_the_button_press() {
        let mock = MockTransport::new(vec![
            plain_frame(0x3111, 0x00),
            ok_reply(),
            plain_frame(0x3111, 0x80),
        ]);
        let mut proto = Ut181aProtocol::new();
        proto.select(&mock, Setting::Hold, 1).expect("held");
        assert_eq!(
            only_write(&mock),
            vec![0xAB, 0xCD, 0x04, 0x00, 0x12, 0x5A, 0x70, 0x00]
        );
    }

    #[test]
    fn selecting_rel_flips_nibble_zero_of_the_mode_word() {
        let mock = MockTransport::new(vec![
            plain_frame(0x1111, 0x00),
            ok_reply(),
            // The REL frame format (misc format_type 1) is what lights REL.
            build_command(&make_relative_payload(0x1112, 2.0, 10.0, 12.0)),
        ]);
        let mut proto = Ut181aProtocol::new();
        proto.select(&mock, Setting::Rel, 1).expect("relative");
        // SET_MODE 0x1112, the REL companion of 0x1111.
        assert_eq!(
            only_write(&mock),
            vec![0xAB, 0xCD, 0x05, 0x00, 0x01, 0x12, 0x11, 0x29, 0x00]
        );
    }

    #[test]
    fn a_setting_already_in_the_state_sends_nothing() {
        let mock = MockTransport::new(vec![plain_frame(0x3111, 0x80)]);
        let mut proto = Ut181aProtocol::new();
        proto.select(&mock, Setting::Hold, 1).expect("already held");
        assert!(mock.written.borrow().is_empty());
    }

    #[test]
    fn an_er_reply_rejects_a_hold() {
        let mock = MockTransport::new(vec![plain_frame(0x3111, 0x00), er_reply()]);
        let mut proto = Ut181aProtocol::new();
        let err = proto.select(&mock, Setting::Hold, 1).unwrap_err();
        assert!(matches!(err, Error::CommandRejected(_)), "{err}");
    }

    #[test]
    fn a_state_the_meter_lacks_costs_no_io() {
        let (mut proto, mock) = proto_in(0x1111, 0);
        mock.written.borrow_mut().clear();
        let err = proto.select(&mock, Setting::MinMax, 2).unwrap_err();
        assert!(matches!(err, Error::UnsupportedCommand(_)), "{err}");
        assert!(mock.written.borrow().is_empty());
    }

    /// Every command the profile advertises must be accepted from a mode that
    /// supports it, and nothing else may be.
    #[test]
    fn advertised_commands_are_accepted() {
        for &cmd in UT181A_COMMANDS {
            let (mut proto, mock) = proto_in(0x3111, 0);
            assert!(
                proto.send_command(&mock, cmd).is_ok(),
                "UT181A_COMMANDS lists '{cmd}' but send_command rejects it in V DC"
            );
        }
        let (mut proto, mock) = proto_in(0x3111, 0);
        assert!(proto.send_command(&mock, "nonexistent").is_err());
    }

    // --- Detection (crate::detect) ---------------------------------------

    /// Whichever trigger went out last, expressed as the engine records it.
    fn after(family: DeviceFamily) -> Probing {
        Probing {
            sent: vec![family],
            ..Probing::default()
        }
    }

    fn recognised(buf: &[u8], probing: &Probing) -> Option<Evidence> {
        (FINGERPRINT.recognise)(buf, probing)
    }

    fn ut181a() -> Option<Evidence> {
        Some(Evidence::Model {
            id: "ut181a",
            reported_name: None,
        })
    }

    /// The real frames this module's parser pins are 32 and 57 payload bytes,
    /// both past the length only a UT181A reaches — so no context is needed
    /// to claim them.
    #[test]
    fn a_long_frame_is_a_ut181a_whatever_elicited_it() {
        for payload in [
            parse::tests::real_frame_temp_dual_probe(),
            parse::tests::real_frame_vac_hz(),
        ] {
            let frame = framing::test_frame_le16(&payload);
            for probing in [Probing::default(), after(DeviceFamily::Ut171)] {
                assert_eq!(
                    recognised(&frame, &probing),
                    ut181a(),
                    "payload {}",
                    payload.len()
                );
            }
        }
    }

    /// A UT181A in its shortest normal format is 19 payload bytes — inside
    /// the UT171's range, so only its own trigger tells the two apart.
    #[test]
    fn a_short_frame_is_a_ut181a_only_after_set_monitor() {
        let mut payload = vec![0u8; 19];
        payload[0] = parse::RESPONSE_MEASUREMENT;
        let frame = framing::test_frame_le16(&payload);
        assert_eq!(recognised(&frame, &after(DeviceFamily::Ut181a)), ut181a());
        assert_eq!(recognised(&frame, &after(DeviceFamily::Ut171)), None);
        assert_eq!(recognised(&frame, &Probing::default()), None);
    }

    /// An OK/ER reply names no model, whatever asked for it.
    #[test]
    fn a_command_reply_identifies_nothing() {
        let reply = framing::test_frame_le16(&[0x01, 0x4F, 0x4B]);
        assert_eq!(recognised(&reply, &after(DeviceFamily::Ut181a)), None);
    }

    // --- Unrecognised data (protocol::unrecognised) -----------------------

    use crate::protocol::capture_reports;

    /// The stream skips every frame that is not a reading; the documented
    /// types (§4.1) go by silently, any other is reported.
    #[test]
    fn a_frame_of_an_undocumented_type_is_reported_and_skipped() {
        let reading = make_payload(0x3111, 1.0, 0x20, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let frames: [&[u8]; 9] = [
            &[0x01, b'O', b'K'],
            &[0x03, 0x00],
            &[0x04, 0x00],
            &[0x05, 0x00],
            &[0x72, 0x00, 0x00],
            &[0x06, 0x01, 0x02],
            &[0x00, 0x00],
            &[],
            &reading,
        ];
        let mock = MockTransport::new(frames.iter().map(|p| build_command(p)).collect());
        let mut proto = Ut181aProtocol::new();
        let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().mode_raw, 0x3111);
        assert_eq!(
            reports,
            [
                "ut181a: unrecognised frame type: 0x06, 3 bytes",
                "ut181a: unrecognised frame type: 0x00, 2 bytes",
                "ut181a: unrecognised frame type: none, 0 bytes",
            ]
        );
    }

    /// Detection reads bytes other meters sent, so it reports nothing.
    #[test]
    fn detection_reports_no_frame_type() {
        let mut buf = framing::test_frame_le16(&[0x06, 0x01, 0x02]);
        buf.extend(framing::test_frame_le16(&[0x00; 40]));
        let (evidence, reports) =
            capture_reports(|| recognised(&buf, &after(DeviceFamily::Ut181a)));
        assert_eq!(evidence, None);
        assert!(reports.is_empty(), "{reports:?}");
    }
}
