pub mod command;
pub mod mode;
pub(crate) mod specs;
pub mod tables;

use crate::error::{Error, ErrorKind, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN};
use crate::protocol::registry;
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    Choice, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Setting,
    Stability, check_len, cycle, unknown_mode, unknown_mode16, unsupported_setting,
};
use crate::transport::Transport;
use command::Command;
use log::{debug, warn};
use mode::Mode;
use specs::SpecModel;
use std::borrow::Cow;
use std::time::{Duration, Instant};
use tables::DeviceTable;

const UT61EPLUS_COMMANDS: &[&str] = &[
    "hold",
    "minmax",
    "exit_minmax",
    "range",
    "auto",
    "rel",
    "select2",
    "select",
    "light",
    "peak",
    "exit_peak",
];

/// Protocol implementation for the UT61E+/UT61B+/UT61D+/UT161 family.
pub struct Ut61PlusProtocol {
    table: Box<dyn DeviceTable>,
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
}

impl Default for Ut61PlusProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut61PlusProtocol {
    /// A UT61E+.
    pub fn new() -> Self {
        Self::with_profile(
            Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
            "UNI-T UT61E+",
            true,
            SpecModel::Ut61ePlus,
        )
    }

    /// Create a protocol instance for a specific model name.
    ///
    /// Recognized model strings (case-insensitive):
    /// - "ut61e+" (Verified), "ut161e" -> UT61E+ table
    /// - "ut61b+" (Verified), "ut161b" -> UT61B+ table
    /// - "ut61d+", "ut161d" -> UT61D+ table
    ///
    /// Several models share a table — the UT161x meters are believed to speak
    /// the same protocol as their UT61x+ counterparts — so the reported model
    /// name and stability come from the requested model, not from the table.
    /// Otherwise a UT161E would introduce itself as a verified UT61E+.
    ///
    /// Returns `None` if the model string is not recognized.
    pub fn for_model(model: &str) -> Option<Self> {
        // (table, reported model name, verified against real hardware,
        // manual spec tables)
        let (table, model_name, verified, specs): (Box<dyn DeviceTable>, _, _, _) =
            match model.to_lowercase().as_str() {
                "ut61e+" => (
                    Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
                    "UNI-T UT61E+",
                    true,
                    SpecModel::Ut61ePlus,
                ),
                "ut161e" => (
                    Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
                    "UNI-T UT161E",
                    false,
                    SpecModel::Ut161e,
                ),
                // Verified by three captures reported in issue #19,
                // 2026-09-09 to 2026-09-11: every mode its dial reaches
                // decoded correctly, every command moved the flag it should,
                // the second run passed the gate outright and the third
                // walked the Ω and DC V ladders. What is left open on this
                // model is in `docs/verification-backlog.md`, "UT61B+ —
                // hardware reports".
                "ut61b+" => (
                    Box::new(tables::ut61b_plus::Ut61bPlusTable::new()),
                    "UNI-T UT61B+",
                    true,
                    SpecModel::Ut61bPlus,
                ),
                "ut161b" => (
                    Box::new(tables::ut61b_plus::Ut61bPlusTable::new()),
                    "UNI-T UT161B",
                    false,
                    SpecModel::Ut161b,
                ),
                "ut61d+" => (
                    Box::new(tables::ut61d_plus::Ut61dPlusTable::new()),
                    "UNI-T UT61D+",
                    false,
                    SpecModel::Ut61dPlus,
                ),
                "ut161d" => (
                    Box::new(tables::ut61d_plus::Ut61dPlusTable::new()),
                    "UNI-T UT161D",
                    false,
                    SpecModel::Ut161d,
                ),
                _ => return None,
            };
        Some(Self::with_profile(table, model_name, verified, specs))
    }

    fn with_profile(
        table: Box<dyn DeviceTable>,
        model_name: &'static str,
        verified: bool,
        specs: SpecModel,
    ) -> Self {
        // A model no meter has answered for is RE of the vendor software plus
        // manual specs, so it reports as experimental.
        let stability = if verified {
            Stability::Verified
        } else {
            Stability::Experimental
        };
        // The family issue stays on every model but the UT61E+, verified or
        // not: the UT61B+ is decoded correctly everywhere it was looked at,
        // and its range rungs above the ones auto-ranging reached are still
        // open there.
        let verification_issue = (model_name != "UNI-T UT61E+").then_some(7);
        Self {
            table,
            specs,
            rx_buf: Vec::with_capacity(64),
            dial: cycle::DialState::default(),
            streaming: false,
            profile: DeviceProfile {
                family_name: "UT61+/UT161",
                model_name,
                stability,
                supported_commands: UT61EPLUS_COMMANDS,
                max_aux_values: 0,
                verification_issue,
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
            &framing::HEADER,
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
                return parse_measurement(&payload, self.table.as_ref());
            }
            debug!(
                "skipping non-measurement frame ({} bytes): {:02X?}",
                payload.len(),
                payload
            );
            report_unknown_frame(&payload);
        }
        Err(Error::Timeout)
    }

