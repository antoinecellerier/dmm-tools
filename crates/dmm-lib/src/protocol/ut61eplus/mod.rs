mod capture;
pub(crate) mod command;
pub(crate) mod devices;
pub(crate) mod mode;
pub(crate) mod specs;
pub(crate) mod tables;

use crate::error::{Error, Result, Unsupported};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MainLabel, MeasuredValue, Measurement};
use crate::protocol::command as cmd;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::registry::{DEVICES, SelectableDevice};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    Choice, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKey, MeterKeys, Probing,
    Protocol, Setting, Stability, check_len, cycle, display_number, spells_ol, unknown_mode,
    unknown_mode16, unsupported_setting,
};
use crate::transport::{Link, Transport};
use command::Command;
use log::{debug, warn};
use mode::Mode;
use specs::SpecModel;
use std::borrow::Cow;
use std::time::{Duration, Instant};
use tables::ModelTable;

/// The commands the family takes; the mock, a UT61E+, offers the same.
pub(crate) const UT61EPLUS_COMMANDS: &[&str] = &[
    cmd::HOLD,
    cmd::MINMAX,
    cmd::EXIT_MINMAX,
    cmd::RANGE,
    cmd::AUTO,
    cmd::REL,
    "select2",
    cmd::SELECT,
    cmd::LIGHT,
    cmd::PEAK,
    cmd::EXIT_PEAK,
];

/// The UT61B+ and UT161B's commands: the family's, less Peak, which their
/// manual doesn't give them (UT61+ manual p.10; family spec §6).
pub(crate) const UT61BPLUS_COMMANDS: &[&str] = &[
    cmd::HOLD,
    cmd::MINMAX,
    cmd::EXIT_MINMAX,
    cmd::RANGE,
    cmd::AUTO,
    cmd::REL,
    "select2",
    cmd::SELECT,
    cmd::LIGHT,
];

/// The modes on a Hz/% ring, the same on every dial here (family spec §3.1):
/// the key does nothing in any other.
const HZ_PERCENT_RING: [u16; 7] = [
    tables::m(Mode::AcV),
    tables::m(Mode::AcMv),
    tables::m(Mode::AcUa),
    tables::m(Mode::AcMa),
    tables::m(Mode::AcA),
    tables::m(Mode::Hz),
    tables::m(Mode::DutyCycle),
];

/// The Hz/% key, printed Hz% over USB (UT61+ manual p.10): its short press, as
/// no command reaches the long one.
static HZ_PERCENT: MeterKey = MeterKey {
    command: "select2",
    label: "Hz/%",
    in_words: false,
    hover: None,
    applies: |m| HZ_PERCENT_RING.contains(&m.mode_raw),
};

/// The keys of a model that takes `select2`.
pub(crate) const METER_KEYS: MeterKeys = MeterKeys {
    context: std::slice::from_ref(&HZ_PERCENT),
    ..MeterKeys::NONE
};

/// Expected payload length for a UT61E+ measurement response.
const UT61EPLUS_MEASUREMENT_PAYLOAD_LEN: usize = 14;

/// Protocol implementation for the UT61E+/UT61B+/UT61D+/UT161 family.
pub(crate) struct Ut61PlusProtocol {
    table: &'static ModelTable,
    /// The model's manual spec tables.
    specs: SpecModel,
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// What the last reading said about where the dial sits, for
    /// [`Protocol::choices`] and [`Protocol::select`] for [`Setting::Mode`].
    dial: cycle::DialState,
    /// Readings arrive unasked: the link's adapter polls the meter for us
    /// (see [`Protocol::init`]), so a request is a read, not a write.
    streaming: bool,
    /// The secondary display last sent, held for the next main reading
    /// (family spec §2.3). That reading takes it, so a main frame with no
    /// secondary since the one before it carries none. Nothing blanks it
    /// after 500 ms as UNI-T's app does, and a command leaves it held. A
    /// replay records each reading's own frame, so it plays none back.
    secondary: Option<AuxValue>,
}

impl Default for Ut61PlusProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut61PlusProtocol {
    /// A UT61E+.
    pub(crate) fn new() -> Self {
        Self::for_model(FALLBACK.id)
    }

    /// The protocol for the family's registry entry `id`; an id outside the
    /// family gets the UT61E+'s, as a name no entry claims does.
    ///
    /// Several models share a table — the UT161x meters are believed to speak
    /// the same protocol as their UT61x+ counterparts — so the reported model
    /// and stability come from the entry, not from the table. Otherwise a
    /// UT161E would introduce itself as a verified UT61E+.
    pub(crate) fn for_model(id: &str) -> Self {
        // (table, the entry's registry id, stability, manual spec tables)
        let (table, id, stability, specs) = match id {
            // Verified by two captures reported in issue #7 (@frixit,
            // CP2110), both 2026-10-21: its frames carried the UT61E+'s
            // mode, range and flag bytes except after one AC+DC V range
            // press (family spec §2.2), and the second run passed the
            // gate and drove every command. What is left open on this
            // model is in `docs/research/ut61-family/verification.md`.
            "ut161e" => (
                &tables::ut61e_plus::UT61E_PLUS,
                "ut161e",
                Stability::Verified,
                SpecModel::Ut161e,
            ),
            // Verified by three captures reported in issue #19,
            // 2026-09-09 to 2026-09-11: every mode its dial reaches
            // decoded correctly, every command moved the flag it should,
            // the second run passed the gate outright and the third
            // walked the Ω and DC V ladders. What is left open on this
            // model is in `docs/research/ut61-family/verification.md`.
            "ut61b+" => (
                &tables::ut61b_plus::UT61B_PLUS,
                "ut61b+",
                Stability::Verified,
                SpecModel::Ut61bPlus,
            ),
            "ut161b" => (
                &tables::ut61b_plus::UT61B_PLUS,
                "ut161b",
                Stability::Experimental,
                SpecModel::Ut161b,
            ),
            "ut61d+" => (
                &tables::ut61d_plus::UT61D_PLUS,
                "ut61d+",
                Stability::Experimental,
                SpecModel::Ut61dPlus,
            ),
            "ut161d" => (
                &tables::ut61d_plus::UT61D_PLUS,
                "ut161d",
                Stability::Experimental,
                SpecModel::Ut161d,
            ),
            "ut60bt" => (
                &tables::ut60bt::UT60BT,
                "ut60bt",
                Stability::Experimental,
                SpecModel::Untranscribed,
            ),
            "ut202bt" => (
                &tables::ut202bt::UT202BT,
                "ut202bt",
                Stability::Experimental,
                SpecModel::Untranscribed,
            ),
            // "ut61eplus", and any id outside the family.
            _ => (
                &tables::ut61e_plus::UT61E_PLUS,
                FALLBACK.id,
                Stability::Verified,
                SpecModel::Ut61ePlus,
            ),
        };
        Self::with_profile(table, id, stability, specs)
    }

    fn with_profile(
        table: &'static ModelTable,
        id: &'static str,
        stability: Stability,
        specs: SpecModel,
    ) -> Self {
        // One sub-value each: a model's secondary display (family spec §2.3),
        // and the AC component of AC+DC V on a model whose dial reaches it
        // (see `parse_measurement`).
        let max_aux_values = usize::from(table.secondary_display)
            + usize::from(table.range_info(Mode::AcDcV, 0).is_some());
        // The family issue stays on every model but the UT61E+, verified or
        // not: the UT61B+ is decoded correctly everywhere it was looked at,
        // and its range rungs above the ones auto-ranging reached are still
        // open there. The UT60BT and UT202BT have issues of their own.
        let verification_issue = match id {
            "ut61eplus" => None,
            "ut60bt" => Some(26),
            "ut202bt" => Some(27),
            _ => Some(7),
        };
        let supported_commands = table.commands;
        Self {
            table,
            specs,
            rx_buf: Vec::with_capacity(64),
            dial: cycle::DialState::default(),
            streaming: false,
            secondary: None,
            profile: DeviceProfile {
                family_name: "UT61+/UT161",
                id,
                stability,
                supported_commands,
                max_aux_values,
                verification_issue,
                meter_keys: if table.commands.contains(&"select2") {
                    METER_KEYS
                } else {
                    MeterKeys::NONE
                },
            },
        }
    }

    /// Read a raw payload frame from the transport.
    fn read_raw_payload(&mut self, transport: &dyn Transport) -> Result<Vec<u8>> {
        framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "ut61eplus",
        )
    }

    /// Ask for one reading and read it.
    fn poll_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        debug!("sending measurement request");
        transport.write(&Command::GetMeasurement.encode())?;
        self.read_measurement(transport)
    }

    /// Read and parse a measurement response, skipping non-measurement frames.
    fn read_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        for _ in 0..5 {
            let payload = self.read_raw_payload(transport)?;
            if payload.len() >= UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
                match self.take_reading(&payload) {
                    Some(reading) => return reading,
                    None => continue,
                }
            }
            debug!(
                "skipping non-measurement frame ({} bytes): {:02X?}",
                payload.len(),
                payload
            );
            report_unknown_frame(self.profile.id, &payload);
        }
        Err(Error::Timeout)
    }

    /// A measurement-sized payload as a reading, or `None` for a
    /// secondary-display frame (family spec §2.3) on a model that has the
    /// display, which is held for the next main reading instead of being
    /// returned as one. On any other model [`parse_measurement`] refuses it.
    fn take_reading(&mut self, payload: &[u8]) -> Option<Result<Measurement>> {
        if self.table.secondary_display && is_secondary(payload) {
            // An unrecognised one clears the held value too: it is older
            // than what the meter now shows beside the next reading.
            self.secondary = parse_secondary(self.profile.id, payload, self.table);
            return None;
        }
        let secondary = self.secondary.take();
        Some(
            parse_measurement(self.profile.id, payload, self.table).map(|mut m| {
                m.aux_values.extend(secondary);
                m
            }),
        )
    }

    /// Write one command frame and wait for the meter's ack before returning.
    ///
    /// Whatever arrives up to and including the ack is discarded: leaving it
    /// in the buffer would make the next measurement read start mid-stream.
    /// Waiting matters as much as discarding — see [`PRESS_ACK_TIMEOUT`].
    fn press_command(&mut self, transport: &dyn Transport, cmd: Command) -> Result<()> {
        let encoded = cmd.encode();
        transport.write(&encoded)?;

        self.rx_buf.clear();
        // Real time, not the session clock: this waits on the meter itself.
        let sent = Instant::now();
        let wait = if self.streaming {
            BLUETOOTH_PRESS_ACK_TIMEOUT
        } else {
            PRESS_ACK_TIMEOUT
        };
        let deadline = sent + wait;
        // The bytes seen so far, trimmed to the tail an ack could still start
        // in: a CP2110 can hand over `AB CD 04` and `FF 00 02 7B` in separate
        // reads.
        let mut seen: Vec<u8> = Vec::with_capacity(64 + ACK_FRAME.len());
        let mut tmp = [0u8; 64];
        loop {
            let n = framing::read_uart_bytes(transport, &mut tmp, deadline)?;
            if n == 0 {
                warn!("no ack within {} ms of the command", wait.as_millis());
                break;
            }
            seen.extend_from_slice(&tmp[..n]);
            if seen.windows(ACK_FRAME.len()).any(|w| w == ACK_FRAME) {
                let waited = Instant::now()
                    .checked_duration_since(sent)
                    .unwrap_or_default();
                debug!("ack {} ms after the command", waited.as_millis());
                break;
            }
            let keep_from = seen.len().saturating_sub(ACK_FRAME.len() - 1);
            seen.drain(..keep_from);
        }

        Ok(())
    }

    fn command_from_name(name: &str) -> Result<Command> {
        match name {
            cmd::HOLD => Ok(Command::Hold),
            cmd::MINMAX => Ok(Command::MinMax),
            cmd::EXIT_MINMAX => Ok(Command::ExitMinMax),
            cmd::RANGE => Ok(Command::Range),
            cmd::AUTO => Ok(Command::Auto),
            cmd::REL => Ok(Command::Rel),
            "select2" => Ok(Command::Select2),
            cmd::SELECT => Ok(Command::Select),
            cmd::LIGHT => Ok(Command::Light),
            cmd::PEAK => Ok(Command::PeakMinMax),
            cmd::EXIT_PEAK => Ok(Command::ExitPeak),
            _ => Err(Unsupported::Command(name.to_string()).into()),
        }
    }
}

impl Protocol for Ut61PlusProtocol {
    fn delivery(&self) -> crate::protocol::Delivery {
        if self.streaming {
            crate::protocol::Delivery::Streamed
        } else {
            crate::protocol::Delivery::Polled
        }
    }

    fn discard_input(&mut self, transport: &dyn Transport) -> Result<()> {
        self.rx_buf.clear();
        // A secondary display held from before would be attached to the
        // first reading after, from another moment.
        self.secondary = None;
        crate::protocol::framing::discard_queued(transport)
    }

    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // A cable needs nothing: the CP2110 is set up by `Cp2110::open()`
        // before the protocol exists, and the meter only acknowledges 0x5D.
        // The UT-D07B polls the meter itself once told to, at about three
        // readings a second against under two when each one is a round trip
        // over the radio (adapter spec §3, §5).
        // A model that wants its name asked first has had it by now
        // (`name_before_init`).
        if transport.link() == Some(Link::Bluetooth) {
            debug!("starting the adapter's readings stream");
            transport.write(&Command::StartStream.encode())?;
            self.streaming = true;
        }
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let m = if self.streaming {
            match self.read_measurement(transport) {
                // A start command can be lost on the adapter, so a silent
                // stream is started again — and polled, because an adapter
                // that ignores 0x5D (none seen yet) still answers a poll.
                Err(Error::Timeout) => {
                    debug!("no streamed reading: restarting the stream and polling");
                    transport.write(&Command::StartStream.encode())?;
                    self.poll_measurement(transport)?
                }
                other => other?,
            }
        } else {
            self.poll_measurement(transport)?
        };
        // The stream is the only place the meter states its mode, so every
        // reading is what keeps the dial position current.
        self.dial.observe(self.table.dial, m.mode_raw);
        Ok(m)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(self.profile.id, payload, self.table)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        // A button this model does not have (family spec §6.5).
        if !self.profile.supported_commands.contains(&command) {
            return Err(Unsupported::Command(command.to_string()).into());
        }
        let cmd = Self::command_from_name(command)?;
        debug!("sending command: {command}");
        self.press_command(transport, cmd)
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        let cmd = Command::GetName.encode();
        debug!("sending get_name request");
        transport.write(&cmd)?;

        // The reply is two frames, ack then name (§2.3). The third read is
        // for a bridge that had a reading buffered before the request: only
        // a frame that fails to be the name costs one, so a meter that
        // answers straight away never waits for it. A streaming adapter can
        // have a few readings queued ahead of the reply, hence the wider
        // budget there.
        let budget = if self.streaming { 10 } else { 3 };
        for _ in 0..budget {
            let payload = self.read_raw_payload(transport)?;
            report_unknown_frame(self.profile.id, &payload);
            // Anything else is another frame on this wire, not a name: the
            // ack, or a reading the bridge held. Returning one as the name
            // would put it in the GUI header and `dmm-cli info`.
            if let Some(name) = name_from_reply(&payload) {
                debug!("device name: {name}");
                return Ok(Some(name));
            }
        }

        Ok(None)
    }

    fn entry_named(&self, name: &str) -> Option<&'static SelectableDevice> {
        device_for_reported_name(name)
    }

    /// Get Name, answered, ahead of 0x5D, as UNI-T's app sends it: a UT60BT
    /// is reported to ignore 0x5D until it has answered 0x5F (family spec
    /// §6.4). The wait is `get_name`'s, a few frames of at most 2 s each.
    fn name_before_init(&self, transport: &dyn Transport) -> bool {
        transport.link() == Some(Link::Bluetooth) && self.table.name_before_stream
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn spec_info(&self, m: &Measurement) -> Option<&'static crate::specs::SpecInfo> {
        self.specs.row(m).map(|row| &row.spec)
    }

    fn mode_spec_info(&self, m: &Measurement) -> Option<&'static crate::specs::ModeSpecInfo> {
        self.specs.table(m).map(|table| &table.mode)
    }

    fn spec_sheet(&self) -> Vec<crate::specs::SpecSheetTable> {
        self.specs.sheet()
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        cycle::choices(self, setting, current)
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        cycle::select(self, transport, setting, id)
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        capture::steps(self.table, &self.profile)
    }
}