    /// The newest reading the adapter has already delivered, `first` being
    /// the oldest.
    ///
    /// The adapter streams about three readings a second whoever is reading
    /// (adapter spec §3), so a caller that reads slower — a one-second
    /// interval, or a pause — would otherwise be handed ever-older readings
    /// stamped with the time they were read. Everything already queued is
    /// taken off with zero-wait reads, and the newest checksummed reading
    /// frame decides the answer, parse error included: that is what the
    /// meter shows now. A frame that fails its checksum is dropped.
    /// "Already queued" is anything that arrives within [`DRAIN_WAIT_MS`] of
    /// the last frame taken off.
    fn newest_streamed(
        &mut self,
        transport: &dyn Transport,
        first: Result<Measurement>,
    ) -> Result<Measurement> {
        // A dead link or a silent adapter is the answer; only a frame, good
        // or not, can be superseded by a newer one.
        if let Err(e) = &first
            && e.kind() != ErrorKind::Protocol
        {
            return first;
        }
        let mut newest = first;
        let mut tmp = [0u8; 64];
        for _ in 0..MAX_DRAIN_READS {
            loop {
                match framing::extract_frame_abcd_be16(&self.rx_buf) {
                    Ok(Some((payload, consumed))) => {
                        self.rx_buf.drain(..consumed);
                        if payload.len() >= UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
                            newest = parse_measurement(&payload, self.table.as_ref());
                        } else {
                            report_unknown_frame(&payload);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        debug!("dropping a corrupt queued frame: {e}");
                        self.rx_buf.clear();
                        break;
                    }
                }
            }
            // Only partial frames are left here, but a stream that never
            // frames must not grow the buffer either.
            if self.rx_buf.len() > MAX_DRAIN_BUF {
                self.rx_buf.clear();
            }
            let n = transport.read_timeout(&mut tmp, DRAIN_WAIT_MS)?;
            if n == 0 {
                break;
            }
            self.rx_buf.extend_from_slice(&tmp[..n]);
        }
        newest
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
            "hold" => Ok(Command::Hold),
            "minmax" => Ok(Command::MinMax),
            "exit_minmax" => Ok(Command::ExitMinMax),
            "range" => Ok(Command::Range),
            "auto" => Ok(Command::Auto),
            "rel" => Ok(Command::Rel),
            "select2" => Ok(Command::Select2),
            "select" => Ok(Command::Select),
            "light" => Ok(Command::Light),
            "peak" => Ok(Command::PeakMinMax),
            "exit_peak" => Ok(Command::ExitPeak),
            _ => Err(Error::UnsupportedCommand(name.to_string())),
        }
    }
}