/// The ack the meter answers every command with
/// (`docs/research/ut61eplus/reverse-engineered-protocol.md` §6). Something is
/// listening, but the ack says nothing about what — [`Protocol::get_name`]
/// reads past it, and detection ignores it.
pub(crate) fn is_ack(payload: &[u8]) -> bool {
    payload == [0xFF, 0x00]
}

/// The whole ack frame on the wire: [`is_ack`]'s payload with its header,
/// length and checksum.
const ACK_FRAME: [u8; 7] = framing::build_abcd_be16(0xFF, &[0x00]);

/// The model name in a Get Name reply, `None` for a payload that is not one.
///
/// The name is printable ASCII, e.g. `"UT61E+"` (§6). The length bounds are
/// what separate it from the other frames on this wire: a measurement payload
/// is 14 bytes of mixed binary and ASCII, and the ack is two.
pub(crate) fn name_from_reply(payload: &[u8]) -> Option<String> {
    if (3..=20).contains(&payload.len()) && payload.iter().all(u8::is_ascii_graphic) {
        return Some(String::from_utf8_lossy(payload).into_owned());
    }
    None
}

/// Report a frame that is none of the three this family sends: a measurement
/// (`docs/research/ut61eplus/reverse-engineered-protocol.md` §2.4), the
/// `FF 00` ack or a Get Name reply (§2.3). Our UT61E+ and UT61B+ captures hold
/// no other shape.
///
/// A measurement-sized frame is never reported here. [`Protocol::get_name`]
/// can read a stale one ahead of the reply on a CH9329, which does not purge
/// its RX buffer on open, and the reading path parses them instead.
fn report_unknown_frame(id: &'static str, payload: &[u8]) {
    if payload.len() != UT61EPLUS_MEASUREMENT_PAYLOAD_LEN
        && !is_ack(payload)
        && name_from_reply(payload).is_none()
    {
        report_unknown(
            id,
            "frame",
            format_args!("{} bytes: {:02X?}", payload.len(), payload),
        );
    }
}

/// Detection for the UT61+/UT161 family.
///
/// Get Name is the only reply on any bridge that pins the exact model, which
/// is why the cascade sends it first; a measurement frame settles the family
/// alone.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Ut61EPlus,
    label: "ut61+ get name",
    trigger: Some(send_get_name),
    send_after: &[],
    checksummed: true,
    claims_info_links: false,
    recognise,
};

/// Ask the meter its name — the same frame [`Protocol::get_name`] writes.
fn send_get_name(transport: &dyn Transport) -> Result<()> {
    transport.write(&Command::GetName.encode())
}

/// The BE16 frames this family sends: the name frame pins the model, a
/// measurement frame only the family.
///
/// The scan runs to the end of the buffer even once a reading has been seen:
/// on the CH9329, which does not purge its RX buffer on open, a stale frame
/// from an earlier session can sit in front of the name.
fn recognise(buf: &[u8], probing: &Probing) -> Option<Evidence> {
    let mut evidence = None;
    for start in framing::abcd_header_offsets(buf) {
        let Ok(Some((payload, _))) = framing::extract_frame_abcd_be16(&buf[start..]) else {
            continue;
        };
        if is_ack(&payload) {
            continue;
        }
        if let Some(name) = name_from_reply(&payload) {
            return Some(match device_for_reported_name(&name) {
                Some(device) => {
                    debug!("detect: name frame {name:?} resolves to {}", device.id);
                    Evidence::Model {
                        device,
                        reported_name: Some(name),
                    }
                }
                // A meter with Bluetooth built in named its model in the name
                // it advertises (docs/detection-design.md, Names and the
                // registry), and its ranges are not the UT61E+'s.
                None => match probing
                    .advertised
                    .filter(|d| d.family == DeviceFamily::Ut61EPlus)
                {
                    Some(device) => {
                        report_unknown(
                            device.id,
                            "model name",
                            format_args!(
                                "{name:?}, using the {} tables, the model its Bluetooth name \
                                 advertises",
                                device.display_name()
                            ),
                        );
                        Evidence::Model {
                            device,
                            reported_name: Some(name),
                        }
                    }
                    None => {
                        report_unknown(
                            FALLBACK.id,
                            "model name",
                            format_args!("{name:?}, using the UT61E+ tables"),
                        );
                        Evidence::Model {
                            device: FALLBACK,
                            reported_name: Some(name),
                        }
                    }
                },
            });
        }
        if payload.len() == UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
            evidence = Some(Evidence::FamilyOnly { fallback: FALLBACK });
        }
    }
    evidence
}

/// The entry a frame that names no model opens: the tables every meter in the
/// family reads with, even where a sibling's ranges differ.
const FALLBACK: &SelectableDevice = &devices::UT61EPLUS;

/// Resolve the ASCII model name a UT61+/UT161 meter answers Get Name
/// (`0x5F`) with — "UT61E+", "UT61B+" — to its registry entry.
///
/// Only [`DeviceFamily::Ut61EPlus`] entries are considered: the name frame is
/// that family's alone, so matching the whole registry would let a meter that
/// happens to report "mock" or "UT171" pick an entry whose framing it does not
/// speak. Matched case-insensitively against the model alone: the models are
/// exactly what the meters send, and the aliases are what users type.
fn device_for_reported_name(name: &str) -> Option<&'static SelectableDevice> {
    let wanted = name.trim();
    DEVICES
        .iter()
        .copied()
        .filter(|d| d.family == DeviceFamily::Ut61EPlus)
        .find(|d| d.model.eq_ignore_ascii_case(wanted))
}

/// How long a press waits for the meter's ack before the next command goes
/// out.
///
/// A poll sent ahead of the ack can go unanswered: a UT61B+ over CH9329 never
/// answered one that went out 14 ms before a late ack, and the switch it was
/// confirming failed with a timeout (issue #20). Waiting costs a healthy meter
/// nothing, since a poll sent early is only answered after the ack anyway. The
/// slowest acks on record are 415 ms on a UT61E+ (HOLD over a live DC V
/// reading) and 217 ms on a UT61B+ (in Hz with the leads open). Every press on
/// record was acked, so the cap only matters if one goes missing.
const PRESS_ACK_TIMEOUT: Duration = Duration::from_millis(1000);
/// The same wait over the UT-D07B, whose ack came 1.26 s after the command on
/// a fresh link (adapter spec §3).
const BLUETOOTH_PRESS_ACK_TIMEOUT: Duration = Duration::from_millis(2500);

/// How long to leave the meter alone after it acks a button press before
/// asking it what mode it is in.
///
/// The meter is polled, so a press can land while a frame is already on its
/// way and the reading after it still shows the old mode. On a UT61E+ the
/// first read ~200 ms after the press (this delay after a 50 ms drain, before
/// presses waited for the ack) showed the new mode in every leg but one, where
/// a second read did. Counted from the ack instead, which that meter sends
/// 39–415 ms after the press, the first read showed the new mode, range or
/// flag after every press of a mode, range and flag walk (2026-09-14).
const SELECT_SETTLE_DELAY: Duration = Duration::from_millis(150);
/// Readings taken after a press before concluding it changed nothing.
///
/// RANGE presses reuse both constants: nobody has timed 0x46 separately, and
/// the meter answers a press the same way whichever button sent it.
const SELECT_SETTLE_READS: usize = 3;

/// Modes where a HOLD press (0x4A) leaves the flag where it was.
///
/// `[VERIFIED]` on a UT61E+ (`ut61eplus-verify4.yaml`, step `ncv/hold:on`) and a
/// UT61B+ (issue #19, same step): "HOLD did nothing in off". Everywhere else
/// HOLD took, continuity, diode, capacitance, Hz, duty and AC+DC V included,
/// and diode's frames carry the HOLD flag over an OL reading — so OL is no
/// bar to HOLD, only to REL.
const HOLD_DEAD: &[Mode] = &[Mode::Ncv];

/// Modes where a REL press (0x48) leaves the flag where it was.
///
/// The bar is a refusal reproduced with a real reading on screen, on every
/// meter that can be asked. The reading matters: this meter also refuses REL
/// whenever the display shows OL, whatever the mode — `dcmv/rel:on` was
/// refused over OL in one run and taken in the three where DC mV had a value
/// — so a refusal only ever seen over OL says nothing about the mode.
///
/// AC+DC V is here on three refusals from our UT61E+ (2026-09-07 twice at
/// 0.07 V and 0.08 V, 2026-09-10 again), which is every meter that has the
/// mode — the UT61B+ has no such position. HOLD and MIN/MAX both work there,
/// so it is REL specifically, not a mode that ignores commands.
///
/// Diode is here on both meters that have the mode, each asked with a diode
/// fitted rather than over OL: our UT61E+ with a Schottky forward-biased at
/// 0.1968 V (2026-09-10) and a UT61B+ at 0.515 V (issue #19, 2026-09-11).
const REL_DEAD: &[Mode] = &[
    Mode::Continuity,
    Mode::Diode,
    Mode::Hz,
    Mode::DutyCycle,
    Mode::Ncv,
    Mode::AcDcV,
];

/// Modes where a MIN/MAX press (0x41) leaves the flags where they were.
///
/// Unlike REL, MIN/MAX is not confounded by an OL reading: `dcmv/minmax` was
/// taken twice over OL, and continuity and diode give the same answer over OL
/// as over a value. So the UT61B+'s five diode refusals count, and our UT61E+
/// added a sixth over a Schottky at 0.1968 V (2026-09-10) — two meters, and
/// diode is here.
///
/// Capacitance is here and takes REL: the two buttons do not go together.
/// AC+DC V is not, and there the meter said so outright — `acdcv/minmax:min`
/// read 0.0652 V back with the MIN flag set.
const MINMAX_DEAD: &[Mode] = &[
    Mode::Continuity,
    Mode::Diode,
    Mode::Capacitance,
    Mode::Hz,
    Mode::DutyCycle,
    Mode::Ncv,
];

impl cycle::CycleMeter for Ut61PlusProtocol {
    fn dial_positions(&self) -> &'static [cycle::DialPosition] {
        self.table.dial
    }

    fn dial_state(&self) -> &cycle::DialState {
        &self.dial
    }

    fn dial_state_mut(&mut self) -> &mut cycle::DialState {
        &mut self.dial
    }

    fn press(&mut self, transport: &dyn Transport, button: cycle::CycleButton) -> Result<()> {
        let cmd = match button {
            cycle::CycleButton::Select => Command::Select,
            cycle::CycleButton::Hz => Command::Select2,
            cycle::CycleButton::Range => Command::Range,
            cycle::CycleButton::Hold => Command::Hold,
            cycle::CycleButton::Rel => Command::Rel,
            cycle::CycleButton::MinMax => Command::MinMax,
            cycle::CycleButton::Peak => Command::PeakMinMax,
        };
        self.press_command(transport, cmd)
    }

    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        Protocol::request_measurement(self, transport)
    }

    fn mode_label(&self, mode: u16) -> Cow<'static, str> {
        match u8::try_from(mode) {
            Ok(byte) => match Mode::from_byte(byte) {
                Some(m) => Cow::Borrowed(m.as_static_str()),
                None => unknown_mode(byte),
            },
            // This family's mode field is one byte wide; anything wider than
            // that never came from a meter.
            Err(_) => unknown_mode16(mode),
        }
    }

    fn settle(&self) -> cycle::Settle {
        cycle::Settle {
            delay: SELECT_SETTLE_DELAY,
            reads: SELECT_SETTLE_READS,
        }
    }

    /// The model's own range table for the mode, in range-byte order — the
    /// same table `range_label` reads, so a rung is named exactly as the
    /// reading that lands on it will be.
    fn range_ladder(&self, mode: u16) -> Vec<Cow<'static, str>> {
        // The ladder comes with an Auto rung, which AUTO sets: a model that
        // does not take it has no way back from a manual rung (family spec
        // §6.5), so it is offered none.
        if !self.profile.supported_commands.contains(&cmd::AUTO) {
            return Vec::new();
        }
        match u8::try_from(mode).ok().and_then(Mode::from_byte) {
            Some(mode) => tables::range_ladder(self.table, mode),
            // A mode byte this family's parser cannot name has no table.
            None => Vec::new(),
        }
    }

    fn set_auto_range(&mut self, transport: &dyn Transport) -> Result<()> {
        self.press_command(transport, Command::Auto)
    }

    /// Which states each flag command can be driven to in `mode`.
    ///
    /// We used to answer "every state in every mode", on the reading that the
    /// family's command matrix (research spec §6) lists no mode restriction on
    /// 0x4A, 0x48 or 0x41. Two meters have since contradicted that: a UT61E+
    /// (CP2110, 2026-09-07) and a UT61B+ (CH9329, 2026-09-10, issue #19)
    /// refused the same commands in the same modes, each press leaving the
    /// flag where it was. [`REL_DEAD`], [`MINMAX_DEAD`] and [`HOLD_DEAD`] are
    /// that list. Peak depends on the model as well, so it stays with the
    /// table.
    ///
    /// The lists narrow `get`/`set` and the capture sweep alone: the GUI's
    /// buttons and `dmm-cli command` press through `send_command`, so a wrong
    /// entry cannot stop a press, only `set` reaching a state the meter has.
    fn flag_states(&self, setting: cycle::FlagSetting, mode: u16) -> &'static [u16] {
        // A flag whose button the model does not have (family spec §6.5).
        let command = match setting {
            cycle::FlagSetting::Hold => cmd::HOLD,
            cycle::FlagSetting::Rel => cmd::REL,
            cycle::FlagSetting::MinMax => cmd::MINMAX,
            cycle::FlagSetting::Peak => cmd::PEAK,
        };
        if !self.profile.supported_commands.contains(&command) {
            return &[];
        }
        let named = u8::try_from(mode).ok().and_then(Mode::from_byte);
        let dead = |modes: &[Mode]| named.is_some_and(|m| modes.contains(&m));
        match setting {
            cycle::FlagSetting::Hold if dead(HOLD_DEAD) => &[],
            cycle::FlagSetting::Rel if dead(REL_DEAD) => &[],
            cycle::FlagSetting::MinMax if dead(MINMAX_DEAD) => &[],
            cycle::FlagSetting::Hold | cycle::FlagSetting::Rel => &[0, 1],
            // MAX then MIN, the order the meter's own 2-state ring cycles in.
            // No AVG: the UT61E+ reports none over USB.
            cycle::FlagSetting::MinMax => &[0, 1, 2],
            cycle::FlagSetting::Peak => match named {
                Some(mode) if self.table.peak_modes.contains(&mode) => &[0, 1, 2],
                _ => &[],
            },
        }
    }

    fn exit_flag(&mut self, transport: &dyn Transport, setting: cycle::FlagSetting) -> Result<()> {
        match setting {
            cycle::FlagSetting::MinMax => self.press_command(transport, Command::ExitMinMax),
            cycle::FlagSetting::Peak => self.press_command(transport, Command::ExitPeak),
            // HOLD and REL press their own button back off, so the driver
            // never asks this of them.
            other => Err(unsupported_setting(other.setting())),
        }
    }
}

/// What the NCV display shows while no field is detected.
const NCV_IDLE: &str = "EF";

/// The NCV level on the display, `None` for text that is none of its forms.
///
/// The level is drawn as "-" segments, not a digit: "EF" while no field is
/// detected, one more "-" per level as it grows (manual §13; "   EF  " and
/// "     - " observed on 2026-09-07). Dashes mixed with anything else, or a
/// number, are none of its forms.
fn ncv_level(display_compact: &str) -> Option<u8> {
    if display_compact == NCV_IDLE {
        Some(0)
    } else if !display_compact.is_empty() && display_compact.chars().all(|c| c == '-') {
        // A 7-character field: at most 7 dashes.
        u8::try_from(display_compact.len()).ok()
    } else {
        None
    }
}

/// Decode the three UT61+ flag bytes (already masked with `& 0x0F`). Bit map
/// from §2.7, which marks what was verified on our UT61E+:
///
/// - byte 11 (`flag1`): bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX
/// - byte 12 (`flag2`): bit0=HV warning, bit1=Low Battery, bit2=!AUTO (inverted),
///   bit3=APO (§2.7; not carried)
/// - byte 13 (`flag3`): bit0=bar polarity, bit1=Peak MIN, bit2=Peak MAX,
///   bit3=AC (clear = DC)
///
/// `dc` marks the DC component of an AC+DC reading, which alternates frames
/// between its two components: flag3 bit 3 is set on the AC one (§2.7).
/// Elsewhere the mode says AC or DC, and `dc` stays clear. `ac_dc` is whether
/// the mode is an AC+DC one.
fn parse_flags(ac_dc: bool, flag1: u8, flag2: u8, flag3: u8) -> StatusFlags {
    StatusFlags {
        rel: flag1 & 0x01 != 0,
        hold: flag1 & 0x02 != 0,
        min: flag1 & 0x04 != 0,
        max: flag1 & 0x08 != 0,
        hv_warning: flag2 & 0x01 != 0,
        low_battery: flag2 & 0x02 != 0,
        // Inverted: bit2 of flag2 is the MANUAL range indicator.
        // When clear (0), the meter is in auto-range mode.
        auto_range: flag2 & 0x04 == 0,
        dc: ac_dc && flag3 & 0x08 == 0,
        peak_max: flag3 & 0x04 != 0,
        peak_min: flag3 & 0x02 != 0,
        ..Default::default()
    }
}

/// Report what a measurement payload carries outside the spec and our UT61E+
/// and UT61B+ captures. The reading is parsed the same either way. `mode` is
/// `None` for a mode byte no table lists, already reported.
///
/// Sections are `docs/research/ut61eplus/reverse-engineered-protocol.md`;
/// "family" is `docs/research/ut61-family/reverse-engineered-protocol.md`.
fn report_unrecognised_fields(
    id: &'static str,
    payload: &[u8],
    mode: Option<Mode>,
    has_range: bool,
    table: &ModelTable,
) {
    if let Some(mode) = mode {
        let mode_byte = payload[0];
        // Family §3.1: the model's dial lists every mode it reaches, and every
        // mode the captures show is on it. A decodable byte off it is another
        // model's mode or one of §2.5's speculative ones. An empty dial is a
        // table that does not describe one.
        let dial = table.dial;
        if !dial.is_empty() && !dial.iter().any(|p| p.contains(u16::from(mode_byte))) {
            report_unknown(
                id,
                "mode byte",
                format_args!("{mode_byte:#04x} ({mode}), not on the {} dial", table.name),
            );
        } else if !has_range && mode != Mode::Ncv {
            // Family §5 and §6.2: NCV is the one mode without a range table;
            // every other has at least one rung, and the captures stay inside
            // them. A mode off the dial was reported above.
            report_unknown(
                id,
                "range byte",
                format_args!("mode {mode_byte:#04x} range {}", payload[1] & 0x0F),
            );
        }
    }
    let flags = &payload[11..14];
    // Family §4: a model without Peak never sets P-MIN or P-MAX (flag3
    // bits 1-2).
    if payload[13] & 0x06 != 0 && table.peak_modes.is_empty() {
        report_unknown(
            id,
            "flag bits",
            format_args!("{flags:02X?}, Peak on a model without it"),
        );
    }
    for i in PREFIXED_BYTES {
        if payload[i] & 0xF0 != 0x30 {
            report_unknown(
                id,
                "byte prefix",
                format_args!("payload[{i}] = {:#04x}", payload[i]),
            );
        }
    }
    // §2.6: two decimal digits counting lit bar segments, of 46 at most
    // (family §2.1). The captures top out at 44.
    let (tens, ones) = (payload[9], payload[10]);
    if ones > 9 || u16::from(tens) * 10 + u16::from(ones) > 46 {
        report_unknown(id, "bar graph", format_args!("{:02X?}", &payload[9..11]));
    }
}

/// Report a mode byte §2.5 does not list.
///
/// One with bit 7 set over a function we decode is most likely a
/// secondary-display frame (family spec §2.3) from a model not known to
/// send one, so the report says so and carries the whole frame: it is what
/// tells us another meter has a second display.
fn report_unknown_mode(id: &'static str, payload: &[u8]) {
    let mode_byte = payload[0];
    match Mode::from_byte(mode_byte & !SECONDARY) {
        Some(function) if mode_byte & SECONDARY != 0 => report_unknown(
            id,
            "mode byte",
            format_args!(
                "{mode_byte:#04x}, which looks like a secondary-display frame for {function} \
                 (family spec §2.3): {:02X?}",
                &payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN]
            ),
        ),
        _ => report_unknown(id, "mode byte", format_args!("{mode_byte:#04x}")),
    }
}

/// §2.6 (range byte) and §2.7 (flag bytes): these have a 0x30 high nibble.
const PREFIXED_BYTES: [usize; 4] = [1, 11, 12, 13];

/// Whether a payload has a measurement's length and prefixes (§2.4), which
/// is all a frame with an unlisted mode byte has to say it is a reading.
fn shaped_as_a_reading(payload: &[u8]) -> bool {
    payload.len() == UT61EPLUS_MEASUREMENT_PAYLOAD_LEN
        && PREFIXED_BYTES.iter().all(|&i| payload[i] & 0xF0 == 0x30)
}

/// Bit 7 of the mode byte marks a secondary-display frame (family spec §2.3).
const SECONDARY: u8 = 0x80;

/// Whether a measurement-sized payload is a secondary-display frame.
fn is_secondary(payload: &[u8]) -> bool {
    payload[0] & SECONDARY != 0
}

/// The sub-value a secondary-display frame carries, or `None` for a function
/// or range the model's table lacks, reported once.
///
/// The frame is laid out as a main one (family spec §2.3): the function in
/// the mode byte's low seven bits, then the range byte and the 7-char
/// display. Its bar graph and flag bytes are not read: UNI-T's app ignores
/// them, and what they hold is unknown. The unit comes from the model's own
/// table for that function, as a main reading's does.
fn parse_secondary(id: &'static str, payload: &[u8], table: &ModelTable) -> Option<AuxValue> {
    let frame = &payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN];
    let range_byte = payload[1] & 0x0F;
    let Some((mode, range)) = Mode::from_byte(payload[0] & !SECONDARY)
        .and_then(|mode| Some((mode, table.range_info(mode, range_byte)?)))
    else {
        report_unknown(id, "secondary display", format_args!("{frame:02X?}"));
        return None;
    };
    let display_raw = String::from_utf8_lossy(&payload[2..9]).to_string();
    Some(AuxValue {
        label: Cow::Borrowed(secondary_label(mode)),
        value: display_value(id, Some(mode), &display_raw),
        unit: Cow::Borrowed(range.unit),
        display_raw: Some(display_raw),
        elapsed_secs: None,
    })
}

/// The sub-value label of a secondary display showing `mode`.
fn secondary_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Hz => "Frequency",
        Mode::TempC | Mode::TempF => "Temperature",
        other => other.as_static_str(),
    }
}

/// Every label the family's parser and choice lists produce, by model: each
/// mode, the range labels of the model's table, the sub-value labels its
/// secondary display and AC+DC V take, and [`cycle::all_choice_labels`].
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn all_labels(out: &mut crate::text::Labels) {
    use crate::text::TextKind;
    for device in DEVICES
        .iter()
        .filter(|d| d.family == DeviceFamily::Ut61EPlus)
    {
        let protocol = Ut61PlusProtocol::for_model(device.id);
        let by = protocol.profile.id;
        let table = protocol.table;
        for &mode in Mode::ALL {
            out.add(TextKind::Mode, mode.as_static_str(), by);
            // The parser masks the range byte to its low nibble.
            for range in 0..=0x0F {
                let Some(info) = table.range_info(mode, range) else {
                    continue;
                };
                out.add(TextKind::Range, info.label, by);
                if table.secondary_display {
                    out.add(TextKind::Aux, secondary_label(mode), by);
                }
                if mode == Mode::AcDcV {
                    out.add(TextKind::Aux, AC_COMPONENT_LABEL, by);
                }
            }
        }
        let modes = Mode::ALL.iter().map(|&mode| u16::from(mode as u8));
        cycle::all_choice_labels(&protocol, modes, by, out);
    }
}

/// Parse a UT61E+/UT61B+/UT61D+/UT161 measurement payload (pure function).
///
/// Layout (verified against real device captures):
/// - byte 0:    mode   (raw, no masking — does not have 0x30 prefix)
/// - byte 1:    range  (& 0x0F — has 0x30 prefix)
/// - bytes 2-8: display value (7 ASCII chars, no masking needed)
/// - byte 9:    bar graph tens digit (raw, no 0x30 prefix; value = b9*10+b10)
/// - byte 10:   bar graph ones digit (raw, no 0x30 prefix)
/// - byte 11:   flag1  (& 0x0F — has 0x30 prefix)
/// - byte 12:   flag2  (& 0x0F — has 0x30 prefix)
/// - byte 13:   flag3  (& 0x0F — has 0x30 prefix)
///
/// `id` is the registry entry being read, which reports name: a sibling's
/// table can't tell it, as the UT61D+ and UT161D share one.
pub(crate) fn parse_measurement(
    id: &'static str,
    payload: &[u8],
    table: &ModelTable,
) -> Result<Measurement> {
    check_len("ut61eplus", payload, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN)?;
    // §2.4: the measurement payload is 14 bytes; the rest is ignored.
    if payload.len() > UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
        report_unknown(
            id,
            "frame",
            format_args!("{} bytes: {:02X?}", payload.len(), payload),
        );
    }

    // Mode byte is raw (no 0x30 prefix), range byte has 0x30 prefix
    let mode_byte = payload[0];
    let range_byte = payload[1] & 0x0F;
    let display_bytes = &payload[2..9];
    // Bar graph bytes are raw (no 0x30 prefix observed on real device).
    // Encoding is decimal (byte9 * 10 + byte10), NOT nibble shift.
    // Verified on real device: 5V→9, 10V→20, 20V→39 on 22V range;
    // 1V→20 on 2.2V range. Maps to ~46 LCD bar segments.
    let bar_hi = payload[9] as u16;
    let bar_lo = payload[10] as u16;
    let flag1 = payload[11] & 0x0F;
    let flag2 = payload[12] & 0x0F;
    let flag3 = payload[13] & 0x0F;

    // §2.5 lists no mode byte past 0x1E. Other bytes still read, unnamed and
    // with no unit, in a frame shaped as a reading. Not one with bit 7, which
    // marks a secondary display (family spec §2.3): those come between main
    // readings, and as readings would change the mode every other frame. Nor
    // any other long frame, such as a Get Name reply (§2.3).
    let mode = Mode::from_byte(mode_byte);
    if mode.is_none() {
        report_unknown_mode(id, payload);
        if is_secondary(payload) || !shaped_as_a_reading(payload) {
            return Err(Error::invalid_response(
                format!("{id} frame with mode byte {mode_byte:#04x} is no main reading"),
                &payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN],
            ));
        }
    }
    let display_raw = String::from_utf8_lossy(display_bytes).to_string();
    let progress = bar_hi * 10 + bar_lo;
    let ac_dc = matches!(mode, Some(Mode::AcDcV | Mode::AcDcA | Mode::ClampAcDcA));
    let flags = parse_flags(ac_dc, flag1, flag2, flag3);

    // Look up range info from device table
    let range_info = mode.and_then(|mode| table.range_info(mode, range_byte));
    report_unrecognised_fields(id, payload, mode, range_info.is_some(), table);
    let unit = range_info.map(|r| r.unit).unwrap_or("");
    let range_label = range_info.map(|r| r.label).unwrap_or("");

    let value = display_value(id, mode, &display_raw);
    let m = Measurement {
        mode: mode.map_or_else(
            || unknown_mode(mode_byte),
            |mode| Cow::Borrowed(mode.as_static_str()),
        ),
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: Some(progress),
        display_raw: Some(display_raw),
        flags,
        ..Measurement::from_payload(&payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN], value)
    };

    // §2.7: in AC+DC V the meter sends its DC and AC components in frames of
    // their own, in turn, flag3 bit 3 set on the AC one. The DC frame is the
    // reading, named for its component; the AC frame carries its component
    // as a sub-value in the reading's unit and no main reading, so each keeps
    // its own time and frame. Only where the model's dial reaches the mode:
    // the deck's AC+DC current modes have no ranges, and nothing has shown
    // how they send.
    if mode != Some(Mode::AcDcV) || range_info.is_none() {
        return Ok(m);
    }
    let m = Measurement {
        main_label: Some(MainLabel::Dc),
        ..m
    };
    if !m.flags.dc {
        let ac = AuxValue {
            label: Cow::Borrowed(AC_COMPONENT_LABEL),
            value: m.value,
            unit: Cow::Borrowed(""),
            display_raw: m.display_raw,
            elapsed_secs: None,
        };
        return Ok(Measurement {
            value: MeasuredValue::Absent,
            display_raw: None,
            aux_values: vec![ac],
            ..m
        });
    }
    Ok(m)
}

/// Label of the AC component an AC+DC V frame carries, beside the reading
/// named [`MainLabel::Dc`] (see `parse_measurement`).
const AC_COMPONENT_LABEL: &str = "AC";

/// The value a 7-char display field shows in `mode`, the same for a main
/// and a secondary display (UT61E+ spec §2.4). `None` for a mode no table
/// lists.
fn display_value(id: &'static str, mode: Option<Mode>, display_raw: &str) -> MeasuredValue {
    let display_compact: String = display_raw.trim().chars().filter(|c| *c != ' ').collect();
    if mode == Some(Mode::Ncv) {
        match ncv_level(&display_compact) {
            Some(level) => MeasuredValue::NcvLevel(level),
            None => {
                report_unknown(
                    id,
                    "NCV display text",
                    format_args!("{display_compact:?}, shown as no reading"),
                );
                MeasuredValue::UNRECOGNISED
            }
        }
    } else if spells_ol(&display_compact) {
        MeasuredValue::Overload
    } else {
        display_number(id, &display_compact)
    }
}

/// Build a 14-byte UT61E+ protocol payload from parts (for tests).
///
/// This is a convenience helper for tests that need a realistic `Measurement`
/// produced by the protocol parser rather than a hand-constructed struct.
///
/// Parameters mirror the raw protocol layout:
/// - `mode`: mode byte (e.g. 0x02 = DC V)
/// - `range`: range nibble (0x30 prefix added automatically)
/// - `display`: 7-byte ASCII display value (e.g. `b"  5.678"`)
/// - `progress`: (tens, ones) bar graph digits — decoded as tens*10+ones
/// - `flags`: (flag1, flag2, flag3) nibbles (0x30 prefix added automatically)
#[cfg(any(test, feature = "test-support"))]
fn make_payload(
    mode: u8,
    range: u8,
    display: &[u8; 7],
    progress: (u8, u8),
    flags: (u8, u8, u8),
) -> Vec<u8> {
    vec![
        mode,
        range | 0x30,
        display[0],
        display[1],
        display[2],
        display[3],
        display[4],
        display[5],
        display[6],
        progress.0,
        progress.1,
        flags.0 | 0x30,
        flags.1 | 0x30,
        flags.2 | 0x30,
    ]
}

#[cfg(any(test, feature = "test-support"))]
#[expect(
    clippy::unwrap_used,
    reason = "a test fixture: a payload that fails to parse fails the test"
)]
pub fn make_test_measurement(
    mode: u8,
    range: u8,
    display: &[u8; 7],
    progress: (u8, u8),
    flags: (u8, u8, u8),
) -> Measurement {
    let payload = make_payload(mode, range, display, progress, flags);
    parse_measurement(FALLBACK.id, &payload, &tables::ut61e_plus::UT61E_PLUS).unwrap()
}