impl Protocol for Ut61PlusProtocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // A cable needs nothing: the CP2110 is set up by `Cp2110::init_uart()`
        // before the protocol exists, and the meter only acknowledges 0x5D.
        // The UT-D07B polls the meter itself once told to, at about three
        // readings a second against under two when each one is a round trip
        // over the radio (adapter spec §3, §5).
        if transport.transport_name() == crate::BLUETOOTH {
            debug!("starting the adapter's readings stream");
            transport.write(&Command::StartStream.encode())?;
            self.streaming = true;
        }
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let m = if self.streaming {
            let first = match self.read_measurement(transport) {
                // A start command can be lost on the adapter, so a silent
                // stream is started again — and polled, because an adapter
                // that ignores 0x5D (none seen yet) still answers a poll.
                Err(Error::Timeout) => {
                    debug!("no streamed reading: restarting the stream and polling");
                    transport.write(&Command::StartStream.encode())?;
                    self.poll_measurement(transport)
                }
                other => other,
            };
            self.newest_streamed(transport, first)?
        } else {
            self.poll_measurement(transport)?
        };
        // The stream is the only place the meter states its mode, so every
        // reading is what keeps the dial position current.
        self.dial.observe(self.table.dial_positions(), m.mode_raw);
        Ok(m)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(payload, self.table.as_ref())
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
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
            report_unknown_frame(&payload);
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
        use crate::flags::Flag;
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{CaptureStep, Expect, Need, RangeExpect, ValueExpect};

        // The list is shared by the whole UT61+/UT161 family; the UT61E+ and
        // the UT61B+ have run every step of it (docs/verification-backlog.md).
        // A step a verified model has not run must not ride on this flag —
        // the two ladder steps were held back that way for the B+ until it
        // walked them on 2026-09-11.
        let hw = self.profile.stability == Stability::Verified;
        // Every step on this meter takes three samples and shares that
        // hardware history, the gate steps included.
        let mark = |s: CaptureStep| s.samples(3).verified_if(hw);
        let [dcv, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
            Volts::DcV,
            CaptureStep::basic("dcv", "Set meter to DC V (V\u{23CF}). Leave leads open."),
            Ohms::Symbol,
            CaptureStep::basic(
                "ohm",
                "Set meter to \u{03A9}. Leave leads open (should show OL).",
            ),
        )
        .map(mark);

        let mut steps = vec![
            // The six gate steps first, both trios, so the gate is decided by
            // step six and every step after it can be driven. Split across the
            // list, as they used to be, the run reached AC V, DC mV and AC mV
            // while still ungated and swept none of them (issue #19).
            dcv,
            dcv_short,
            dcv_negative,
            ohm,
            ohm_body,
            ohm_short,
            // Gate decided. A gate step is never swept — the step after it
            // assumes the state it left — so each ladder gets a plain step of
            // its own, in the mode the run is already in.
            CaptureStep::basic(
                "ohm_ranges",
                "Set meter to \u{03A9}. Leads open or shorted, either will do.",
            )
            .samples(3)
            .verified_if(hw)
            .expect(Expect::mode("\u{03A9}")),
            // The rest of the resistance dial position, while the leads are
            // still there.
            CaptureStep::basic(
                "continuity",
                "Set meter to continuity (buzzer). Touch probes together.",
            )
            .samples(3)
            .verified_if(hw)
            .needs(&[Need::ShortedLeads])
            .expect(Expect::mode("Continuity").value(ValueExpect::Finite)),
            // Open leads read OL, and the meter refuses REL over OL in any
            // mode, so the old wording ("leave leads open") guaranteed that
            // this step's REL and MIN/MAX sub-steps said nothing about diode
            // — which is exactly what happened until 2026-09-10. Both are
            // now known dead here, so the step no longer sweeps them, but a
            // finite reading is still what verifies the decode.
            CaptureStep::basic(
                "diode",
                "Set meter to diode. A diode across the probes if you have one, \
                 otherwise leave the leads open (OL).",
            )
            .samples(3)
            .verified_if(hw)
            .expect(Expect::mode("Diode")),
            CaptureStep::basic("capacitance", "Set meter to capacitance. Leave leads open.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("Capacitance")),
            // Back to volts for its ladder and for the button steps below,
            // which have always assumed the dial is here.
            CaptureStep::basic(
                "dcv_ranges",
                "Set meter to DC V (V\u{23CF}). Leave leads open.",
            )
            .samples(3)
            .verified_if(hw)
            .expect(Expect::mode("DC V")),
            // Flags & commands. These run wherever the dial is, so they sit
            // on DC V: at the end of the list the dial was on DC A, a single
            // range where RANGE and AUTO have nothing to do (`auto did
            // nothing` on hardware, 2026-09-07); DC V has four rungs.
            CaptureStep::with_command(
                "hold",
                "DC V mode: press HOLD on the meter, or we will send the command.",
                "hold",
                3,
            )
            .verified_if(hw)
            .expect(Expect::new().flags(&[(Flag::Hold, true)])),
            CaptureStep::with_command("hold_off", "Press HOLD again to turn it off.", "hold", 3)
                .verified_if(hw)
                .expect(Expect::new().flags(&[(Flag::Hold, false)])),
            CaptureStep::with_command("rel", "DC V mode: we will send REL.", "rel", 3)
                .verified_if(hw)
                .expect(Expect::new().flags(&[(Flag::Rel, true)])),
            CaptureStep::with_command(
                "rel_off",
                "We will send REL again to turn it off.",
                "rel",
                3,
            )
            .verified_if(hw)
            .expect(Expect::new().flags(&[(Flag::Rel, false)])),
            // Which of MIN and MAX the first press lands on is the meter's
            // own cycle, so only the exit is asserted.
            CaptureStep::with_command("minmax", "We will send MIN/MAX.", "minmax", 3)
                .verified_if(hw),
            CaptureStep::with_command("minmax_off", "We will exit MIN/MAX.", "exit_minmax", 3)
                .verified_if(hw)
                .expect(Expect::new().flags(&[(Flag::Min, false), (Flag::Max, false)])),
            // A single RANGE press, not a sweep. A six-step sweep was tried
            // and removed: on hardware it produced range indices 0, 2, 0, 0,
            // 0, 0 — never visiting 22V or 1000V — and flipped the mode byte
            // between DC V (0x02) and AC+DC V (0x19) partway through, which
            // is the documented effect of SELECT (0x4C), not RANGE (0x46).
            // Until what 0x46 actually does is known, stepping it repeatedly
            // just files misleading data. See the UT61E+ section of
            // docs/verification-backlog.md.
            CaptureStep::with_command(
                "range",
                "We will send RANGE to switch to manual.",
                "range",
                3,
            )
            .verified_if(hw)
            .expect(Expect::new().range(RangeExpect::Manual)),
            CaptureStep::with_command(
                "auto",
                "We will send AUTO to return to auto-range.",
                "auto",
                3,
            )
            .verified_if(hw)
            .expect(Expect::new().range(RangeExpect::Auto)),
            // The rest of each dial position's SELECT ring (spec §3.1): one
            // step per mode the ring reaches, in dial order.
            CaptureStep::basic(
                "acdcv",
                "Set meter to V\u{23CF} and press SELECT for AC+DC V.",
            )
            .samples(3)
            .verified_if(hw)
            .expect(Expect::mode("AC+DC V")),
            CaptureStep::basic("acv", "Set meter to AC V (V~). Leave leads open.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("AC V")),
            CaptureStep::basic("lpfv", "Set meter to V~ and press SELECT for LPF V.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("LPF V")),
            CaptureStep::basic("dcmv", "Set meter to DC mV. Leave leads open.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("DC mV")),
            CaptureStep::basic("acmv", "Set meter to mV and press SELECT for AC mV.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("AC mV")),
            CaptureStep::basic("hz", "Set meter to the Hz/% dial position.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("Hz")),
            CaptureStep::basic(
                "duty",
                "Hz/% position: short-press the USB button for Duty %.",
            )
            .samples(3)
            .verified_if(hw)
            .expect(Expect::mode("Duty %")),
            CaptureStep::basic("ncv", "Set meter to NCV. Hold near a live wire.")
                .samples(3)
                .verified_if(hw)
                .needs(&[Need::LiveWire])
                .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
            CaptureStep::basic("hfe", "Set meter to hFE (transistor test).")
                .samples(3)
                .verified_if(hw)
                .needs(&[Need::Transistor])
                .expect(Expect::mode("hFE")),
            CaptureStep::basic("dcua", "Set meter to DC µA.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("DC µA")),
            CaptureStep::basic("acua", "Set meter to µA and press SELECT for AC µA.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("AC µA")),
            CaptureStep::basic("dcma", "Set meter to DC mA.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("DC mA")),
            CaptureStep::basic("acma", "Set meter to mA and press SELECT for AC mA.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("AC mA")),
            CaptureStep::basic("dca", "Set meter to DC A (A\u{23CF}).")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("DC A")),
            CaptureStep::basic("aca", "Set meter to A and press SELECT for AC A.")
                .samples(3)
                .verified_if(hw)
                .expect(Expect::mode("AC A")),
            // Temperature needs a thermocouple, so it has never been run.
            CaptureStep::basic("temp", "Set meter to temperature (K-type thermocouple).")
                .samples(3)
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("\u{00B0}C")),
            CaptureStep::basic("tempf", "Temperature position: press SELECT for \u{00B0}F.")
                .samples(3)
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("\u{00B0}F")),
            CaptureStep::basic("loz", "Set meter to LoZ (low-impedance volts).")
                .samples(3)
                .expect(Expect::mode("LoZ V")),
        ];
        // The list names every mode in the family; a model's dial table says
        // which it reaches (spec §2.1: temperature and LoZ are UT61D+/UT161D
        // positions), so the others are not asked for.
        let dial = self.table.dial_positions();
        steps.retain(|step| {
            let Some(label) = step.expect.and_then(|e| e.mode) else {
                return true;
            };
            dial.iter().flat_map(|p| p.modes()).any(|m| {
                u8::try_from(m)
                    .ok()
                    .and_then(|b| Mode::from_byte(b).ok())
                    .is_some_and(|mode| mode.as_static_str() == label)
            })
        });
        steps
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
const ACK_FRAME: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];

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
fn report_unknown_frame(payload: &[u8]) {
    if payload.len() != UT61EPLUS_MEASUREMENT_PAYLOAD_LEN
        && !is_ack(payload)
        && name_from_reply(payload).is_none()
    {
        report_unknown(
            "ut61eplus",
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
fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    let mut evidence = None;
    for start in framing::abcd_header_offsets(buf) {
        let Ok(Some((payload, _))) = framing::extract_frame_abcd_be16(&buf[start..]) else {
            continue;
        };
        if is_ack(&payload) {
            continue;
        }
        if let Some(name) = name_from_reply(&payload) {
            return Some(match registry::device_for_reported_name(&name) {
                Some(device) => {
                    debug!("detect: name frame {name:?} resolves to {}", device.id);
                    Evidence::Model {
                        id: device.id,
                        reported_name: Some(name),
                    }
                }
                None => {
                    report_unknown(
                        "ut61eplus",
                        "model name",
                        format_args!("{name:?}, using the UT61E+ tables"),
                    );
                    Evidence::Model {
                        id: FALLBACK_ID,
                        reported_name: Some(name),
                    }
                }
            });
        }
        if payload.len() == UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
            evidence = Some(Evidence::FamilyOnly {
                fallback: FALLBACK_ID,
            });
        }
    }
    evidence
}

/// The entry a frame that names no model opens: the tables every meter in the
/// family reads with, even where a sibling's ranges differ.
const FALLBACK_ID: &str = "ut61eplus";

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
/// a fresh link (adapter spec §5).
const BLUETOOTH_PRESS_ACK_TIMEOUT: Duration = Duration::from_millis(2500);

/// Most zero-wait reads one streamed request takes off the queue: one
/// notification each, so about twenty minutes of readings at the adapter's
/// three a second (adapter spec §3). A longer backlog — a pause left on for
/// an hour — drains over the next few requests.
const MAX_DRAIN_READS: usize = 4096;

/// How long each of those reads waits. Not zero: the Bluetooth transport's
/// runtime only moves notifications from the platform's socket onto its
/// queue while a read is waiting, and a zero wait gives it no turn to.
const DRAIN_WAIT_MS: i32 = 10;

/// Most bytes of an unfinished frame kept between those reads; a frame is
/// 19 bytes.
const MAX_DRAIN_BUF: usize = 256;

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
/// [VERIFIED] on a UT61E+ (`ut61eplus-verify4.yaml`, step `ncv/hold:on`) and a
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
        self.table.dial_positions()
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
                Ok(m) => Cow::Borrowed(m.as_static_str()),
                Err(_) => unknown_mode(byte),
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
        match u8::try_from(mode).map(Mode::from_byte) {
            Ok(Ok(mode)) => tables::range_ladder(self.table.as_ref(), mode),
            // A mode byte this family's parser cannot name has no table.
            _ => Vec::new(),
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
    fn flag_states(&self, setting: cycle::FlagSetting, mode: u16) -> &'static [u16] {
        let named = u8::try_from(mode).map(Mode::from_byte);
        let dead = |modes: &[Mode]| matches!(named, Ok(Ok(m)) if modes.contains(&m));
        match setting {
            cycle::FlagSetting::Hold if dead(HOLD_DEAD) => &[],
            cycle::FlagSetting::Rel if dead(REL_DEAD) => &[],
            cycle::FlagSetting::MinMax if dead(MINMAX_DEAD) => &[],
            cycle::FlagSetting::Hold | cycle::FlagSetting::Rel => &[0, 1],
            // MAX then MIN, the order the meter's own 2-state ring cycles in.
            // No AVG: the UT61E+ reports none over USB.
            cycle::FlagSetting::MinMax => &[0, 1, 2],
            cycle::FlagSetting::Peak => match named {
                Ok(Ok(mode)) if self.table.peak_modes().contains(&mode) => &[0, 1, 2],
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

/// Whether the display spells overload.
///
/// The 7-char field is a segment dump: the meter lights the decimal point
/// belonging to the rung it is on and puts `O` and `L` in the digit slots
/// either side of it, so the point lands anywhere among the letters. All
/// three forms are on record, on both meters and across ranges — ` .OL   `
/// (E+ 2.2MΩ and diode, B+ diode), `  O.L  ` (E+ 22kΩ, B+ 60MΩ) and
/// `  OL.  ` (E+ 220kΩ, 220MΩ, 220mV and continuity). Dropping the point is
/// what makes the middle one read as overload rather than fall through to
/// the "could not parse" branch.
fn is_overload(display_compact: &str) -> bool {
    display_compact.replace('.', "").contains("OL")
}

/// What the NCV display shows while no field is detected.
const NCV_IDLE: &str = "EF";

/// The NCV level on the display, `None` for text that is none of its forms.
///
/// The level is drawn as "-" segments, not a digit: "EF" while no field is
/// detected, one more "-" per level as it grows (manual §13; "   EF  " and
/// "     - " observed on 2026-09-07). A numeric display is still accepted in
/// case some firmware sends one.
fn ncv_level(display_compact: &str) -> Option<u8> {
    let dashes = display_compact.chars().filter(|c| *c == '-').count() as u8;
    if dashes > 0 {
        Some(dashes)
    } else if display_compact == NCV_IDLE {
        Some(0)
    } else {
        display_compact.parse::<u8>().ok()
    }
}

/// Decode the three UT61+ flag bytes (already masked with `& 0x0F`).
///
/// - byte 11 (`flag1`): bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX
/// - byte 12 (`flag2`): bit0=HV warning, bit1=Low Battery, bit2=!AUTO (inverted),
///   bit3=APO (§2.7; not carried)
/// - byte 13 (`flag3`): bit0=bar polarity, bit1=Peak MIN, bit2=Peak MAX,
///   bit3=AC (clear = DC)
///
/// `dc` marks the DC component of an AC+DC reading, which alternates frames
/// between its two components: flag3 bit 3 is set on the AC one (§2.7).
/// Elsewhere the mode says AC or DC, and `dc` stays clear.
fn parse_flags(mode: Mode, flag1: u8, flag2: u8, flag3: u8) -> StatusFlags {
    let ac_dc = matches!(mode, Mode::AcDcV | Mode::AcDcA | Mode::ClampAcDcA);
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
/// and UT61B+ captures. The reading is parsed the same either way.
///
/// Sections are `docs/research/ut61eplus/reverse-engineered-protocol.md`;
/// "family" is `docs/research/ut61-family/reverse-engineered-protocol.md`.
fn report_unrecognised_fields(
    payload: &[u8],
    mode: Mode,
    has_range: bool,
    table: &dyn DeviceTable,
) {
    const FAMILY: &str = "ut61eplus";
    let mode_byte = payload[0];
    // Family §3.1: the model's dial lists every mode it reaches, and every
    // mode the captures show is on it. A decodable byte off it is another
    // model's mode or one of §2.5's speculative ones. An empty dial is a
    // table that does not describe one.
    let dial = table.dial_positions();
    if !dial.is_empty() && !dial.iter().any(|p| p.contains(u16::from(mode_byte))) {
        report_unknown(
            FAMILY,
            "mode byte",
            format_args!(
                "{mode_byte:#04x} ({mode}), not on the {} dial",
                table.model_name()
            ),
        );
    } else if !has_range && mode != Mode::Ncv {
        // Family §5 and §6.2: NCV is the one mode without a range table;
        // every other has at least one rung, and the captures stay inside
        // them. A mode off the dial was reported above.
        report_unknown(
            FAMILY,
            "range byte",
            format_args!("mode {mode_byte:#04x} range {}", payload[1] & 0x0F),
        );
    }
    let flags = &payload[11..14];
    // Family §4: a model without Peak never sets P-MIN or P-MAX (flag3
    // bits 1-2).
    if payload[13] & 0x06 != 0 && table.peak_modes().is_empty() {
        report_unknown(
            FAMILY,
            "flag bits",
            format_args!("{flags:02X?}, Peak on a model without it"),
        );
    }
    // §2.6 (range byte) and §2.7 (flag bytes): a 0x30 high nibble.
    for i in [1, 11, 12, 13] {
        if payload[i] & 0xF0 != 0x30 {
            report_unknown(
                FAMILY,
                "byte prefix",
                format_args!("payload[{i}] = {:#04x}", payload[i]),
            );
        }
    }
    // §2.6: two decimal digits counting lit bar segments, of 46 at most
    // (family §2.1). The captures top out at 44.
    let (tens, ones) = (payload[9], payload[10]);
    if ones > 9 || u16::from(tens) * 10 + u16::from(ones) > 46 {
        report_unknown(
            FAMILY,
            "bar graph",
            format_args!("{:02X?}", &payload[9..11]),
        );
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
pub fn parse_measurement(payload: &[u8], table: &dyn DeviceTable) -> Result<Measurement> {
    check_len("ut61eplus", payload, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN)?;
    // §2.4: the measurement payload is 14 bytes; the rest is ignored.
    if payload.len() > UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
        report_unknown(
            "ut61eplus",
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

    // §2.5 lists no mode byte past 0x1E.
    let mode = Mode::from_byte(mode_byte).inspect_err(|_| {
        report_unknown("ut61eplus", "mode byte", format_args!("{mode_byte:#04x}"));
    })?;
    let display_raw = String::from_utf8_lossy(display_bytes).to_string();
    let progress = bar_hi * 10 + bar_lo;
    let flags = parse_flags(mode, flag1, flag2, flag3);

    // Look up range info from device table
    let range_info = table.range_info(mode, range_byte);
    report_unrecognised_fields(payload, mode, range_info.is_some(), table);
    let unit = range_info.map(|r| r.unit).unwrap_or("");
    let range_label = range_info.map(|r| r.label).unwrap_or("");

    // Parse display value.
    let display_trimmed = display_raw.trim();
    let display_compact: String = display_trimmed.chars().filter(|c| *c != ' ').collect();
    let value = if mode == Mode::Ncv {
        let level = ncv_level(&display_compact).unwrap_or_else(|| {
            report_unknown(
                "ut61eplus",
                "NCV display text",
                format_args!("{display_compact:?}, shown as level 0"),
            );
            0
        });
        MeasuredValue::NcvLevel(level)
    } else if is_overload(&display_compact) {
        MeasuredValue::Overload
    } else {
        match display_compact.parse::<f64>() {
            Ok(v) => MeasuredValue::Normal(v),
            Err(_) => {
                report_unknown(
                    "ut61eplus",
                    "display text",
                    format_args!("{display_compact:?}, shown as OL"),
                );
                MeasuredValue::Overload
            }
        }
    };

    Ok(Measurement {
        mode: Cow::Borrowed(mode.as_static_str()),
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: Some(progress),
        display_raw: Some(display_raw),
        flags,
        ..Measurement::from_payload(&payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN])
    })
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
pub fn make_test_measurement(
    mode: u8,
    range: u8,
    display: &[u8; 7],
    progress: (u8, u8),
    flags: (u8, u8, u8),
) -> Measurement {
    let table = tables::ut61e_plus::Ut61ePlusTable::new();
    let payload = make_payload(mode, range, display, progress, flags);
    parse_measurement(&payload, &table).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_support::snapshot;

    #[test]
    fn parse_no_flags_auto_on() {
        // All zero → AUTO is on (inverted logic), everything else off
        let flags = parse_flags(Mode::DcV, 0x00, 0x00, 0x00);
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
        let flags = parse_flags(Mode::DcV, 0x02, 0x00, 0x00);
        assert!(flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range);
    }

    #[test]
    fn parse_manual_range() {
        // flag2=0x04 → AUTO bit set → auto_range OFF
        let flags = parse_flags(Mode::DcV, 0x00, 0x04, 0x00);
        assert!(!flags.auto_range);
    }

    #[test]
    fn parse_low_battery() {
        // flag2=0x02 → LOW BAT
        let flags = parse_flags(Mode::DcV, 0x00, 0x02, 0x00);
        assert!(flags.low_battery);
        assert!(flags.auto_range); // AUTO still on (bit2 is clear)
    }

    #[test]
    fn parse_min_max() {
        // flag1: bit2=MIN, bit3=MAX
        let flags = parse_flags(Mode::DcV, 0x0C, 0x00, 0x00);
        assert!(flags.min);
        assert!(flags.max);
    }

    #[test]
    fn parse_all_flag1() {
        // flag1=0x0F: REL + HOLD + MIN + MAX
        let flags = parse_flags(Mode::DcV, 0x0F, 0x00, 0x00);
        assert!(flags.rel);
        assert!(flags.hold);
        assert!(flags.min);
        assert!(flags.max);
    }

    /// Our UT61E+ in AC+DC V across a 1.6 V cell (2026-09-19): the frames
    /// reading 1.61 V carry flag3 bit 3 clear, the 0.0000 V ones set.
    #[test]
    fn parse_dc_flag() {
        assert!(parse_flags(Mode::AcDcV, 0x00, 0x00, 0x01).dc);
        assert!(!parse_flags(Mode::AcDcV, 0x00, 0x00, 0x09).dc);
        assert!(!parse_flags(Mode::DcV, 0x00, 0x00, 0x00).dc);
    }

    #[test]
    fn parse_real_device_hold() {
        // Real capture: meter on DC V with HOLD active
        // flag bytes (masked): 0x02, 0x00, 0x01
        let flags = parse_flags(Mode::DcV, 0x02, 0x00, 0x01);
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
                .expect("known model")
                .capture_steps()
                .iter()
                .map(|s| s.id)
                .collect()
        };
        let e_plus = ids("ut61e+");
        assert!(e_plus.contains(&"acua"));
        for id in ["temp", "tempf", "loz"] {
            assert!(!e_plus.contains(&id), "{id} asked for on the E+");
        }
        let d_plus = ids("ut61d+");
        for id in ["temp", "tempf", "loz"] {
            assert!(d_plus.contains(&id), "{id} missing on the D+");
        }
    }
    use tables::ut61e_plus::Ut61ePlusTable;

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

    /// A mock that says it is the Bluetooth link, for the adapter-only paths.
    struct BluetoothMock(MockTransport);

    impl Transport for BluetoothMock {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.0.write(data)
        }
        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
            self.0.read_timeout(buf, timeout_ms)
        }
        fn send_feature_report(&self, data: &[u8]) -> Result<()> {
            self.0.send_feature_report(data)
        }
        fn transport_name(&self) -> &'static str {
            crate::BLUETOOTH
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

    /// A streaming protocol, started, over `frames`.
    fn streaming(frames: Vec<Vec<u8>>) -> (BluetoothMock, Ut61PlusProtocol) {
        let mock = BluetoothMock(MockTransport::new(frames));
        let mut p = Ut61PlusProtocol::new();
        p.init(&mock).unwrap();
        (mock, p)
    }

    /// The adapter streams whoever reads, so a reader slower than it finds
    /// a backlog; it gets the newest reading, not the oldest, and the next
    /// request starts from what arrives after it.
    #[test]
    fn a_slow_reader_gets_the_newest_streamed_reading() {
        let (mock, mut p) = streaming(vec![frame_in(0x02), frame_in(0x04), frame_in(0x05)]);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
        mock.0.push_response(frame_in(0x02));
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "DC V");
        assert_eq!(mock.0.written.borrow().len(), 1, "no poll went out");
    }

    /// A frame split across notifications, or two in one, still drains to
    /// the newest whole one; the unfinished tail waits for its next read.
    #[test]
    fn the_drain_follows_frames_across_reads() {
        let newest = frame_in(0x05);
        let (head, tail) = newest.split_at(7);
        let mut two = frame_in(0x02);
        two.extend(frame_in(0x04));
        let (mock, mut p) = streaming(vec![two, head.to_vec()]);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Hz");
        mock.0.push_response(tail.to_vec());
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
    }

    /// A queued frame that fails its checksum is dropped, not reported:
    /// a good reading behind it is newer.
    #[test]
    fn a_corrupt_queued_frame_is_dropped() {
        let mut corrupt = frame_in(0x04);
        *corrupt.last_mut().unwrap() ^= 0xFF;
        let (mock, mut p) = streaming(vec![frame_in(0x02), corrupt, frame_in(0x05)]);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
    }

    /// A backlog longer than one request drains is caught up over the next
    /// ones, so a long pause costs a few stale readings, not minutes of them.
    #[test]
    fn a_long_backlog_drains_over_a_few_requests() {
        let mut frames = vec![frame_in(0x02); MAX_DRAIN_READS + 10];
        frames.push(frame_in(0x05));
        let (mock, mut p) = streaming(frames);
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "DC V");
        assert_eq!(p.request_measurement(&mock).unwrap().mode, "Duty %");
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

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
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

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
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
    /// them (issue #19, 2026-09-11). Both verified models have now run every
    /// step they declare, so `capture --unverified` asks them for nothing.
    #[test]
    fn verified_models_are_asked_for_nothing() {
        let unverified = |model: &str| -> Vec<&'static str> {
            Ut61PlusProtocol::for_model(model)
                .expect("known model")
                .capture_steps()
                .into_iter()
                .filter(|s| !s.verified)
                .map(|s| s.id)
                .collect()
        };
        assert!(
            unverified("ut61e+").is_empty(),
            "the UT61E+ has run every step it declares: {:?}",
            unverified("ut61e+")
        );
        assert!(
            unverified("ut61b+").is_empty(),
            "the UT61B+ has run every step it declares: {:?}",
            unverified("ut61b+")
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
        for model in ["ut61e+", "ut61b+", "ut61d+"] {
            let proto = Ut61PlusProtocol::for_model(model).expect("known model");
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
    /// (docs/verification-backlog.md).
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
        let proto = Ut61PlusProtocol::for_model("ut61b+").expect("known model");
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

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
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
            matches!(&err, Error::UnsupportedCommand(m)
                if m == "peak cannot be set in DC V on this meter"),
            "{err}"
        );
        assert!(commands(&meter).is_empty());
    }

    /// DC V on the second rung, with bar-graph digits reading 26.
    #[test]
    fn parse_dc_voltage() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x01, b" 12.345", (0x02, 0x06), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
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
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
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
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range);
        assert!(!m.flags.rel);
    }

    /// The meter puts a space between the sign and the digits.
    #[test]
    fn parse_negative_with_space() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x03, 0x00, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-55.79)).abs() < 1e-6));
    }

    #[test]
    fn parse_payload_too_short() {
        let table = Ut61ePlusTable::new();
        let payload = vec![0x30; 10];
        assert!(parse_measurement(&payload, &table).is_err());
    }

    #[test]
    fn display_format() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }

    /// Every form the overload display takes, taken off the captures named
    /// in `is_overload`. `O.L` used to miss the check and reach `Overload`
    /// only through the "could not parse" fallback, which would have
    /// swallowed a corrupt display just as quietly.
    #[test]
    fn overload_is_read_wherever_the_decimal_point_lands() {
        let table = Ut61ePlusTable::new();
        for display in [b" .OL   ", b"  O.L  ", b"  OL.  ", b"    OL "] {
            let payload = make_payload(0x06, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            let m = parse_measurement(&payload, &table).unwrap();
            assert!(
                matches!(m.value, MeasuredValue::Overload),
                "display {:?}: {:?}",
                String::from_utf8_lossy(display),
                m.value
            );
        }
        // A reading that merely contains the letters is still a reading.
        let payload = make_payload(0x06, 0x00, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(_)), "{:?}", m.value);
    }

    /// The numeric branch of the NCV level, which **no meter has been seen to
    /// use**: both a UT61E+ and a UT61B+ draw the level as "-" segments. The
    /// branch is a guess at other firmware, so it is tested here and nowhere
    /// else — there was a `ncv_3` golden fixture asserting this frame until
    /// 2026-09-12, which made a hand-built payload look like a captured one.
    #[test]
    fn parse_ncv_numeric_fallback() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert_eq!(m.mode, "NCV");
        assert!(matches!(m.value, MeasuredValue::NcvLevel(3)));
    }

    /// NCV level from the "-" segments (manual §13). Only "EF" and a single
    /// dash have been seen on hardware; two dashes follow our counting rule.
    #[test]
    fn parse_ncv_dash_levels() {
        let table = Ut61ePlusTable::new();
        for (display, level) in [(b"   EF  ", 0u8), (b"     - ", 1), (b"    -- ", 2)] {
            let payload = make_payload(0x14, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            let m = parse_measurement(&payload, &table).unwrap();
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
        assert_eq!(ncv_level("3"), Some(3));
        assert_eq!(ncv_level("CUT"), None);
        assert_eq!(ncv_level(""), None);
    }

    /// Text no form covers — e.g. the "CUT" the manual says an overheated
    /// meter shows — still reads as OL, and is reported; the idle "EF" is not.
    #[test]
    fn unrecognised_display_text_is_reported() {
        let table = Ut61ePlusTable::new();
        let parse = |mode, display: &[u8; 7]| {
            let payload = make_payload(mode, 0x00, display, (0x00, 0x00), (0x00, 0x00, 0x00));
            crate::protocol::capture_reports(|| parse_measurement(&payload, &table).unwrap())
        };

        let (m, reports) = parse(0x06, b"  CUT  ");
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised display text: \"CUT\", shown as OL"]
        );

        let (m, reports) = parse(0x14, b"  ?!   ");
        assert!(matches!(m.value, MeasuredValue::NcvLevel(0)));
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised NCV display text: \"?!\", shown as level 0"]
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
        let table = tables::ut61b_plus::Ut61bPlusTable::new();

        let dcv = [
            0x02, 0x30, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        assert_eq!(
            snapshot(&parse_measurement(&dcv, &table).unwrap()),
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
            snapshot(&parse_measurement(&acv, &table).unwrap()),
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
        let m = parse_measurement(&ohm, &table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!((m.unit.as_ref(), m.range_label.as_ref()), ("MΩ", "60MΩ"));
        assert_eq!(m.progress, Some(30));

        let dcma = [
            0x0E, 0x30, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x30,
        ];
        let m = parse_measurement(&dcma, &table).unwrap();
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

    /// The name frame is the only reply that pins the exact model, and each
    /// sibling's name is its own registry entry.
    #[test]
    fn a_name_frame_picks_the_registry_entry() {
        for (frame, id, name) in [
            (NAME_UT61EPLUS, "ut61eplus", "UT61E+"),
            (NAME_UT61BPLUS, "ut61b+", "UT61B+"),
        ] {
            assert_eq!(
                recognised(&frame),
                Some(Evidence::Model {
                    id,
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
            recognised(&test_frame_be16(b"UT60BT")),
            Some(Evidence::Model {
                id: "ut61eplus",
                reported_name: Some("UT60BT".to_string()),
            })
        );
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
                fallback: "ut61eplus",
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
    use tables::RangeInfo;
    use tables::ut61b_plus::Ut61bPlusTable;
    use tables::ut61d_plus::Ut61dPlusTable;

    /// Parse `payload` with `table`, keeping what it reported.
    fn parse_reporting(
        payload: &[u8],
        table: &dyn DeviceTable,
    ) -> (Result<Measurement>, Vec<String>) {
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
        // AC+DC V with the DC bit, AC V with P-MAX, DC V under MAX, REL and HOLD.
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
            (
                "UT61E+",
                &Ut61ePlusTable::new() as &dyn DeviceTable,
                E_PLUS_FRAMES,
            ),
            ("UT61B+", &Ut61bPlusTable::new(), B_PLUS_FRAMES),
        ] {
            for frame in frames {
                let (m, reports) = parse_reporting(frame, table);
                assert!(m.is_ok(), "{model} {frame:02X?}: {m:?}");
                assert!(reports.is_empty(), "{model} {frame:02X?}: {reports:?}");
            }
        }
    }

    #[test]
    fn an_unknown_mode_byte_is_reported_and_still_rejected() {
        let payload = make_payload(0x1F, 0x00, b"  0.000", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
        assert!(matches!(m, Err(Error::UnknownMode(0x1F))), "{m:?}");
        assert_eq!(reports, ["ut61eplus: unrecognised mode byte: 0x1f"]);
    }

    /// Temperature is a UT61D+ position (family spec §3.1), not a UT61E+ one.
    #[test]
    fn a_mode_off_the_model_s_dial_is_reported() {
        let payload = make_payload(0x0A, 0x00, b"   23.4", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
        assert_eq!(m.unwrap().mode, "°C");
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised mode byte: 0x0a (°C), not on the UNI-T UT61E+ dial"]
        );

        let (_, reports) = parse_reporting(&payload, &Ut61dPlusTable::new());
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A table that describes no dial, with a rung for every mode.
    struct NoDial;

    static ANY_RUNG: RangeInfo = tables::r("any", "V");

    impl DeviceTable for NoDial {
        fn range_info(&self, _mode: Mode, _range: u8) -> Option<&RangeInfo> {
            Some(&ANY_RUNG)
        }

        fn model_name(&self) -> &'static str {
            "no dial"
        }
    }

    #[test]
    fn a_table_without_a_dial_takes_any_mode() {
        let payload = make_payload(0x13, 0x00, b"  0.000", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &NoDial);
        assert_eq!(m.unwrap().mode, "Live");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// DC V has four rungs on the UT61E+; NCV has no table at all and says
    /// nothing about it.
    #[test]
    fn a_range_byte_past_the_table_is_reported() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x04, b" 12.345", (0, 0), (0, 0, 0));
        let (m, reports) = parse_reporting(&payload, &table);
        assert_eq!(m.unwrap().range_label, "");
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised range byte: mode 0x02 range 4"]
        );

        let payload = make_payload(0x14, 0x00, b"   EF  ", (0, 0), (0, 0, 0));
        let (_, reports) = parse_reporting(&payload, &table);
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// §2.7: flag2 bit 3 is the protocol deck's APO flag. No capture has
    /// set it, but it is documented, so it passes without a report.
    #[test]
    fn the_apo_flag_bit_is_not_reported() {
        let payload = make_payload(0x02, 0x01, b" 12.345", (0, 0), (0, 0x08, 0));
        let (m, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
        assert!(m.unwrap().flags.auto_range);
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn a_byte_without_its_prefix_is_reported() {
        let mut payload = make_payload(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        payload[1] = 0x01;
        payload[12] = 0x00;
        let (m, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
        assert_eq!(m.unwrap().range_label, "22V");
        assert_eq!(
            reports,
            [
                "ut61eplus: unrecognised byte prefix: payload[1] = 0x01",
                "ut61eplus: unrecognised byte prefix: payload[12] = 0x00",
            ]
        );
    }

    /// The UT61B+ has no Peak (family spec §4); the UT61E+ does.
    #[test]
    fn peak_bits_on_a_model_without_peak_are_reported() {
        let payload = make_payload(0x00, 0x00, b"  0.037", (0, 0), (0, 0, F_PEAK_MAX));
        let (m, reports) = parse_reporting(&payload, &Ut61bPlusTable::new());
        assert!(m.unwrap().flags.peak_max);
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised flag bits: [30, 30, 34], Peak on a model without it"]
        );

        let (_, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn a_bar_graph_past_46_segments_is_reported() {
        let table = Ut61ePlusTable::new();
        for (bar, progress, report) in [
            ((4, 7), 47, Some("[04, 07]")),
            ((0, 10), 10, Some("[00, 0A]")),
            ((4, 6), 46, None),
        ] {
            let payload = make_payload(0x02, 0x01, b" 12.345", bar, (0, 0, 0));
            let (m, reports) = parse_reporting(&payload, &table);
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
        let (m, reports) = parse_reporting(&payload, &Ut61ePlusTable::new());
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
        let (evidence, reports) = capture_reports(|| recognised(&test_frame_be16(b"UT60BT")));
        assert!(
            matches!(
                evidence,
                Some(Evidence::Model {
                    id: "ut61eplus",
                    ..
                })
            ),
            "{evidence:?}"
        );
        assert_eq!(
            reports,
            ["ut61eplus: unrecognised model name: \"UT60BT\", using the UT61E+ tables"]
        );

        let (_, reports) = capture_reports(|| recognised(&NAME_UT61BPLUS));
        assert!(reports.is_empty(), "{reports:?}");
    }
}