/// Every unit a reading of this family can carry, for
/// [`crate::protocol::all_units`].
#[cfg(test)]
pub(crate) fn all_units(out: &mut crate::protocol::Units) {
    out.insert("");
    for device in DEVICES
        .iter()
        .filter(|d| d.family == DeviceFamily::Ut61EPlus)
    {
        let table = Ut61PlusProtocol::for_model(device.id).table;
        for &mode in Mode::ALL {
            for range in 0..=0x0F {
                if let Some(info) = table.range_info(mode, range) {
                    out.insert(info.unit);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::registry;
    use crate::protocol::test_support::snapshot;

    /// [`super::parse_measurement`] as the UT61E+ entry reads: what these
    /// tests check is the decoding, whichever model's table they pass.
    fn parse_measurement(payload: &[u8], table: &ModelTable) -> Result<Measurement> {
        super::parse_measurement(FALLBACK.id, payload, table)
    }

    #[test]
    fn parse_no_flags_auto_on() {
        // All zero → AUTO is on (inverted logic), everything else off
        let flags = parse_flags(false, 0x00, 0x00, 0x00);
        assert!(!flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range); // inverted: bit clear = auto ON
        assert!(!flags.min);
        assert!(!flags.max);
        assert!(!flags.low_battery);
    }

    #[test]
    fn parse_hold_with_auto() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on)
        let flags = parse_flags(false, 0x02, 0x00, 0x00);
        assert!(flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range);
    }

    #[test]
    fn parse_manual_range() {
        // flag2=0x04 → AUTO bit set → auto_range OFF
        let flags = parse_flags(false, 0x00, 0x04, 0x00);
        assert!(!flags.auto_range);
    }

    #[test]
    fn parse_low_battery() {
        // flag2=0x02 → LOW BAT
        let flags = parse_flags(false, 0x00, 0x02, 0x00);
        assert!(flags.low_battery);
        assert!(flags.auto_range); // AUTO still on (bit2 is clear)
    }

    #[test]
    fn parse_min_max() {
        // flag1: bit2=MIN, bit3=MAX
        let flags = parse_flags(false, 0x0C, 0x00, 0x00);
        assert!(flags.min);
        assert!(flags.max);
    }

    #[test]
    fn parse_all_flag1() {
        // flag1=0x0F: REL + HOLD + MIN + MAX
        let flags = parse_flags(false, 0x0F, 0x00, 0x00);
        assert!(flags.rel);
        assert!(flags.hold);
        assert!(flags.min);
        assert!(flags.max);
    }

    /// Our UT61E+ in AC+DC V across a 1.6 V cell (2026-09-19): the frames
    /// reading 1.61 V carry flag3 bit 3 clear, the 0.0000 V ones set.
    #[test]
    fn parse_dc_flag() {
        assert!(parse_flags(true, 0x00, 0x00, 0x01).dc);
        assert!(!parse_flags(true, 0x00, 0x00, 0x09).dc);
        assert!(!parse_flags(false, 0x00, 0x00, 0x00).dc);
    }

    #[test]
    fn parse_real_device_hold() {
        // Real capture: meter on DC V with HOLD active
        // flag bytes (masked): 0x02, 0x00, 0x01
        let flags = parse_flags(false, 0x02, 0x00, 0x01);
        assert!(flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range);
        assert!(!flags.low_battery);
    }

    /// Each model is asked only for the modes its dial reaches: the E+ has
    /// no temperature or LoZ position, the D+ has both.
    #[test]
    fn capture_steps_follow_the_model_s_dial() {
        let ids = |model: &str| -> Vec<&'static str> {
            Ut61PlusProtocol::for_model(model)
                .capture_steps()
                .iter()
                .map(|s| s.id)
                .collect()
        };
        let e_plus = ids("ut61eplus");
        assert!(e_plus.contains(&"acua"));
        for id in ["temp", "tempf", "loz", "loz_hz", "loz_duty"] {
            assert!(!e_plus.contains(&id), "{id} asked for on the E+");
        }
        let d_plus = ids("ut61d+");
        for id in ["temp", "tempf", "loz", "loz_hz", "loz_duty"] {
            assert!(d_plus.contains(&id), "{id} missing on the D+");
        }
        // No dial described: the table's own mode list decides.
        let ut60bt = ids("ut60bt");
        for id in [
            "acv", "dcmv", "duty", "ncv", "dcua", "acma", "temp", "tempf",
        ] {
            assert!(ut60bt.contains(&id), "{id} missing on the UT60BT");
        }
        // Nor the commands its buttons do not take: MAX/MIN (spec §6.5).
        for id in [
            "acdcv",
            "lpfv",
            "hfe",
            "dca",
            "aca",
            "loz",
            "minmax",
            "minmax_off",
        ] {
            assert!(!ut60bt.contains(&id), "{id} asked for on the UT60BT");
        }
        for id in ["hold", "rel", "range", "auto"] {
            assert!(ut60bt.contains(&id), "{id} missing on the UT60BT");
        }
    }

    /// The UT60BT's dial words three steps its own way; the E+ keeps the
    /// family text.
    #[test]
    fn ut60bt_steps_follow_its_dial() {
        let text = |model: &str, id: &str| -> &'static str {
            Ut61PlusProtocol::for_model(model)
                .capture_steps()
                .iter()
                .find(|s| s.id == id)
                .unwrap_or_else(|| panic!("{id} missing on {model}"))
                .instruction
        };
        assert_eq!(
            text("ut60bt", "acv"),
            "Set meter to V and press SELECT for AC V. Leave leads open."
        );
        assert_eq!(
            text("ut60bt", "ohm"),
            "Set meter to the \u{03A9} position and press SELECT for \u{03A9}. \
             Leave leads open (should show OL)."
        );
        assert_eq!(
            text("ut60bt", "duty"),
            "Hz/% position: press SELECT for Duty %."
        );
        assert_eq!(
            text("ut61eplus", "acv"),
            "Set meter to AC V (V~). Leave leads open."
        );
        assert_eq!(
            text("ut61eplus", "ohm"),
            "Set meter to \u{03A9}. Leave leads open (should show OL)."
        );
        assert_eq!(
            text("ut61eplus", "duty"),
            "Hz/% position: short-press the USB button for Duty %."
        );
    }

    /// The UT202BT has no dial and takes HOLD and RANGE alone of our
    /// commands, and every mode its steps assert is one its table has.
    #[test]
    fn ut202bt_steps_follow_its_buttons() {
        let proto = Ut61PlusProtocol::for_model("ut202bt");
        let steps = proto.capture_steps();
        let ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
        for id in [
            "rel",
            "rel_off",
            "minmax",
            "minmax_off",
            "auto",
            "diode",
            "dcmv",
        ] {
            assert!(!ids.contains(&id), "{id} asked for on the UT202BT");
        }
        for id in ["dcv", "ohm", "hold", "range", "lpfv", "inrush", "ncv"] {
            assert!(ids.contains(&id), "{id} missing on the UT202BT");
        }
        let modes = tables::ut202bt::UT202BT.modes;
        for label in steps.iter().filter_map(|s| s.expect.and_then(|e| e.mode)) {
            assert!(
                modes.iter().any(|m| m.as_static_str() == label),
                "{label} is not a UT202BT mode"
            );
        }
    }

    /// The modes the Hz/% key is offered in are the Hz/% rings of every dial
    /// described here.
    #[test]
    fn the_hz_percent_ring_is_every_dials() {
        use crate::protocol::cycle::CycleButton;
        for table in [
            &tables::ut61e_plus::UT61E_PLUS,
            &tables::ut61b_plus::UT61B_PLUS,
            &tables::ut61d_plus::UT61D_PLUS,
        ] {
            let mut ring: Vec<u16> = table
                .dial
                .iter()
                .flat_map(|position| position.rings)
                .filter(|ring| ring.button == CycleButton::Hz)
                .flat_map(|ring| ring.modes.iter().copied())
                .collect();
            ring.sort_unstable();
            ring.dedup();
            let mut expected = HZ_PERCENT_RING;
            expected.sort_unstable();
            assert_eq!(ring, expected, "{}", table.name);
        }
    }

    /// Each model offers the commands its buttons take: the whole family
    /// list on the models that have always had it, less Peak on the UT61B+ and
    /// UT161B, and on the UT60BT and UT202BT what UNI-T's app sends them
    /// (family spec §6.5). A capture step
    /// never sends one the model does not offer.
    #[test]
    fn each_model_offers_the_commands_its_buttons_take() {
        let commands = |model| {
            Ut61PlusProtocol::for_model(model)
                .profile
                .supported_commands
        };
        for model in ["ut61eplus", "ut161e", "ut61d+", "ut161d"] {
            assert_eq!(commands(model), UT61EPLUS_COMMANDS, "{model}");
        }
        for model in ["ut61b+", "ut161b"] {
            assert_eq!(commands(model), UT61BPLUS_COMMANDS, "{model}");
        }
        let without_peak: Vec<&str> = UT61EPLUS_COMMANDS
            .iter()
            .copied()
            .filter(|c| ![cmd::PEAK, cmd::EXIT_PEAK].contains(c))
            .collect();
        assert_eq!(UT61BPLUS_COMMANDS, without_peak);
        assert_eq!(
            commands("ut60bt"),
            ["hold", "range", "auto", "rel", "select"]
        );
        assert_eq!(commands("ut202bt"), ["hold", "range"]);

        for model in ["ut61eplus", "ut61b+", "ut61d+", "ut60bt", "ut202bt"] {
            let proto = Ut61PlusProtocol::for_model(model);
            for step in proto.capture_steps() {
                if let Some(command) = step.command {
                    assert!(commands(model).contains(&command), "{model}: {}", step.id);
                }
            }
        }
    }

    /// A command the model does not take is refused before it reaches the
    /// wire.
    #[test]
    fn a_command_the_model_does_not_take_is_refused() {
        let mock = MockTransport::new(vec![]);
        let mut ut60bt = Ut61PlusProtocol::for_model("ut60bt");
        assert!(matches!(
            ut60bt.send_command(&mock, "minmax"),
            Err(Error::UnsupportedCommand(Unsupported::Command(c))) if c == "minmax"
        ));
        let mut ut202bt = Ut61PlusProtocol::for_model("ut202bt");
        for command in ["auto", "rel", "peak"] {
            assert!(
                matches!(
                    ut202bt.send_command(&mock, command),
                    Err(Error::UnsupportedCommand(_))
                ),
                "{command}"
            );
        }
        assert!(mock.written.borrow().is_empty());
    }

    /// The flag settings follow the same buttons: no MAX/MIN on the UT60BT,
    /// and on the UT202BT HOLD alone — no Peak either, though its frames can
    /// carry the Peak bits, until the byte that starts it is known (family
    /// spec §6.5). Its range ladder goes too: it would come with an Auto
    /// rung, which AUTO sets.
    #[test]
    fn the_new_models_offer_only_the_settings_their_buttons_take() {
        use cycle::{CycleMeter, FlagSetting};
        let (dcv, acv) = (u16::from(Mode::DcV as u8), u16::from(Mode::AcV as u8));
        let ut60bt = Ut61PlusProtocol::for_model("ut60bt");
        assert!(ut60bt.flag_states(FlagSetting::MinMax, dcv).is_empty());
        assert_eq!(ut60bt.flag_states(FlagSetting::Rel, dcv), [0, 1]);
        assert_eq!(ut60bt.flag_states(FlagSetting::Hold, dcv), [0, 1]);
        assert!(!ut60bt.range_ladder(dcv).is_empty());

        let ut202bt = Ut61PlusProtocol::for_model("ut202bt");
        for setting in [FlagSetting::Rel, FlagSetting::MinMax, FlagSetting::Peak] {
            assert!(ut202bt.flag_states(setting, acv).is_empty(), "{setting:?}");
        }
        assert_eq!(ut202bt.flag_states(FlagSetting::Hold, acv), [0, 1]);
        assert!(ut202bt.range_ladder(dcv).is_empty());

        // The UT61E+ keeps every one of them.
        let e_plus = Ut61PlusProtocol::new();
        assert_eq!(e_plus.flag_states(FlagSetting::MinMax, dcv), [0, 1, 2]);
        assert_eq!(e_plus.flag_states(FlagSetting::Peak, acv), [0, 1, 2]);
        assert!(!e_plus.range_ladder(dcv).is_empty());
    }
    use tables::ut61e_plus::UT61E_PLUS;

    // --- Remote mode selection (protocol::cycle) --------------------------
    //
    // The cycles these exercise are verified on a real UT61E+ (research spec
    // §2.3/§2.5); what is not verified is the settle timing, which the fake
    // transport below sidesteps by answering instantly.

    use crate::protocol::cycle::{CycleButton, CycleMeter};
    use crate::protocol::framing::test_frame_be16;
    use crate::transport::mock::MockTransport;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    /// A frame carrying a DC-V-shaped reading in `mode`.
    fn frame_in(mode: u8) -> Vec<u8> {
        test_frame_be16(&make_payload(
            mode,
            0x01,
            b" 12.345",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ))
    }

    /// A mock on the Bluetooth link, for the adapter-only paths.
    struct BluetoothMock(MockTransport);

    impl Transport for BluetoothMock {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.0.write(data)
        }
        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
            self.0.read_timeout(buf, timeout_ms)
        }
        fn set_baud(&self, baud: u32) -> Result<()> {
            self.0.set_baud(baud)
        }
        fn link(&self) -> Option<Link> {
            Some(Link::Bluetooth)
        }
    }

    /// Over the adapter, init starts the stream and a reading is taken
    /// without a request of its own.
    #[test]
    fn over_bluetooth_readings_are_streamed_not_polled() {
        let mock = BluetoothMock(MockTransport::new(vec![frame_in(0x02)]));
        let mut p = Ut61PlusProtocol::new();
        p.init(&mock).unwrap();
        assert_eq!(
            mock.0.written.borrow().as_slice(),
            &[Command::StartStream.encode().to_vec()]
        );
        p.request_measurement(&mock).unwrap();
        mock.0.push_response(frame_in(0x02));
        p.request_measurement(&mock).unwrap();
        assert_eq!(mock.0.written.borrow().len(), 1, "no poll went out");
    }

    /// Get Name, then Start Stream: the order a UT60BT or UT202BT is started
    /// in over Bluetooth (family spec §6.4).
    fn name_then_start() -> [Vec<u8>; 2] {
        [
            Command::GetName.encode().to_vec(),
            Command::StartStream.encode().to_vec(),
        ]
    }

    /// Which models want their name asked before `init`: the two flagged ones,
    /// over Bluetooth only. The protocol keeps no name, so `init` on its own
    /// only starts the stream; [`crate::Dmm`] does the asking.
    #[test]
    fn only_the_flagged_models_want_their_name_first_and_only_on_bluetooth() {
        for model in [
            "ut61eplus",
            "ut161e",
            "ut61b+",
            "ut161b",
            "ut61d+",
            "ut161d",
            "ut60bt",
            "ut202bt",
        ] {
            let p = Ut61PlusProtocol::for_model(model);
            let flagged = matches!(model, "ut60bt" | "ut202bt");
            let radio = BluetoothMock(MockTransport::new(vec![]));
            assert_eq!(p.name_before_init(&radio), flagged, "{model}");
            assert!(
                !p.name_before_init(&MockTransport::new(vec![])),
                "{model} on a cable"
            );
        }
        let mock = BluetoothMock(MockTransport::new(vec![]));
        let mut p = Ut61PlusProtocol::for_model("ut60bt");
        p.init(&mock).unwrap();
        assert_eq!(
            mock.0.written.borrow().as_slice(),
            &[Command::StartStream.encode().to_vec()]
        );
    }

    /// A model that wants its name asked first is asked, waited for, then
    /// started, and later asks for the name get the one it gave without
    /// another write: every ask beeps the meter.
    #[test]
    fn a_model_that_wants_its_name_asked_first_gets_it_before_the_stream() {
        for (model, name) in [("ut60bt", "UT60BT"), ("ut202bt", "UT202BT")] {
            let mock = BluetoothMock(MockTransport::new(vec![
                ACK.to_vec(),
                test_frame_be16(name.as_bytes()),
                frame_in(0x02),
            ]));
            let p = Ut61PlusProtocol::for_model(model);
            let mut dmm = crate::Dmm::new(mock, Box::new(p)).unwrap();
            assert_eq!(
                dmm.transport().0.written.borrow().as_slice(),
                &name_then_start(),
                "{model}"
            );
            assert_eq!(dmm.request_measurement().unwrap().mode, "DC V");
            assert_eq!(dmm.get_name().unwrap().as_deref(), Some(name));
            assert_eq!(
                dmm.transport().0.written.borrow().len(),
                2,
                "{model}: no poll or ask went out"
            );
        }
    }

    /// Start Stream goes out only once the name has come back, the reply
    /// being waited for across the ack and a name split in two.
    #[test]
    fn the_stream_is_started_after_the_name_arrives() {
        /// Writes and non-empty reads, in the order they happen.
        struct Ordered {
            mock: MockTransport,
            log: RefCell<Vec<String>>,
        }
        impl Transport for Ordered {
            fn write(&self, data: &[u8]) -> Result<()> {
                self.log.borrow_mut().push(format!("write {:02X}", data[3]));
                self.mock.write(data)
            }
            fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
                let n = self.mock.read_timeout(buf, timeout_ms)?;
                if n > 0 {
                    self.log.borrow_mut().push(format!("read {n}"));
                }
                Ok(n)
            }
            fn set_baud(&self, baud: u32) -> Result<()> {
                self.mock.set_baud(baud)
            }
            fn link(&self) -> Option<Link> {
                Some(Link::Bluetooth)
            }
        }
        let name = test_frame_be16(b"UT60BT");
        let (head, tail) = name.split_at(4);
        let link = Ordered {
            mock: MockTransport::new(vec![ACK.to_vec(), head.to_vec(), tail.to_vec()]),
            log: RefCell::new(Vec::new()),
        };
        let p = Ut61PlusProtocol::for_model("ut60bt");
        let dmm = crate::Dmm::new(link, Box::new(p)).unwrap();
        assert_eq!(
            *dmm.transport().log.borrow(),
            ["write 5F", "read 7", "read 4", "read 7", "write 5D"]
        );
    }

    /// A meter that never names itself is started all the same, and a later
    /// ask goes to the meter.
    #[test]
    fn a_meter_that_gives_no_name_is_still_started() {
        let mock = BluetoothMock(MockTransport::new(vec![]));
        let p = Ut61PlusProtocol::for_model("ut60bt");
        let mut dmm = crate::Dmm::new(mock, Box::new(p)).unwrap();
        assert_eq!(
            dmm.transport().0.written.borrow().as_slice(),
            &name_then_start()
        );
        assert_eq!(dmm.known_name(), None);
        assert!(dmm.get_name().is_err(), "nothing answers");
        assert_eq!(dmm.transport().0.written.borrow().len(), 3);
    }

    /// A streaming protocol, started, over `frames`.
    fn streaming(frames: Vec<Vec<u8>>) -> (BluetoothMock, Ut61PlusProtocol) {
        let mock = BluetoothMock(MockTransport::new(frames));
        let mut p = Ut61PlusProtocol::new();
        p.init(&mock).unwrap();
        (mock, p)
    }

    /// Streamed readings come in the order the adapter sent them, with no
    /// poll written; the stream above picks among them.
    #[test]
    fn streamed_readings_come_in_order() {
        let (mock, mut p) = streaming(vec![frame_in(0x02), frame_in(0x04), frame_in(0x05)]);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "DC V");
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Hz");
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
        assert_eq!(mock.0.written.borrow().len(), 1, "no poll went out");
    }

    /// A frame split across notifications, or two in one, is read whole;
    /// the unfinished tail waits for its next read.
    #[test]
    fn frames_are_read_across_notifications() {
        let last = frame_in(0x05);
        let (head, tail) = last.split_at(7);
        let mut two = frame_in(0x02);
        two.extend(frame_in(0x04));
        let (mock, mut p) = streaming(vec![two, head.to_vec()]);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "DC V");
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Hz");
        mock.0.push_response(tail.to_vec());
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
    }

    // --- The secondary display (family spec §2.3) --------------------------

    /// A UT202BT secondary-display frame: `function` with bit 7 set.
    fn secondary(function: u8, range: u8, display: &[u8; 7]) -> Vec<u8> {
        make_payload(function | 0x80, range, display, (0, 0), (0, 0, 0))
    }

    fn main_frame(mode: u8, range: u8, display: &[u8; 7]) -> Vec<u8> {
        make_payload(mode, range, display, (0, 0), (0, 0, 0))
    }

    fn ut202bt() -> Ut61PlusProtocol {
        Ut61PlusProtocol::for_model("ut202bt")
    }

    /// Only the UT202BT has a secondary display, so only it has a sub-value
    /// slot in an export.
    #[test]
    fn a_sub_value_slot_for_a_secondary_display_or_ac_dc_v() {
        for model in [
            "ut61eplus",
            "ut161e",
            "ut61b+",
            "ut161b",
            "ut61d+",
            "ut161d",
            "ut60bt",
            "ut202bt",
        ] {
            let p = Ut61PlusProtocol::for_model(model);
            let want = usize::from(matches!(model, "ut202bt" | "ut61eplus" | "ut161e"));
            assert_eq!(p.profile().max_aux_values, want, "{model}");
        }
    }

    /// The captured pair across a 1.6 V cell: the DC frame is the reading,
    /// the AC frame carries only its component.
    #[test]
    fn ac_dc_v_splits_its_components() {
        let table = &super::tables::ut61e_plus::UT61E_PLUS;
        let dc = [
            0x19, 0x30, 0x20, 0x31, 0x2E, 0x36, 0x31, 0x31, 0x33, 0x00, 0x00, 0x30, 0x30, 0x31,
        ];
        let m = parse_measurement(&dc, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 1.6113));
        assert!(m.flags.dc);
        assert!(m.aux_values.is_empty());
        assert_eq!(m.main_label, Some(MainLabel::Dc));
        assert_eq!(m.to_string(), "DC 1.6113 V [AUTO]");

        let ac = [
            0x19, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x39,
        ];
        let m = parse_measurement(&ac, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Absent));
        assert_eq!(m.display_raw, None);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("AC+DC V", "V"));
        assert_eq!(m.range_label, "2.2V");
        assert!(!m.flags.dc);
        assert_eq!(m.raw_payload, ac, "the frame stays the AC frame's own");
        assert_eq!(
            m.main_label,
            Some(MainLabel::Dc),
            "named from the first frame"
        );
        assert_eq!(m.aux_values.len(), 1);
        let aux = &m.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("AC", ""));
        assert!(matches!(aux.value, MeasuredValue::Normal(v) if v == 0.0));
        assert_eq!(aux.display_raw.as_deref(), Some(" 0.0000"));
        assert_eq!(m.to_string(), "AC 0.0000 V [AUTO]");
    }

    /// An AC component over range is an over-range sub-value, not an
    /// over-range reading.
    #[test]
    fn an_over_range_ac_component_is_an_over_range_sub_value() {
        let table = &super::tables::ut61e_plus::UT61E_PLUS;
        let ac_ol = [
            0x19, 0x30, 0x20, 0x20, 0x20, 0x4F, 0x4C, 0x20, 0x20, 0x00, 0x00, 0x30, 0x30, 0x38,
        ];
        let m = parse_measurement(&ac_ol, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Absent));
        assert!(matches!(m.aux_values[0].value, MeasuredValue::Overload));
    }

    /// Frequency beside AC V, as the UT202BT manual describes (P8/14): one
    /// reading, carrying the secondary as its one sub-value.
    #[test]
    fn a_secondary_frame_rides_on_the_next_reading() {
        let mut p = ut202bt();
        let mut replies = vec![ACK.to_vec(), NAME_UT61EPLUS.to_vec()];
        replies.push(test_frame_be16(&secondary(0x04, 0x00, b"  50.02")));
        replies.push(test_frame_be16(&main_frame(0x00, 0x02, b"  230.4")));
        let mock = BluetoothMock(MockTransport::new(replies));
        p.init(&mock).unwrap();
        let (m, reports) = capture_reports(|| p.request_measurement(&mock).unwrap());
        assert!(reports.is_empty(), "{reports:?}");
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("AC V", "V"));
        assert_eq!(m.aux_values.len(), 1);
        let aux = &m.aux_values[0];
        assert_eq!(aux.label, "Frequency");
        assert_eq!(aux.value_str(), "50.02");
        assert_eq!(aux.unit, "Hz");
        assert_eq!(m.aux_summary(), "Frequency 50.02 Hz");
    }

    /// °F beside °C (manual P11/19), the unit from the table's °F row.
    #[test]
    fn fahrenheit_beside_celsius() {
        let mut p = ut202bt();
        assert!(p.take_reading(&secondary(0x0B, 0x00, b"   77  ")).is_none());
        let m = p
            .take_reading(&main_frame(0x0A, 0x01, b"   25  "))
            .unwrap()
            .unwrap();
        assert_eq!(m.unit, "°C");
        assert_eq!(m.aux_summary(), "Temperature 77 °F");
    }

    /// A secondary goes with the main frame after it and no further: a main
    /// frame with none since the one before it carries none.
    #[test]
    fn a_secondary_is_carried_once() {
        let mut p = ut202bt();
        let reading = |p: &mut Ut61PlusProtocol| {
            p.take_reading(&main_frame(0x00, 0x02, b"  230.4"))
                .unwrap()
                .unwrap()
        };
        p.take_reading(&secondary(0x04, 0x00, b"  50.02"));
        assert_eq!(reading(&mut p).aux_values.len(), 1);
        assert!(reading(&mut p).aux_values.is_empty());
        // The newest of two secondaries is the one kept.
        p.take_reading(&secondary(0x04, 0x00, b"  50.02"));
        p.take_reading(&secondary(0x04, 0x00, b"  49.98"));
        assert_eq!(reading(&mut p).aux_summary(), "Frequency 49.98 Hz");
    }

    /// A model with no secondary display refuses the frame: a reading would
    /// change the mode every other frame. The report says what the frame looks
    /// like and carries it whole, so a second meter that sends one shows up.
    #[test]
    fn a_secondary_frame_on_a_model_without_one_is_refused() {
        for model in ["ut61eplus", "ut60bt"] {
            let mut p = Ut61PlusProtocol::for_model(model);
            let frame = secondary(0x04, 0x00, b"  50.02");
            let (reading, reports) = capture_reports(|| p.take_reading(&frame));
            assert!(
                matches!(reading, Some(Err(Error::InvalidResponse { .. }))),
                "{model}: {reading:?}"
            );
            assert_eq!(
                reports,
                [format!(
                    "{model}: unrecognised mode byte: 0x84, which looks like a \
                     secondary-display frame for Hz (family spec §2.3): {frame:02X?}"
                )],
                "{model}"
            );
        }
    }

    /// A report names the entry being read, which its hint passes to
    /// `--device`: the UT61E+ entry can't open a UT60BT or UT202BT, and
    /// reads a UT61B+ or UT161 with the wrong tables.
    #[test]
    fn reports_name_the_entry_being_read() {
        for device in DEVICES
            .iter()
            .filter(|d| d.family == DeviceFamily::Ut61EPlus)
        {
            let mut p = Ut61PlusProtocol::for_model(device.id);
            let frame = main_frame(0x7F, 0x30, b"  0.000");
            let (_, reports) = capture_reports(|| p.take_reading(&frame));
            assert_eq!(
                reports,
                [format!("{}: unrecognised mode byte: 0x7f", device.id)]
            );
        }
    }

    /// A secondary function or range the table lacks is reported, not shown.
    #[test]
    fn a_secondary_the_table_lacks_is_reported() {
        let mut p = ut202bt();
        // DC µA is not on the UT202BT; Hz stops at range byte 5.
        for frame in [
            secondary(0x0C, 0x00, b"  12.34"),
            secondary(0x04, 0x06, b"  12.34"),
        ] {
            let (_, reports) = capture_reports(|| p.take_reading(&frame));
            assert_eq!(
                reports,
                [format!(
                    "ut202bt: unrecognised secondary display: {frame:02X?}"
                )]
            );
            let m = p
                .take_reading(&main_frame(0x00, 0x02, b"  230.4"))
                .unwrap()
                .unwrap();
            assert!(m.aux_values.is_empty());
        }
    }

    /// An unrecognised secondary replaces the one held before it: the older
    /// value is no longer what the meter shows.
    #[test]
    fn an_unrecognised_secondary_drops_the_held_one() {
        let mut p = ut202bt();
        p.take_reading(&secondary(0x04, 0x00, b"  50.02"));
        // DC µA is not on the UT202BT.
        let (_, reports) = capture_reports(|| p.take_reading(&secondary(0x0C, 0x00, b"  12.34")));
        assert_eq!(reports.len(), 1, "{reports:?}");
        let m = p
            .take_reading(&main_frame(0x00, 0x02, b"  230.4"))
            .unwrap()
            .unwrap();
        assert!(m.aux_values.is_empty(), "{:?}", m.aux_values);
    }

    /// A stream of secondaries alone is never read as a reading.
    #[test]
    fn a_secondary_frame_is_never_a_reading() {
        let mut p = ut202bt();
        let frame = test_frame_be16(&secondary(0x04, 0x00, b"  50.02"));
        let mock = BluetoothMock(MockTransport::new(vec![
            ACK.to_vec(),
            NAME_UT61EPLUS.to_vec(),
            frame.clone(),
            frame,
        ]));
        p.init(&mock).unwrap();
        assert!(matches!(p.request_measurement(&mock), Err(Error::Timeout)));
    }

    /// Over a cable nothing is started and every reading is a request.
    #[test]
    fn over_a_cable_every_reading_is_polled() {
        let mock = MockTransport::new(vec![frame_in(0x02)]);
        let mut p = Ut61PlusProtocol::new();
        p.init(&mock).unwrap();
        assert!(mock.written.borrow().is_empty());
        p.request_measurement(&mock).unwrap();
        assert_eq!(
            mock.written.borrow().as_slice(),
            &[Command::GetMeasurement.encode().to_vec()]
        );
    }

    /// An adapter that never streams is polled instead of timing out.
    #[test]
    fn a_silent_stream_is_polled() {
        let mock = BluetoothMock(MockTransport::new(vec![]));
        let mut p = Ut61PlusProtocol::new();
        p.init(&mock).unwrap();
        assert!(matches!(p.request_measurement(&mock), Err(Error::Timeout)));
        assert_eq!(
            mock.0.written.borrow().as_slice(),
            &[
                Command::StartStream.encode().to_vec(),
                Command::StartStream.encode().to_vec(),
                Command::GetMeasurement.encode().to_vec()
            ]
        );
    }

    #[test]
    fn press_writes_the_select_and_hz_frames() {
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut61PlusProtocol::new();
        proto.press(&mock, CycleButton::Select).unwrap();
        proto.press(&mock, CycleButton::Hz).unwrap();

        let written = mock.written.borrow();
        // 0x4C + 379 = 0x01C7, 0x49 + 379 = 0x01C4.
        assert_eq!(written.len(), 2, "{written:02X?}");
        assert_eq!(written[0], [0xAB, 0xCD, 0x03, 0x4C, 0x01, 0xC7]);
        assert_eq!(written[1], [0xAB, 0xCD, 0x03, 0x49, 0x01, 0xC4]);
    }

    /// A frame already in flight when the press landed is discarded with the
    /// ack; the reading after the ack is the next poll's.
    #[test]
    fn a_press_waits_for_the_ack_and_leaves_what_follows() {
        let mock = MockTransport::new(vec![
            vec![],
            frame_in(0x02),
            vec![],
            ACK_FRAME.to_vec(),
            frame_in(0x19),
        ]);
        let mut proto = Ut61PlusProtocol::new();
        proto.press(&mock, CycleButton::Select).unwrap();
        assert_eq!(proto.request_measurement(&mock).unwrap().mode, "AC+DC V");
    }

    /// A CP2110 can hand the ack over in two reads (`ut61eplus-verify5.yaml`).
    #[test]
    fn an_ack_split_across_reads_ends_the_wait() {
        let (head, tail) = ACK_FRAME.split_at(3);
        let mock = MockTransport::new(vec![head.to_vec(), vec![], tail.to_vec(), frame_in(0x05)]);
        let mut proto = Ut61PlusProtocol::new();
        proto.press(&mock, CycleButton::Hz).unwrap();
        assert_eq!(proto.request_measurement(&mock).unwrap().mode, "Duty %");
    }

    #[test]
    fn a_press_without_an_ack_gives_up_and_the_poll_still_reads() {
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut61PlusProtocol::new();
        proto.press(&mock, CycleButton::Hold).unwrap();
        mock.push_response(frame_in(0x04));
        assert_eq!(proto.request_measurement(&mock).unwrap().mode, "Hz");
    }

    #[test]
    fn mode_choices_on_the_dc_volts_dial_offer_ac_dc() {
        let mock = MockTransport::new(vec![frame_in(0x02)]);
        let mut proto = Ut61PlusProtocol::new();
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.mode, "DC V");

        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(
            choices.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![0x02, 0x19]
        );
        assert_eq!(choices[0].label, "DC V");
        assert!(choices[0].current, "DC V is the live mode");
        assert_eq!(choices[1].label, "AC+DC V");
        assert!(!choices[1].current);
    }

    /// A UT61E+ on the V⎓ dial: it answers 0x5E with a reading, and SELECT
    /// (0x4C) flips DC V ↔ AC+DC V and acks with `FF 00`.
    ///
    /// `MockTransport` cannot stand in here — it ignores writes, so a press
    /// waiting for its ack would swallow a queued measurement frame.
    struct VoltsDial {
        mode: Cell<u8>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        presses: Cell<usize>,
    }

    impl VoltsDial {
        fn new(mode: u8) -> Self {
            Self {
                mode: Cell::new(mode),
                queued: RefCell::new(VecDeque::new()),
                presses: Cell::new(0),
            }
        }
    }

    impl Transport for VoltsDial {
        fn link(&self) -> Option<crate::transport::Link> {
            None
        }

        fn write(&self, data: &[u8]) -> Result<()> {
            match data.get(3) {
                Some(&0x5E) => self
                    .queued
                    .borrow_mut()
                    .push_back(frame_in(self.mode.get())),
                Some(&0x4C) => {
                    self.presses.set(self.presses.get() + 1);
                    self.mode
                        .set(if self.mode.get() == 0x02 { 0x19 } else { 0x02 });
                    // The meter acks a press before the next reading.
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                _ => {}
            }
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }
    }

    #[test]
    fn select_mode_presses_once_and_the_stream_keeps_parsing() {
        let meter = VoltsDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        assert_eq!(proto.request_measurement(&meter).unwrap().mode, "DC V");

        proto.select(&meter, Setting::Mode, 0x19).expect("switched");
        assert_eq!(meter.presses.get(), 1, "one press per ring step");
        assert_eq!(proto.dial.last_mode(), Some(0x19));

        // The ack must not have been left in the buffer for the next read.
        let m = proto.request_measurement(&meter).unwrap();
        assert_eq!(m.mode, "AC+DC V");
        assert_eq!(m.range_label, "22V");
    }

    // --- Range selection (protocol::cycle) --------------------------------

    /// Flag nibble 2 with the MANUAL range bit set (`flags.auto_range` is
    /// its inverse).
    const MANUAL: u8 = 0x04;

    fn range_ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().map(|c| c.id).collect()
    }

    #[test]
    fn dc_volts_offers_auto_and_the_four_manual_ranges() {
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, MANUAL, 0));
        let proto = Ut61PlusProtocol::new();
        let choices = proto.choices(Setting::Range, &m);
        assert_eq!(range_ids(&choices), vec![0, 1, 2, 3, 4]);
        let labels: Vec<_> = choices.iter().map(|c| c.label.as_ref()).collect();
        assert_eq!(labels, vec!["Auto", "2.2V", "22V", "220V", "1000V"]);
        // Range byte 1 is the second rung.
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
    fn an_auto_ranging_reading_marks_auto_current() {
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        let proto = Ut61PlusProtocol::new();
        let choices = proto.choices(Setting::Range, &m);
        assert!(choices[0].current, "auto is the live choice");
        assert!(choices[1..].iter().all(|c| !c.current));
    }

    /// The mV dial has one range on the E+ and RANGE does nothing in DC mV
    /// or AC mV; DC A's table is a placeholder plus the one verified 20A
    /// entry; diode has a single entry. None of the four is a ladder.
    #[test]
    fn modes_without_a_ladder_offer_no_ranges() {
        let proto = Ut61PlusProtocol::new();
        for (mode, range) in [(0x03, 0x00), (0x01, 0x00), (0x10, 0x01), (0x08, 0x00)] {
            let m = make_test_measurement(mode, range, b" 12.345", (0, 0), (0, MANUAL, 0));
            assert!(
                proto.choices(Setting::Range, &m).is_empty(),
                "{} should offer no range",
                m.mode
            );
        }
    }

    /// A UT61E+ on the V⎓ dial whose RANGE button (0x46) steps the four DC V
    /// ranges and whose AUTO button (0x47) returns to auto-ranging.
    struct RangeDial {
        range: Cell<u8>,
        manual: Cell<bool>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        presses: Cell<usize>,
        autos: Cell<usize>,
    }

    impl RangeDial {
        fn auto() -> Self {
            Self {
                range: Cell::new(1),
                manual: Cell::new(false),
                queued: RefCell::new(VecDeque::new()),
                presses: Cell::new(0),
                autos: Cell::new(0),
            }
        }

        fn manual_at(range: u8) -> Self {
            let dial = Self::auto();
            dial.range.set(range);
            dial.manual.set(true);
            dial
        }
    }

    impl Transport for RangeDial {
        fn link(&self) -> Option<crate::transport::Link> {
            None
        }

        fn write(&self, data: &[u8]) -> Result<()> {
            match data.get(3) {
                Some(&0x5E) => {
                    let flag2 = if self.manual.get() { MANUAL } else { 0 };
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&make_payload(
                            0x02,
                            self.range.get(),
                            b" 12.345",
                            (0x00, 0x00),
                            (0x00, flag2, 0x00),
                        )));
                }
                Some(&0x46) => {
                    self.presses.set(self.presses.get() + 1);
                    // The first press engages manual ranging where auto had
                    // left the meter; later ones step the ladder.
                    if self.manual.replace(true) {
                        self.range.set((self.range.get() + 1) % 4);
                    }
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                Some(&0x47) => {
                    self.autos.set(self.autos.get() + 1);
                    self.manual.set(false);
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                _ => {}
            }
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }
    }

    #[test]
    fn selecting_a_range_presses_the_range_button_to_it() {
        let meter = RangeDial::manual_at(1);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Range, 4).expect("switched");
        assert_eq!(meter.presses.get(), 2, "22V -> 220V -> 1000V");
        assert_eq!(meter.range.get(), 3);
        assert_eq!(meter.autos.get(), 0);
    }

    #[test]
    fn selecting_auto_sends_the_auto_command_and_confirms_it() {
        let meter = RangeDial::manual_at(2);
        let mut proto = Ut61PlusProtocol::new();
        proto
            .select(&meter, Setting::Range, 0)
            .expect("back to auto");
        assert_eq!(meter.autos.get(), 1);
        assert_eq!(meter.presses.get(), 0);
        assert!(!meter.manual.get());
    }

    #[test]
    fn a_meter_already_auto_ranging_is_not_told_to_be() {
        let meter = RangeDial::auto();
        let mut proto = Ut61PlusProtocol::new();
        proto
            .select(&meter, Setting::Range, 0)
            .expect("already auto");
        assert_eq!(meter.autos.get(), 0);
        assert_eq!(meter.presses.get(), 0);
    }

    // --- Flag-backed settings (HOLD, REL, MIN/MAX, Peak) ------------------

    /// Flag nibble 1 bits: REL is bit 0, HOLD bit 1, MIN bit 2, MAX bit 3.
    const F_REL: u8 = 0x01;
    const F_HOLD: u8 = 0x02;
    const F_MIN: u8 = 0x04;
    const F_MAX: u8 = 0x08;
    /// Flag nibble 3 bits: P-MIN is bit 1, P-MAX bit 2.
    const F_PEAK_MIN: u8 = 0x02;
    const F_PEAK_MAX: u8 = 0x04;

    fn flag_labels(choices: &[Choice]) -> Vec<String> {
        choices.iter().map(|c| c.label.to_string()).collect()
    }

    #[test]
    fn hold_and_rel_choices_follow_the_flags() {
        let proto = Ut61PlusProtocol::new();
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_HOLD, 0, 0));

        let hold = proto.choices(Setting::Hold, &m);
        assert_eq!(range_ids(&hold), vec![0, 1]);
        assert_eq!(flag_labels(&hold), vec!["off", "on"]);
        assert_eq!(hold.iter().filter(|c| c.current).count(), 1);
        assert!(hold[1].current, "HOLD is lit");

        let rel = proto.choices(Setting::Rel, &m);
        assert_eq!(range_ids(&rel), vec![0, 1]);
        assert!(rel[0].current, "REL is dark");
    }

    #[test]
    fn minmax_choices_are_the_two_state_ring_plus_off() {
        let proto = Ut61PlusProtocol::new();
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_MIN, MANUAL, 0));
        let choices = proto.choices(Setting::MinMax, &m);
        assert_eq!(range_ids(&choices), vec![0, 1, 2]);
        assert_eq!(flag_labels(&choices), vec!["off", "MAX", "MIN"]);
        assert!(choices[2].current, "MIN is lit");

        let max = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_MAX, MANUAL, 0));
        assert!(proto.choices(Setting::MinMax, &max)[1].current);
    }

    /// A step is verified where it has *run*, which is not the same as its
    /// family being verified: the UT61B+ reached `Stability::Verified` before
    /// the two ladder steps existed and was asked for them until it walked
    /// them (issue #19, 2026-09-11). The verified models have now run every
    /// step they declare, so `capture --unverified` asks them for nothing.
    #[test]
    fn verified_models_are_asked_for_nothing() {
        let unverified = |model: &str| -> Vec<&'static str> {
            Ut61PlusProtocol::for_model(model)
                .capture_steps()
                .into_iter()
                .filter(|s| !s.verified)
                .map(|s| s.id)
                .collect()
        };
        assert!(
            unverified("ut61eplus").is_empty(),
            "the UT61E+ has run every step it declares: {:?}",
            unverified("ut61eplus")
        );
        assert!(
            unverified("ut61b+").is_empty(),
            "the UT61B+ has run every step it declares: {:?}",
            unverified("ut61b+")
        );
        assert!(
            unverified("ut161e").is_empty(),
            "the UT161E has run every step it declares: {:?}",
            unverified("ut161e")
        );
    }

    /// The modes where a press leaves the flag where it was offer nothing to
    /// press. Both a UT61E+ (2026-09-07) and a UT61B+ (issue #19, 2026-09-10)
    /// refused these, mode for mode — diode's REL on 2026-09-11, once both
    /// had been asked with a diode fitted instead of over OL.
    #[test]
    fn a_dead_flag_offers_nothing_to_switch_to() {
        let proto = Ut61PlusProtocol::new();
        // (mode byte, HOLD, REL, MIN/MAX) — true means the meter takes it.
        let cases = [
            (0x07u8, true, false, false), // continuity
            (0x08, true, false, false),   // diode
            (0x09, true, true, false),    // capacitance
            (0x04, true, false, false),   // Hz
            (0x05, true, false, false),   // duty %
            (0x14, false, false, false),  // NCV
            (0x19, true, false, true),    // AC+DC V
            (0x02, true, true, true),     // DC V, the control
        ];
        for (mode, hold, rel, minmax) in cases {
            let m = make_test_measurement(mode, 0x00, b" 12.345", (0, 0), (0, MANUAL, 0));
            for (setting, want, name) in [
                (Setting::Hold, hold, "HOLD"),
                (Setting::Rel, rel, "REL"),
                (Setting::MinMax, minmax, "MIN/MAX"),
            ] {
                assert_eq!(
                    !proto.choices(setting, &m).is_empty(),
                    want,
                    "{name} in mode {mode:#04x}"
                );
            }
        }
    }

    /// RANGE is dead in capacitance and Hz on every model of the family, so
    /// no ladder is offered there even though both tables name their rungs.
    #[test]
    fn capacitance_and_hz_offer_no_range_ladder() {
        for model in ["ut61eplus", "ut61b+", "ut61d+"] {
            let proto = Ut61PlusProtocol::for_model(model);
            for mode in [0x09u8, 0x04] {
                let m = make_test_measurement(mode, 0x00, b"   0.00", (0, 0), (0, 0, 0));
                assert!(
                    proto.choices(Setting::Range, &m).is_empty(),
                    "{model} mode {mode:#04x} should offer no range"
                );
                // The rungs are still named, for whatever auto-ranging picks.
                assert!(!m.range_label.is_empty(), "{model} mode {mode:#04x} label");
            }
        }
    }

    /// Peak activates on AC mV and does nothing on DC V, verified 2026-03-21
    /// (UT61E+ spec §2.7).
    #[test]
    fn peak_is_offered_in_ac_but_not_in_dc_volts() {
        let proto = Ut61PlusProtocol::new();
        let ac = make_test_measurement(0x01, 0x00, b"  8.700", (0, 0), (0, 0, F_PEAK_MAX));
        let choices = proto.choices(Setting::Peak, &ac);
        assert_eq!(range_ids(&choices), vec![0, 1, 2]);
        assert_eq!(flag_labels(&choices), vec!["off", "P-MAX", "P-MIN"]);
        assert!(choices[1].current, "P-MAX is lit");

        let dc = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        assert!(proto.choices(Setting::Peak, &dc).is_empty());
    }

    /// The B+ has no Peak flags and its command matrix marks 0x4D/0x4E "No
    /// effect" (research spec §4 and §6).
    #[test]
    fn the_b_plus_offers_no_peak_anywhere() {
        let proto = Ut61PlusProtocol::for_model("ut61b+");
        let ac = make_test_measurement(0x01, 0x00, b"  8.700", (0, 0), (0, 0, 0));
        assert!(proto.choices(Setting::Peak, &ac).is_empty());
        // The buttons it does have are still offered.
        assert_eq!(proto.choices(Setting::MinMax, &ac).len(), 3);
    }

    /// A UT61E+ whose HOLD (0x4A), MIN/MAX (0x41) and Peak (0x4D) buttons do
    /// what the 2026-03-21 capture saw, and whose 0x42/0x4E leave those
    /// states.
    struct FlagDial {
        mode: u8,
        flag1: Cell<u8>,
        flag3: Cell<u8>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        writes: RefCell<Vec<u8>>,
    }

    impl FlagDial {
        fn new(mode: u8) -> Self {
            Self {
                mode,
                flag1: Cell::new(0),
                flag3: Cell::new(0),
                queued: RefCell::new(VecDeque::new()),
                writes: RefCell::new(Vec::new()),
            }
        }

        /// Step a two-state ring held in `cell`: off enters the first state,
        /// and the two swap from there. Never returns to off.
        fn ring(cell: &Cell<u8>, first: u8, second: u8) {
            let now = cell.get();
            let next = if now & first != 0 { second } else { first };
            cell.set((now & !(first | second)) | next);
        }

        fn ack(&self) {
            self.queued
                .borrow_mut()
                .push_back(test_frame_be16(&[0xFF, 0x00]));
        }
    }

    impl Transport for FlagDial {
        fn link(&self) -> Option<crate::transport::Link> {
            None
        }

        fn write(&self, data: &[u8]) -> Result<()> {
            let Some(&cmd) = data.get(3) else {
                return Ok(());
            };
            self.writes.borrow_mut().push(cmd);
            match cmd {
                0x5E => {
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&make_payload(
                            self.mode,
                            0x00,
                            b" 12.345",
                            (0x00, 0x00),
                            (self.flag1.get(), 0x00, self.flag3.get()),
                        )));
                    return Ok(());
                }
                0x4A => self.flag1.set(self.flag1.get() ^ F_HOLD),
                0x48 => self.flag1.set(self.flag1.get() ^ F_REL),
                0x41 => Self::ring(&self.flag1, F_MAX, F_MIN),
                0x42 => self.flag1.set(self.flag1.get() & !(F_MAX | F_MIN)),
                0x4D => Self::ring(&self.flag3, F_PEAK_MAX, F_PEAK_MIN),
                0x4E => self
                    .flag3
                    .set(self.flag3.get() & !(F_PEAK_MAX | F_PEAK_MIN)),
                _ => {}
            }
            self.ack();
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }
    }

    /// The command bytes the meter was sent, measurement requests aside.
    fn commands(meter: &FlagDial) -> Vec<u8> {
        meter
            .writes
            .borrow()
            .iter()
            .copied()
            .filter(|&c| c != 0x5E)
            .collect()
    }

    #[test]
    fn holding_presses_the_hold_command_once() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Hold, 1).expect("held");
        assert_eq!(commands(&meter), vec![0x4A]);
        assert_eq!(meter.flag1.get(), F_HOLD);
    }

    #[test]
    fn reaching_min_presses_the_minmax_button_twice() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::MinMax, 2).expect("in MIN");
        assert_eq!(commands(&meter), vec![0x41, 0x41], "off -> MAX -> MIN");
        assert_eq!(meter.flag1.get(), F_MIN);
    }

    #[test]
    fn leaving_minmax_sends_the_exit_command() {
        let meter = FlagDial::new(0x02);
        meter.flag1.set(F_MAX);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::MinMax, 0).expect("left");
        assert_eq!(commands(&meter), vec![0x42]);
        assert_eq!(meter.flag1.get(), 0);
    }

    #[test]
    fn reaching_peak_min_presses_the_peak_button_and_leaves_by_its_own() {
        let meter = FlagDial::new(0x01);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Peak, 2).expect("in P-MIN");
        assert_eq!(commands(&meter), vec![0x4D, 0x4D]);
        assert_eq!(meter.flag3.get(), F_PEAK_MIN);

        proto.select(&meter, Setting::Peak, 0).expect("left peak");
        assert_eq!(commands(&meter), vec![0x4D, 0x4D, 0x4E]);
        assert_eq!(meter.flag3.get(), 0);
    }

    #[test]
    fn peak_in_dc_volts_is_refused_without_writing() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.request_measurement(&meter).unwrap();
        let err = proto.select(&meter, Setting::Peak, 1).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(Unsupported::SettingNotInMode {
                setting: Setting::Peak,
                mode,
            }) if mode == "DC V"),
            "{err}"
        );
        assert!(commands(&meter).is_empty());
    }

    /// DC V on the second rung, with bar-graph digits reading 26.
    #[test]
    fn parse_dc_voltage() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x02, 0x01, b" 12.345", (0x02, 0x06), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x01
value=Normal(12.345)
unit=V
range_label=22V
progress=26
display_raw=Some(" 12.345")
flags=auto_range
aux=0
raw_payload=14"#
        );
    }

    /// "OL" in the digits is an overload, whatever the digits parse to.
    #[test]
    fn parse_overload() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x06
range_raw=0x00
value=Overload
unit=Ω
range_label=220Ω
progress=0
display_raw=Some("    OL ")
flags=auto_range
aux=0
raw_payload=14"#
        );
    }

    /// Flag nibble 1 bit 1 is HOLD; REL (bit 0) stays clear and AUTO is on.
    #[test]
    fn parse_with_hold_flag() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range);
        assert!(!m.flags.rel);
    }

    /// The meter puts a space between the sign and the digits.
    #[test]
    fn parse_negative_with_space() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x03, 0x00, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-55.79)).abs() < 1e-6));
    }

    #[test]
    fn parse_payload_too_short() {
        let table = &UT61E_PLUS;
        let payload = vec![0x30; 10];
        assert!(parse_measurement(&payload, table).is_err());
    }

    #[test]
    fn display_format() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }

    /// Every form the overload display takes, taken off the captures in the
    /// ut61-family spec §5.8.
    #[test]
    fn overload_is_read_wherever_the_decimal_point_lands() {
        let table = &UT61E_PLUS;
        for display in [b" .OL   ", b"  O.L  ", b"  OL.  ", b"    OL "] {
            let payload = make_payload(0x06, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            let m = parse_measurement(&payload, table).unwrap();
            assert!(
                matches!(m.value, MeasuredValue::Overload),
                "display {:?}: {:?}",
                String::from_utf8_lossy(display),
                m.value
            );
        }
        // A reading that merely contains the letters is still a reading.
        let payload = make_payload(0x06, 0x00, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(_)), "{:?}", m.value);
    }

    /// A number in NCV is none of its forms (no meter has been seen to send
    /// one: a UT61E+ and a UT61B+ draw "-" segments), so it is no reading.
    #[test]
    fn parse_ncv_number_is_no_reading() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let (m, reports) = crate::protocol::capture_reports(|| parse_measurement(&payload, table));
        let m = m.unwrap();
        assert_eq!(m.mode, "NCV");
        assert!(
            matches!(m.value, MeasuredValue::NoReading("?")),
            "{:?}",
            m.value
        );
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// NCV level from the "-" segments (manual §13). Only "EF" and a single
    /// dash have been seen on hardware; two dashes follow our counting rule.
    #[test]
    fn parse_ncv_dash_levels() {
        let table = &UT61E_PLUS;
        for (display, level) in [(b"   EF  ", 0u8), (b"     - ", 1), (b"    -- ", 2)] {
            let payload = make_payload(0x14, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            let m = parse_measurement(&payload, table).unwrap();
            assert!(
                matches!(m.value, MeasuredValue::NcvLevel(l) if l == level),
                "display {display:?}: {:?}",
                m.value
            );
        }
    }

    #[test]
    fn ncv_level_reads_every_known_form() {
        assert_eq!(ncv_level("EF"), Some(0));
        assert_eq!(ncv_level("-"), Some(1));
        assert_eq!(ncv_level("----"), Some(4));
        assert_eq!(ncv_level("3"), None);
        assert_eq!(ncv_level("-?"), None);
        assert_eq!(ncv_level("CUT"), None);
        assert_eq!(ncv_level(""), None);
    }

    /// Text no form covers — e.g. the "CUT" the manual says an overheated
    /// meter shows — is no reading, and is reported; the idle "EF" is not.
    #[test]
    fn unrecognised_display_text_is_reported() {
        let table = &UT61E_PLUS;
        let parse = |mode, display: &[u8; 7]| {
            let payload = make_payload(mode, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            crate::protocol::capture_reports(|| parse_measurement(&payload, table).unwrap())
        };

        let (m, reports) = parse(0x06, b"  CUT  ");
        assert!(matches!(m.value, MeasuredValue::NoReading("?")));
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised display text: \"CUT\", shown as no reading"]
        );

        let (m, reports) = parse(0x14, b"  ?!   ");
        assert!(
            matches!(m.value, MeasuredValue::NoReading("?")),
            "{:?}",
            m.value
        );
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised NCV display text: \"?!\", shown as no reading"]
        );

        let (_, reports) = parse(0x14, b"   EF  ");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// Frames from a UT61B+ capture reported 2026-09-09 (v0.6.0 report, steps
    /// `dcv`, `acv`, `ohm`, `dcma`). The meter showed volts on both voltage
    /// steps while range byte 0 was read as 60mV, labelling them a thousandth
    /// low; the other modes were already right and must stay so.
    #[test]
    fn parse_ut61b_plus_capture_frames() {
        let table = &tables::ut61b_plus::UT61B_PLUS;

        let dcv = [
            0x02, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        assert_eq!(
            snapshot(&parse_measurement(&dcv, table).unwrap()),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x00
value=Normal(0.0)
unit=V
range_label=6V
progress=0
display_raw=Some("  0.000")
flags=auto_range
aux=0
raw_payload=14"#
        );

        let acv = [
            0x00, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x33, 0x37, 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        assert_eq!(
            snapshot(&parse_measurement(&acv, table).unwrap()),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x00
value=Normal(0.037)
unit=V
range_label=6V
progress=0
display_raw=Some("  0.037")
flags=auto_range
aux=0
raw_payload=14"#
        );

        // Open leads on the top resistance rung: index 5 of six, unchanged.
        let ohm = [
            0x06, 0x35, 0x20, 0x20, 0x20, 0x4F, 0x2E, 0x4C, 0x20, 0x03, 0x00, 0x30, 0x30, 0x30,
        ];
        let m = parse_measurement(&ohm, table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!((m.unit.as_ref(), m.range_label.as_ref()), ("MΩ", "60MΩ"));
        assert_eq!(m.progress, Some(30));

        let dcma = [
            0x0E, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        let m = parse_measurement(&dcma, table).unwrap();
        assert_eq!((m.unit.as_ref(), m.range_label.as_ref()), ("mA", "60mA"));
    }

    // --- Detection (crate::detect) ---------------------------------------

    /// The ack and the ASCII name frame the meter answers Get Name with,
    /// both captured from our UT61E+ over CP2110.
    const ACK: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];
    const NAME_UT61EPLUS: [u8; 11] = [
        0xAB, 0xCD, 0x08, 0x55, 0x54, 0x36, 0x31, 0x45, 0x2B, 0x03, 0x00,
    ];
    /// The same reply from a UT61B+ over CH9329 (issue #19).
    const NAME_UT61BPLUS: [u8; 11] = [
        0xAB, 0xCD, 0x08, 0x55, 0x54, 0x36, 0x31, 0x42, 0x2B, 0x02, 0xFD,
    ];

    fn recognised(buf: &[u8]) -> Option<Evidence> {
        (FINGERPRINT.recognise)(buf, &Probing::default())
    }

    /// Auto-detection picks the entry from the name the meter reports, so
    /// every model in the family has to be reachable by its own display name
    /// — a new entry whose name does not round-trip would be detected as a
    /// plain UT61E+ and decoded with the wrong table.
    #[test]
    fn reported_names_round_trip_for_the_whole_family() {
        for device in DEVICES
            .iter()
            .filter(|d| d.family == DeviceFamily::Ut61EPlus)
        {
            let found = device_for_reported_name(device.model)
                .unwrap_or_else(|| panic!("{} does not round-trip", device.display_name()));
            assert_eq!(found.id, device.id);
        }
    }

    /// The meter's ASCII is uppercase, but the lookup must not depend on it.
    #[test]
    fn reported_name_is_case_insensitive() {
        assert_eq!(device_for_reported_name("ut61b+").unwrap().id, "ut61b+");
    }

    /// A UT181A never sends a name frame; if something else ever put that
    /// string in one, falling back to the UT61E+ tables beats decoding
    /// LE16 frames as BE16 ones.
    #[test]
    fn reported_name_ignores_other_families() {
        assert!(device_for_reported_name("UT181A").is_none());
        assert!(device_for_reported_name("mock").is_none());
    }

    /// The name frame is the only reply that pins the exact model, and each
    /// sibling's name is its own registry entry.
    #[test]
    fn a_name_frame_picks_the_registry_entry() {
        for (frame, device, name) in [
            (NAME_UT61EPLUS, &devices::UT61EPLUS, "UT61E+"),
            (NAME_UT61BPLUS, &devices::UT61BPLUS, "UT61B+"),
        ] {
            assert_eq!(
                recognised(&frame),
                Some(Evidence::Model {
                    device,
                    reported_name: Some(name.to_string()),
                })
            );
        }
    }

    /// A name no registry entry carries still identifies the family: the
    /// UT61E+ tables are the fallback and the name is kept for the user.
    #[test]
    fn an_unknown_name_falls_back_to_the_ut61eplus_tables() {
        assert_eq!(
            recognised(&test_frame_be16(b"UT216XD")),
            Some(Evidence::Model {
                device: &devices::UT61EPLUS,
                reported_name: Some("UT216XD".to_string()),
            })
        );
    }

    /// On a meter with Bluetooth built in, a name no registry entry carries
    /// falls back to the model its advertised name picked, and the report
    /// says so. The unflagged case is `an_unknown_model_name_is_reported`.
    #[test]
    fn an_unknown_name_on_a_built_in_meter_falls_back_to_its_own_entry() {
        let probing = Probing {
            advertised: registry::find_device("ut60bt"),
            ..Probing::default()
        };
        let (evidence, reports) = crate::protocol::capture_reports(|| {
            (FINGERPRINT.recognise)(&test_frame_be16(b"UT216XD"), &probing)
        });
        assert_eq!(
            evidence,
            Some(Evidence::Model {
                device: &devices::UT60BT,
                reported_name: Some("UT216XD".to_string()),
            })
        );
        assert_eq!(
            reports,
            [
                "ut60bt: unrecognised model name: \"UT216XD\", using the UNI-T UT60BT tables, the \
              model its Bluetooth name advertises"
            ]
        );
    }

    /// The meters with Bluetooth built in answer Get Name with their own
    /// names (one UT60BT's reply is on record in the family approach doc,
    /// 2026-09-25), which pick their own tables.
    #[test]
    fn the_bluetooth_meters_names_pick_their_own_tables() {
        for (name, device) in [("UT60BT", &devices::UT60BT), ("UT202BT", &devices::UT202BT)] {
            assert_eq!(
                recognised(&test_frame_be16(name.as_bytes())),
                Some(Evidence::Model {
                    device,
                    reported_name: Some(name.to_string()),
                })
            );
            assert_eq!((device.new_protocol)().profile().id, device.id);
        }
        // Range byte 0 of DC V is 999.9 mV on a UT60BT, 9.999 V on a UT202BT.
        let payload = make_payload(0x02, 0, b" 12.345", (0, 0), (0, 0, 0));
        let unit = |id| {
            let proto = Ut61PlusProtocol::for_model(id);
            proto.parse_payload(&payload).expect("parses").unit
        };
        assert_eq!(unit("ut60bt"), "mV");
        assert_eq!(unit("ut202bt"), "V");
    }

    /// A measurement frame says which family answered, not which model: the
    /// CH9329 does not purge its RX buffer on open, so it may be left over
    /// from an earlier session.
    #[test]
    fn a_measurement_frame_settles_the_family_only() {
        let reading = test_frame_be16(&[
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, 0x00, 0x02, 0x30, 0x30, 0x30,
        ]);
        assert_eq!(
            recognised(&reading),
            Some(Evidence::FamilyOnly {
                fallback: &devices::UT61EPLUS,
            })
        );
    }

    /// The ack says something is listening, but nothing about what.
    #[test]
    fn the_ack_identifies_nothing() {
        assert!(is_ack(&[0xFF, 0x00]));
        assert_eq!(recognised(&ACK), None);
    }

    // --- Unrecognised data (protocol::unrecognised) -----------------------

    use crate::protocol::capture_reports;
    use tables::ut61b_plus::UT61B_PLUS;
    use tables::ut61d_plus::UT61D_PLUS;
    use tables::{BASE, RangeInfo};

    /// Parse `payload` with `table`, keeping what it reported.
    fn parse_reporting(payload: &[u8], table: &ModelTable) -> (Result<Measurement>, Vec<String>) {
        capture_reports(|| parse_measurement(payload, table))
    }

    /// Frames our UT61E+ sent (captures 2026-09-07): a reading in every mode
    /// it reached, and the flags, bar and overload forms those runs saw.
    const E_PLUS_FRAMES: &[[u8; 14]] = &[
        [
            0x00, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x32, 0x38, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x01, 0x30, 0x20, 0x20, 0x20, 0x34, 0x2E, 0x39, 0x39, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x33, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x03, 0x30, 0x2D, 0x20, 0x32, 0x37, 0x2E, 0x30, 0x32, 0x00, 0x04, 0x30, 0x34, 0x31,
        ],
        [
            0x04, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x05, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x06, 0x30, 0x20, 0x20, 0x4F, 0x4C, 0x2E, 0x20, 0x20, 0x04, 0x04, 0x30, 0x30, 0x30,
        ],
        [
            0x07, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x34, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x08, 0x30, 0x20, 0x31, 0x2E, 0x39, 0x30, 0x30, 0x33, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x09, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x34, 0x32, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0C, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0D, 0x30, 0x20, 0x20, 0x20, 0x33, 0x2E, 0x33, 0x34, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0E, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0F, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x33, 0x31, 0x34, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x10, 0x31, 0x20, 0x20, 0x30, 0x2E, 0x31, 0x34, 0x32, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x11, 0x31, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x14, 0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x2D, 0x20, 0x04, 0x02, 0x30, 0x34, 0x30,
        ],
        [
            0x14, 0x30, 0x20, 0x20, 0x20, 0x45, 0x46, 0x20, 0x20, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        // LPF V with the HV warning lit.
        [
            0x18, 0x33, 0x20, 0x20, 0x31, 0x36, 0x35, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x35, 0x30,
        ],
        [
            0x19, 0x30, 0x2D, 0x30, 0x2E, 0x30, 0x31, 0x33, 0x33, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        // AC+DC V's AC component (flag3 bit 3), AC V with P-MAX, DC V under
        // MAX, REL and HOLD.
        [
            0x19, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x33, 0x36, 0x37, 0x00, 0x00, 0x30, 0x30, 0x38,
        ],
        [
            0x00, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x31, 0x33, 0x36, 0x00, 0x00, 0x30, 0x34, 0x34,
        ],
        [
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x38, 0x34, 0x31,
        ],
        [
            0x02, 0x30, 0x2D, 0x30, 0x2E, 0x30, 0x30, 0x34, 0x36, 0x00, 0x01, 0x31, 0x34, 0x30,
        ],
        [
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x32, 0x30, 0x31,
        ],
    ];

    /// Frames a UT61B+ sent (issue #19 and #20 captures): a reading in every
    /// mode it reached, and the flags, bar and overload forms seen there.
    const B_PLUS_FRAMES: &[[u8; 14]] = &[
        [
            0x00, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x34, 0x32, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x01, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x02, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x03, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x04, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x05, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x06, 0x31, 0x20, 0x20, 0x2E, 0x4F, 0x4C, 0x20, 0x20, 0x03, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x07, 0x30, 0x20, 0x20, 0x20, 0x4F, 0x4C, 0x2E, 0x20, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x08, 0x30, 0x20, 0x20, 0x2E, 0x4F, 0x4C, 0x20, 0x20, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        [
            0x09, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0C, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0D, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0E, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x0F, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x10, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x11, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x14, 0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x2D, 0x20, 0x00, 0x00, 0x30, 0x34, 0x30,
        ],
        // AC mV overloaded, bar 30; AC V on mains, HV lit; DC V negative;
        // AC V under MAX.
        [
            0x01, 0x31, 0x20, 0x20, 0x20, 0x4F, 0x4C, 0x2E, 0x20, 0x03, 0x00, 0x30, 0x30, 0x30,
        ],
        [
            0x00, 0x32, 0x20, 0x20, 0x32, 0x33, 0x36, 0x2E, 0x36, 0x01, 0x01, 0x30, 0x31, 0x30,
        ],
        [
            0x02, 0x30, 0x20, 0x2D, 0x30, 0x2E, 0x30, 0x30, 0x31, 0x00, 0x00, 0x30, 0x30, 0x31,
        ],
        [
            0x00, 0x31, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x38, 0x34, 0x30,
        ],
    ];

    #[test]
    fn captured_frames_report_nothing() {
        for (model, table, frames) in [
            ("UT61E+", &UT61E_PLUS, E_PLUS_FRAMES),
            ("UT61B+", &UT61B_PLUS, B_PLUS_FRAMES),
        ] {
            for frame in frames {
                let (m, reports) = parse_reporting(frame, table);
                assert!(m.is_ok(), "{model} {frame:02X?}: {m:?}");
                assert!(reports.is_empty(), "{model} {frame:02X?}: {reports:?}");
            }
        }
    }

    #[test]
    fn an_unknown_mode_byte_is_reported_and_reads_unnamed() {
        let payload = make_payload(0x1F, 0x00, b"  1.234", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        let m = m.unwrap();
        assert_eq!(
            (
                m.mode.as_ref(),
                m.mode_raw,
                m.unit.as_ref(),
                m.range_label.as_ref()
            ),
            ("Unknown(0x1f)", 0x1F, "", "")
        );
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 1.234));
        assert_eq!(reports, ["ut61eplus: unrecognised mode byte: 0x1f"]);
    }

    /// Whichever mode it is, the display can say OL.
    #[test]
    fn an_unknown_mode_byte_reads_its_overload() {
        let payload = make_payload(0x1F, 0x00, b"   OL  ", (0, 0), (0, 0, 0));
        let (m, _) = parse_reporting(&payload, &UT61E_PLUS);
        let m = m.unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.unit, "");
    }

    /// Bit 7 marks a secondary-display frame whether or not we know its
    /// function, so the frame is refused either way.
    #[test]
    fn bit_7_over_an_unknown_function_is_refused() {
        let payload = make_payload(0x9F, 0x00, b"  0.000", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert!(matches!(m, Err(Error::InvalidResponse { .. })), "{m:?}");
        assert_eq!(reports, ["ut61eplus: unrecognised mode byte: 0x9f"]);
    }

    /// A replay reaches the parser with no secondary-display handling, and
    /// a frame with bit 7 is no reading there either.
    #[test]
    fn a_replayed_secondary_frame_is_refused() {
        let p = ut202bt();
        let frame = secondary(0x04, 0x00, b"  50.02");
        let (m, _) = capture_reports(|| p.parse_payload(&frame));
        assert!(matches!(m, Err(Error::InvalidResponse { .. })), "{m:?}");
    }

    /// Any other long frame is no reading, a Get Name reply among them, and
    /// its first byte would read as a mode no table lists.
    #[test]
    fn a_long_frame_that_is_no_reading_is_refused() {
        let mut no_prefixes = make_payload(0x1F, 0x00, b"  1.234", (0, 0), (0, 0, 0));
        no_prefixes[11] = 0x00;
        for frame in [
            &b"UNI-T UT61E+ 1234"[..],
            &b"UNI-T UT61E+ X"[..],
            &no_prefixes,
        ] {
            let (m, reports) = parse_reporting(frame, &UT61E_PLUS);
            assert!(
                matches!(m, Err(Error::InvalidResponse { .. })),
                "{frame:?}: {m:?}"
            );
            assert!(
                reports.iter().any(|r| r.contains("unrecognised mode byte")),
                "{reports:?}"
            );
        }
    }

    /// An unnamed mode keeps the checks that don't rest on one.
    #[test]
    fn an_unknown_mode_still_reports_a_bad_bar_graph() {
        let payload = make_payload(0x1F, 0x00, b"  1.234", (9, 9), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert_eq!(m.unwrap().mode, "Unknown(0x1f)");
        assert_eq!(
            reports,
            [
                "ut61eplus: unrecognised mode byte: 0x1f",
                "ut61eplus: unrecognised bar graph: [09, 09]",
            ]
        );
    }

    /// Through the driver: a reading, with nothing offered that needs a mode.
    #[test]
    fn the_driver_reads_an_unknown_mode() {
        let mock = MockTransport::new(vec![frame_in(0x1F)]);
        let mut p = Ut61PlusProtocol::for_model("ut61eplus");
        let m = p.request_measurement(&mock).unwrap();
        assert_eq!((m.mode.as_ref(), m.mode_raw), ("Unknown(0x1f)", 0x1F));
        assert!(p.range_ladder(0x1F).is_empty());
        assert_eq!(p.mode_label(0x1F), "Unknown(0x1f)");
    }

    /// A value held from a secondary frame stays with the next reading,
    /// whatever its mode.
    #[test]
    fn an_unknown_mode_carries_the_held_secondary() {
        let mut p = ut202bt();
        assert!(p.take_reading(&secondary(0x04, 0x00, b"  50.02")).is_none());
        let reading = p.take_reading(&main_frame(0x1F, 0x00, b"   1.23"));
        let m = reading.unwrap().unwrap();
        assert_eq!(m.mode, "Unknown(0x1f)");
        assert_eq!(m.aux_summary(), "Frequency 50.02 Hz");
    }

    /// Temperature is a UT61D+ position (family spec §3.1), not a UT61E+ one.
    #[test]
    fn a_mode_off_the_model_s_dial_is_reported() {
        let payload = make_payload(0x0A, 0x00, b"   23.4", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert_eq!(m.unwrap().mode, "°C");
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised mode byte: 0x0a (°C), not on the UNI-T UT61E+ dial"]
        );

        let (_, reports) = parse_reporting(&payload, &UT61D_PLUS);
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A table that describes no dial, with a rung for every mode.
    static NO_DIAL: ModelTable = ModelTable {
        name: "no dial",
        entry: |_| Some(&ANY_RUNG),
        ..BASE
    };

    static ANY_RUNG: [RangeInfo; 1] = [tables::r("any", "V")];

    #[test]
    fn a_table_without_a_dial_takes_any_mode() {
        let payload = make_payload(0x13, 0x00, b"  0.000", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &NO_DIAL);
        assert_eq!(m.unwrap().mode, "Live");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// DC V has four rungs on the UT61E+; NCV has no table at all and says
    /// nothing about it.
    #[test]
    fn a_range_byte_past_the_table_is_reported() {
        let table = &UT61E_PLUS;
        let payload = make_payload(0x02, 0x04, b" 12.345", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, table);
        assert_eq!(m.unwrap().range_label, "");
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised range byte: mode 0x02 range 4"]
        );

        let payload = make_payload(0x14, 0x00, b"   EF  ", (0, 0), (0, 0, 0));
        let (_, reports) = parse_reporting(&payload, table);
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// §2.7: flag2 bit 3 is the protocol deck's APO flag. No capture has
    /// set it, but it is documented, so it passes without a report.
    #[test]
    fn the_apo_flag_bit_is_not_reported() {
        let payload = make_payload(0x02, 0x01, b" 12.345", (0, 0), (0, 0x08, 0));
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert!(m.unwrap().flags.auto_range);
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn a_byte_without_its_prefix_is_reported() {
        let mut payload = make_payload(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        payload[1] = 0x01;
        payload[12] = 0x00;
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert_eq!(m.unwrap().range_label, "22V");
        assert_eq!(
            reports,
            [
                "ut61eplus: unrecognised byte prefix: payload[1] = 0x01",
                "ut61eplus: unrecognised byte prefix: payload[12] = 0x00",
            ]
        );
    }

    /// The UT61B+ has no Peak (family spec §4); the UT61E+ does, and so does
    /// the UT202BT in AC A (its manual, P11/20).
    #[test]
    fn peak_bits_on_a_model_without_peak_are_reported() {
        let payload = make_payload(0x00, 0x00, b"  0.037", (0, 0), (0, 0, F_PEAK_MAX));
        let (m, reports) = parse_reporting(&payload, &UT61B_PLUS);
        assert!(m.unwrap().flags.peak_max);
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised flag bits: [30, 30, 34], Peak on a model without it"]
        );

        let (_, reports) = parse_reporting(&payload, &UT61E_PLUS);
        assert!(reports.is_empty(), "{reports:?}");

        let ut202bt = &tables::ut202bt::UT202BT;
        for mode in [0x11, 0x16] {
            let payload = make_payload(mode, 0x02, b"  123  ", (0, 0), (0, 0, F_PEAK_MIN));
            let (m, reports) = parse_reporting(&payload, ut202bt);
            assert!(m.unwrap().flags.peak_min, "{mode:#04x}");
            assert!(reports.is_empty(), "{mode:#04x}: {reports:?}");
        }
    }

    #[test]
    fn a_bar_graph_past_46_segments_is_reported() {
        let table = &UT61E_PLUS;
        for (bar, progress, report) in [
            ((4, 7), 47, Some("[04, 07]")),
            ((0, 10), 10, Some("[00, 0A]")),
            ((4, 6), 46, None),
        ] {
            let payload = make_payload(0x02, 0x01, b" 12.345", bar, (0, 0, 0));
            let (m, reports) = parse_reporting(&payload, table);
            assert_eq!(m.unwrap().progress, Some(progress));
            let want: Vec<String> = report
                .map(|r| format!("ut61eplus: unrecognised bar graph: {r}"))
                .into_iter()
                .collect();
            assert_eq!(reports, want, "bar {bar:?}");
        }
    }

    #[test]
    fn a_payload_past_14_bytes_is_reported() {
        let mut payload = make_payload(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        payload.push(0x00);
        let (m, reports) = parse_reporting(&payload, &UT61E_PLUS);
        let value = m.unwrap().value;
        assert!(
            matches!(value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-9),
            "{value:?}"
        );
        assert_eq!(
            reports,
            [format!(
                "ut61eplus: unrecognised frame: 15 bytes: {:02X?}",
                payload
            )]
        );
    }

    /// The ack and a name reply are skipped quietly on the way to a reading;
    /// anything else is reported.
    #[test]
    fn an_unknown_frame_before_a_reading_is_reported() {
        let mut proto = Ut61PlusProtocol::new();
        let mock = MockTransport::new(vec![
            ACK.to_vec(),
            NAME_UT61EPLUS.to_vec(),
            test_frame_be16(&[0xFF, 0x01]),
            frame_in(0x02),
        ]);
        let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().mode, "DC V");
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised frame: 2 bytes: [FF, 01]"]
        );
    }

    #[test]
    fn get_name_returns_only_a_real_reply() {
        let mut proto = Ut61PlusProtocol::new();
        let mut name = |frames: Vec<Vec<u8>>| {
            let mock = MockTransport::new(frames);
            capture_reports(|| proto.get_name(&mock).unwrap())
        };

        let (got, reports) = name(vec![ACK.to_vec(), NAME_UT61EPLUS.to_vec()]);
        assert_eq!(got.as_deref(), Some("UT61E+"));
        assert!(reports.is_empty(), "{reports:?}");

        // A stale reading from an earlier session on a CH9329, ahead of the
        // reply: skipped without a report, and the name still arrives.
        let (got, reports) = name(vec![frame_in(0x02), ACK.to_vec(), NAME_UT61EPLUS.to_vec()]);
        assert_eq!(got.as_deref(), Some("UT61E+"));
        assert!(reports.is_empty(), "{reports:?}");

        // A frame that is no reply is reported — and never taken for a name,
        // which would put its bytes in the GUI header and `dmm-cli info`.
        let (got, reports) = name(vec![
            test_frame_be16(&[0x01, 0x02, 0x03]),
            ACK.to_vec(),
            ACK.to_vec(),
        ]);
        assert_eq!(got, None);
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised frame: 3 bytes: [01, 02, 03]"]
        );
    }

    #[test]
    fn an_unknown_model_name_is_reported() {
        let (evidence, reports) = capture_reports(|| recognised(&test_frame_be16(b"UT216XD")));
        assert!(
            matches!(
                evidence,
                Some(Evidence::Model { device, .. }) if device == FALLBACK
            ),
            "{evidence:?}"
        );
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised model name: \"UT216XD\", using the UT61E+ tables"]
        );

        let (_, reports) = capture_reports(|| recognised(&NAME_UT61BPLUS));
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn the_golden_readings_labels_are_listed() {
        let all = crate::text::listed();
        for device in ["ut61eplus", "ut161e", "ut61b+"] {
            crate::text::assert_golden_listed(all, device);
        }
    }

    #[test]
    fn every_models_dial_and_secondary_labels_are_listed() {
        use crate::text::TextKind;
        let all = crate::text::listed();
        // A frame in every mode each model's table names, at its first
        // range, as a main and as a secondary display.
        for model in ["ut61eplus", "ut61b+", "ut61d+", "ut60bt", "ut202bt"] {
            let protocol = Ut61PlusProtocol::for_model(model);
            let by = protocol.profile.id;
            for &mode in Mode::ALL {
                let Some(range) =
                    (0..=0x0F).find(|&r| protocol.table.range_info(mode, r).is_some())
                else {
                    continue;
                };
                let frame = main_frame(mode as u8, 0x30 | range, b"  0.000");
                let m = super::parse_measurement(by, &frame, protocol.table).unwrap();
                crate::text::assert_listed(all, &protocol, &m, by);
                if protocol.table.secondary_display {
                    let label = secondary_label(mode);
                    assert!(all.contains(TextKind::Aux, label), "{model} {label}");
                }
            }
        }
    }
}
