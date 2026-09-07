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

pub(crate) mod mode;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::cycle::{self, FlagSetting};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{
    Choice, DeviceProfile, Protocol, Setting, Stability, check_len, unknown_mode16,
    unsupported_setting,
};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// Decode a UT181A mode word (uint16 LE) into a human-readable string.
///
/// Nibble encoding: N3 N2 N1 N0
/// N3 = measurement family, N2 = sub-function, N1 = variant, N0 = 1=std/2=REL
fn decode_mode_word(mode: u16) -> Cow<'static, str> {
    let n3 = (mode >> 12) & 0xF;
    let n2 = (mode >> 8) & 0xF;
    let n1 = (mode >> 4) & 0xF;
    let n0 = mode & 0xF;

    // Two mode words break the "N0=2 means REL" rule (sigrok
    // MODE_CONT_OPEN / MODE_DIODE_ALARM; antage Beeper_Open /
    // Diode_Alarm agree):
    match mode {
        0x5212 => return Cow::Borrowed("Continuity (open)"),
        0x6112 => return Cow::Borrowed("Diode Alarm"),
        _ => {}
    }

    // Temperature families use N1 as the display arrangement, not the
    // generic variant nibble (sigrok: T1(T2), T2(T1), T1-T2, T2-T1).
    if n3 == 0x4 && (n2 == 0x2 || n2 == 0x3) {
        let family = if n2 == 0x2 { "°C" } else { "°F" };
        let arrangement = match n1 {
            0x2 => " T2",
            0x3 => " T1-T2",
            0x4 => " T2-T1",
            _ => "",
        };
        let rel = if n0 == 0x2 { " REL" } else { "" };
        return if arrangement.is_empty() && rel.is_empty() {
            Cow::Borrowed(family)
        } else {
            Cow::Owned(format!("{family}{arrangement}{rel}"))
        };
    }

    let family = match n3 {
        0x1 => "V AC",
        0x2 => "mV AC",
        0x3 => "V DC",
        0x4 => match n2 {
            0x1 => "mV DC",
            0x2 => "°C",
            0x3 => "°F",
            _ => return unknown_mode16(mode),
        },
        0x5 => match n2 {
            0x1 => "Ω",
            0x2 => "Continuity",
            0x3 => "nS",
            _ => return unknown_mode16(mode),
        },
        0x6 => match n2 {
            0x1 => "Diode",
            0x2 => "Capacitance",
            _ => return unknown_mode16(mode),
        },
        0x7 => match n2 {
            0x1 => "Hz",
            0x2 => "Duty %",
            0x3 => "Pulse Width",
            _ => return unknown_mode16(mode),
        },
        0x8 => match n2 {
            0x1 => "µA DC",
            0x2 => "µA AC",
            _ => return unknown_mode16(mode),
        },
        0x9 => match n2 {
            0x1 => "mA DC",
            0x2 => "mA AC",
            _ => return unknown_mode16(mode),
        },
        0xA => match n2 {
            0x1 => "A DC",
            0x2 => "A AC",
            _ => return unknown_mode16(mode),
        },
        _ => return unknown_mode16(mode),
    };

    let variant = match n1 {
        0x1 => "",
        0x2 => match (n3, n2) {
            // V AC / mV AC: frequency display
            (0x1 | 0x2, _) => " Hz",
            // V DC: AC+DC
            (0x3, _) => " AC+DC",
            // mV DC: 0x4121 = mV DC Peak per sigrok/antage (sigrok notes
            // the code might be 0x4131 — hardware check pending)
            (0x4, 0x1) => " Peak",
            // Currents: n1=2 on the DC sub-function (n2=1) is AC+DC
            // (sigrok MODE_uA/mA/A_DC_ACDC = 0x8121/0x9121/0xA121);
            // Hz applies only to the AC sub-function (n2=2)
            (0x8..=0xA, 0x1) => " AC+DC",
            (0x8..=0xA, 0x2) => " Hz",
            _ => "",
        },
        0x3 => " Peak",
        0x4 => match n3 {
            0x1 => " LPF",
            0x2 => " AC+DC",
            _ => "",
        },
        0x5 => " dBV",
        0x6 => " dBm",
        _ => "",
    };

    let rel = if n0 == 0x2 { " REL" } else { "" };

    // When no variant or rel suffix, return the static family string directly
    if variant.is_empty() && rel.is_empty() {
        Cow::Borrowed(family)
    } else {
        Cow::Owned(format!("{family}{variant}{rel}"))
    }
}

/// Parse a UT181A unit string from 8 bytes (null-terminated).
///
/// The meter sends Latin-1, not UTF-8 (spec §8: 0xB0 = degree symbol),
/// so decode byte-by-byte — `from_utf8_lossy` would mangle °C/°F into
/// replacement characters.
fn parse_unit_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect()
}

/// Labels for the two aux slots of a normal-format measurement.
///
/// The meter sends each sub-value's own unit but never says what the value
/// *is*, so the label has to come from the mode word. Two arrangements are
/// pinned by a real UT181A capture (@diego351, issue #5, 2026-09-02):
/// `0x4211` puts one thermocouple on the main display and the other in aux1,
/// and `0x1121` puts the frequency in aux1 with its period in aux2
/// (1/50.00875 Hz = 19.9965 ms, exactly the aux2 reading in that frame). The
/// remaining modes follow the same nibble rule with no frame behind them; any
/// slot whose meaning is unknown keeps its positional label.
fn aux_labels(mode: u16) -> (&'static str, &'static str) {
    let n3 = (mode >> 12) & 0xF;
    let n2 = (mode >> 8) & 0xF;
    let n1 = (mode >> 4) & 0xF;

    // Temperature: n1 selects the display arrangement, so the aux slot holds
    // the other probe. The differential arrangements (n1 = 3/4) put a
    // difference on the main display and no source says which probe lands in
    // the aux slot — those stay positional.
    if n3 == 0x4 && (n2 == 0x2 || n2 == 0x3) {
        return match n1 {
            0x1 => ("T2", "Aux2"),
            0x2 => ("T1", "Aux2"),
            _ => ("Aux1", "Aux2"),
        };
    }

    // The same n1 = 2 codes `decode_mode_word` suffixes with " Hz": the
    // frequency display, with the period alongside it.
    if n1 == 0x2 && matches!((n3, n2), (0x1 | 0x2, _) | (0x8..=0xA, 0x2)) {
        return ("Frequency", "Period");
    }

    ("Aux1", "Aux2")
}

const UT181A_COMMANDS: &[&str] = &[
    "hold",
    "range",
    "auto",
    "rel",
    "minmax",
    "exit_minmax",
    "monitor",
    "save",
];

/// Build a UT181A command frame: AB CD len_lo len_hi payload chk_lo chk_hi.
/// Length = payload.len() + 2 (includes checksum).
/// Checksum = LE sum of length field + payload bytes.
fn build_command(payload: &[u8]) -> Vec<u8> {
    let len_val = (payload.len() + 2) as u16;
    let mut frame = vec![0xAB, 0xCD];
    frame.push((len_val & 0xFF) as u8);
    frame.push((len_val >> 8) as u8);
    frame.extend_from_slice(payload);
    let checksum: u16 = frame[2..].iter().map(|&b| b as u16).sum();
    frame.push((checksum & 0xFF) as u8);
    frame.push((checksum >> 8) as u8);
    frame
}

/// Readings taken after a setting command before concluding it did nothing.
///
/// Three, as on the cycling families: one for the frame already in flight,
/// one for the meter's own reaction, one to spare. Nobody has timed a real
/// UT181A, so the budget is in reads rather than a delay.
const FLAG_CONFIRM_READS: usize = 3;

/// SET_MODE (opcode 0x01) carrying `word` as uint16 LE.
///
/// See docs/research/ut181/reverse-engineered-protocol.md
/// §6.1 Mode switching (SET_MODE) -- [VENDOR].
fn build_set_mode(word: u16) -> Vec<u8> {
    let [lo, hi] = word.to_le_bytes();
    build_command(&[0x01, lo, hi])
}

/// Protocol implementation for the UT181A.
pub struct Ut181aProtocol {
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
    pub fn new() -> Self {
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
                // Stays Experimental so the badge keeps linking to the verification
                // issue; README and docs/supported-devices.md say the same.
                stability: Stability::Experimental,
                supported_commands: UT181A_COMMANDS,
                // aux1 + aux2 + COMP High + COMP Low.
                max_aux_values: 4,
                verification_issue: Some(5),
            },
        }
    }

    /// Write a command frame, then wait for the meter's reply code.
    ///
    /// The meter answers a command with a type-0x01 packet carrying "OK" or
    /// "ER" (spec §4.1). "ER" is the only signal that a mode or range the dial
    /// doesn't allow was refused, so it becomes an error the caller can show;
    /// without it a rejected command looked exactly like an accepted one.
    ///
    /// Measurement frames that arrive while we wait are dropped — the meter
    /// keeps streaming through a command. Whatever follows the reply stays in
    /// `rx_buf`, so the next `request_measurement` resumes mid-stream instead
    /// of losing the bytes a blind drain used to throw away.
    ///
    /// The reply framing is hardware-unverified, so silence is treated as
    /// success: a meter that answers nothing must not fail every command.
    fn send_frame(&mut self, transport: &dyn Transport, frame: &[u8], what: &str) -> Result<()> {
        /// How long to wait for the reply before assuming the meter is not
        /// going to send one.
        const REPLY_TIMEOUT: Duration = Duration::from_millis(300);
        /// Guard against a transport that returns empty without blocking,
        /// which would otherwise busy-spin until the deadline. Same reasoning
        /// as `framing::read_uart_bytes`.
        const MAX_EMPTY_READS: usize = 256;

        debug!("ut181a: sending {what}: {frame:02X?}");
        transport.write(frame)?;

        let deadline = Instant::now() + REPLY_TIMEOUT;
        let mut empty_reads = 0usize;
        let mut tmp = [0u8; 64];
        loop {
            match framing::extract_frame_abcd_2byte_le16(&self.rx_buf) {
                Ok(Some((payload, consumed))) => {
                    self.rx_buf.drain(..consumed);
                    // Reply code packets are type 0x01; anything else is the
                    // measurement stream running underneath us.
                    if payload.first() == Some(&0x01) {
                        return reply_result(&payload, what);
                    }
                    debug!(
                        "ut181a: dropping a {} byte frame while waiting for the {what} reply",
                        payload.len()
                    );
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    // Drop the corrupt data rather than re-extracting it every
                    // pass; `request_measurement` resyncs on the next header.
                    warn!("ut181a: frame error waiting for the {what} reply: {e}, clearing buffer");
                    self.rx_buf.clear();
                }
            }

            // `checked_duration_since` rather than a subtraction: a backward
            // clock jump must not panic here.
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_default();
            let timeout_ms = i32::try_from(remaining.as_millis()).unwrap_or(i32::MAX);
            if timeout_ms == 0 || empty_reads >= MAX_EMPTY_READS {
                debug!("ut181a: no reply to {what} within {REPLY_TIMEOUT:?}, assuming accepted");
                return Ok(());
            }
            let n = transport.read_timeout(&mut tmp, timeout_ms)?;
            if n == 0 {
                empty_reads += 1;
            } else {
                empty_reads = 0;
                self.rx_buf.extend_from_slice(&tmp[..n]);
            }
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

    /// The states `setting` offers in `mode`, [`cycle::OFF_STATE`] first.
    ///
    /// HOLD is a button press; REL is a mode-word variant and only exists
    /// where the vendor app enables it (`mode::rel_supported`); MIN/MAX is
    /// SET_MIN_MAX's on/off byte, not the MAX/MIN ring the cycling meters
    /// walk. Peak is a mode variant on this meter, reached through
    /// [`Setting::Mode`], so it is not offered here.
    fn flag_states(setting: FlagSetting, mode: u16) -> &'static [u16] {
        match setting {
            FlagSetting::Hold | FlagSetting::MinMax => &[0, 1],
            FlagSetting::Rel if mode::rel_supported(mode) => &[0, 1],
            FlagSetting::Rel | FlagSetting::Peak => &[],
        }
    }

    /// Display name of one state. MIN/MAX is a plain switch here, so it is
    /// named off/on rather than after the MAX and MIN badges.
    fn flag_label(setting: FlagSetting, id: u16) -> Cow<'static, str> {
        match setting {
            FlagSetting::MinMax => Cow::Borrowed(if id == cycle::OFF_STATE {
                cycle::OFF_LABEL
            } else {
                cycle::ON_LABEL
            }),
            other => other.label(id),
        }
    }

    fn check_flag_id(setting: FlagSetting, mode: u16, id: u16) -> Result<()> {
        let states = Self::flag_states(setting, mode);
        if states.is_empty() {
            return Err(Error::UnsupportedCommand(format!(
                "{} cannot be set in {} ({mode:#06x}) on this meter",
                setting.setting(),
                decode_mode_word(mode)
            )));
        }
        if !states.contains(&id) {
            return Err(Error::UnsupportedCommand(format!(
                "{} has no state {id}; {} offers off, on",
                setting.setting(),
                decode_mode_word(mode)
            )));
        }
        Ok(())
    }

    /// Read the meter back until it reports something other than `seen`.
    ///
    /// The meter streams through a command, so the frame in flight when it
    /// landed still shows the old state; that has to cost a re-read, never a
    /// second command. Same discipline as `cycle::observe_after_press`, which
    /// this family cannot use — it takes no button presses to walk.
    fn confirm_flag(
        &mut self,
        transport: &dyn Transport,
        setting: FlagSetting,
        seen: u16,
    ) -> Result<u16> {
        for _ in 0..FLAG_CONFIRM_READS {
            let now = setting.state(&self.request_measurement(transport)?.flags);
            if now != seen {
                return Ok(now);
            }
            debug!(
                "ut181a: meter still reports {} {}, re-reading",
                setting.setting(),
                Self::flag_label(setting, seen)
            );
        }
        Ok(seen)
    }

    /// Put `setting` in state `id`: one absolute command (REL, MIN/MAX) or
    /// one button press (HOLD), then the meter's own flags to confirm it.
    fn select_flag(
        &mut self,
        transport: &dyn Transport,
        setting: FlagSetting,
        id: u16,
    ) -> Result<()> {
        // A state the last known mode does not offer costs no I/O at all.
        if let Some(mode) = self.last_mode_raw {
            Self::check_flag_id(setting, mode, id)?;
        }
        // Which state the meter is in is only in the stream, and the mode may
        // have moved since the last reading, so both come from a fresh one.
        let reading = self.request_measurement(transport)?;
        Self::check_flag_id(setting, reading.mode_raw, id)?;
        let seen = setting.state(&reading.flags);
        if seen == id {
            return Ok(());
        }
        let frame = match setting {
            // 0x12 = button-press command, 0x5A = HOLD button code, as in
            // `send_command`. The only toggle here, so the fresh reading
            // above is what decides whether to send it at all.
            FlagSetting::Hold => build_command(&[0x12, 0x5A]),
            FlagSetting::Rel => {
                // `check_flag_id` already refused a mode with no REL.
                let word = mode::rel_word(reading.mode_raw, id != cycle::OFF_STATE)
                    .ok_or_else(|| unsupported_setting(setting.setting()))?;
                build_set_mode(word)
            }
            FlagSetting::MinMax => build_command(&[0x04, id as u8]),
            FlagSetting::Peak => return Err(unsupported_setting(setting.setting())),
        };
        let what = format!("{} {}", setting.setting(), Self::flag_label(setting, id));
        self.send_frame(transport, &frame, &what)?;
        let now = self.confirm_flag(transport, setting, seen)?;
        if now == id {
            Ok(())
        } else {
            Err(Error::CommandRejected(format!(
                "{what} did nothing; the meter is still in {}",
                Self::flag_label(setting, now)
            )))
        }
    }
}

/// Turn a type-0x01 reply packet into a result.
///
/// Payload is `[0x01, 'O', 'K']` or `[0x01, 'E', 'R']` (spec §4.1). Anything
/// else is logged and treated as acceptance: the reply format is
/// hardware-unverified, so an unrecognised answer is more likely our gap than
/// a refusal.
fn reply_result(payload: &[u8], what: &str) -> Result<()> {
    match &payload[1..] {
        [b'O', b'K', ..] => {
            debug!("ut181a: {what} acknowledged");
            Ok(())
        }
        [b'E', b'R', ..] => Err(Error::CommandRejected(format!(
            "{what} rejected by the meter — check the dial position"
        ))),
        other => {
            debug!("ut181a: unrecognised reply to {what}: {other:02X?}");
            Ok(())
        }
    }
}

impl Protocol for Ut181aProtocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // User must enable "Communication ON" on the meter
        // Send CMD_CONT_DATA (0x05, enable=1) to start the measurement stream.
        // Verified against real UT181A hardware: bytes AB CD 04 00 05 01 0A 00.
        debug!("ut181a: sending start-stream command (CMD_CONT_DATA)");
        let frame = build_command(&[0x05, 0x01]);
        transport.write(&frame)?;
        debug!("ut181a: init (streaming, manual enable required)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_2byte_le16,
            // Only accept measurement frames (type 0x02)
            |p| !p.is_empty() && p[0] == 0x02,
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

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
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
        use crate::protocol::{CaptureStep, Expect, Need, RangeExpect, ValueExpect};

        // Only V DC, V AC and °C have been seen on real hardware (issue #5).
        // Core UT181A modes
        vec![
            CaptureStep::basic("vdc", "Set meter to V DC")
                .verified()
                .gate()
                .expect(Expect::mode("V DC").value(ValueExpect::Finite)),
            CaptureStep::basic("dcv_short", "V DC mode: touch the two probe tips together.")
                .gate()
                .needs(&[Need::ShortedLeads])
                .expect(Expect::mode("V DC").value(ValueExpect::Finite)),
            CaptureStep::basic(
                "dcv_negative",
                "V DC mode: leads reversed on a battery or any DC source (skip if none).",
            )
            .gate()
            .needs(&[Need::DcSource])
            .expect(Expect::mode("V DC").value(ValueExpect::Negative)),
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
            CaptureStep::basic(
                "ohm",
                "Set meter to Resistance. Leave leads open (should show OL).",
            )
            .gate()
            .expect(Expect::mode("Ω").value(ValueExpect::Overload)),
            // Open and shorted leads repeat one digit value, so a digit-order
            // or digit-value bug hides; a body reading spreads the digits out.
            CaptureStep::basic(
                "ohm_body",
                "Resistance mode: hold one probe tip between the fingers of each \
                 hand (body resistance, hundreds of kΩ).",
            )
            .gate()
            .expect(Expect::mode("Ω").value(ValueExpect::Finite)),
            CaptureStep::basic(
                "ohm_short",
                "Resistance mode: touch the two probe tips together.",
            )
            .gate()
            .needs(&[Need::ShortedLeads])
            .expect(Expect::mode("Ω").value(ValueExpect::Finite)),
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

/// Look up range label from mode word and range byte.
///
/// Uses the table from protocol spec Section 7. The family nibble (N3) and
/// sub-function nibble (N2) together determine which range table applies.
/// Temperature and A current have fixed ranges (no label).
fn lookup_range_label(mode_word: u16, range: u8) -> &'static str {
    if range == 0 {
        return "Auto";
    }
    let family = (mode_word >> 12) & 0xF;
    let sub = (mode_word >> 8) & 0xF;

    match (family, sub, range) {
        // mV DC (0x4, sub 0x1) and mV AC (0x2, sub 0x1)
        (0x2 | 0x4, 0x1, 1) => "60mV",
        (0x2 | 0x4, 0x1, 2) => "600mV",

        // V AC (0x1) and V DC (0x3)
        (0x1 | 0x3, _, 1) => "6V",
        (0x1 | 0x3, _, 2) => "60V",
        (0x1 | 0x3, _, 3) => "600V",
        (0x1 | 0x3, _, 4) => "1000V",

        // µA DC (0x8, sub 0x1) and µA AC (0x8, sub 0x2)
        (0x8, _, 1) => "600\u{00B5}A",
        (0x8, _, 2) => "6000\u{00B5}A",

        // mA DC (0x9, sub 0x1) and mA AC (0x9, sub 0x2)
        (0x9, _, 1) => "60mA",
        (0x9, _, 2) => "600mA",

        // A DC/AC (0xA): fixed 10A range, no label needed
        (0xA, _, _) => "",

        // Resistance (0x5, sub 0x1)
        (0x5, 0x1, 1) => "600\u{2126}",
        (0x5, 0x1, 2) => "6k\u{2126}",
        (0x5, 0x1, 3) => "60k\u{2126}",
        (0x5, 0x1, 4) => "600k\u{2126}",
        (0x5, 0x1, 5) => "6M\u{2126}",
        (0x5, 0x1, 6) => "60M\u{2126}",

        // Continuity (0x5, sub 0x2), Conductance (0x5, sub 0x3): fixed range
        (0x5, _, _) => "",

        // Diode (0x6, sub 0x1): fixed range
        (0x6, 0x1, _) => "",

        // Capacitance (0x6, sub 0x2)
        (0x6, 0x2, 1) => "6nF",
        (0x6, 0x2, 2) => "60nF",
        (0x6, 0x2, 3) => "600nF",
        (0x6, 0x2, 4) => "6\u{00B5}F",
        (0x6, 0x2, 5) => "60\u{00B5}F",
        (0x6, 0x2, 6) => "600\u{00B5}F",
        (0x6, 0x2, 7) => "6mF",
        (0x6, 0x2, 8) => "60mF",

        // Frequency (0x7, sub 0x1)
        (0x7, 0x1, 1) => "60Hz",
        (0x7, 0x1, 2) => "600Hz",
        (0x7, 0x1, 3) => "6kHz",
        (0x7, 0x1, 4) => "60kHz",
        (0x7, 0x1, 5) => "600kHz",
        (0x7, 0x1, 6) => "6MHz",
        (0x7, 0x1, 7) => "60MHz",

        // Duty cycle (0x7, sub 0x2), Pulse width (0x7, sub 0x3): the vendor
        // range combo holds four items (60/600/6000/60000, see the spec's
        // §7.1) but no source says what the LCD calls them, so the rungs
        // stay unnamed and `range_choices` offers none of them.
        (0x7, _, _) => "",

        // Temperature (0x4, sub 0x2/0x3): fixed range
        (0x4, _, _) => "",

        _ => "",
    }
}

/// Parse a UT181A measurement payload (type 0x02 packet).
///
/// Common header (after type byte):
/// - byte 0:   type (0x02, already verified)
/// - byte 1:   misc (flags: bit7=HOLD, bits4-6=format, bit3=bargraph, etc.)
/// - byte 2:   misc2 (bit0=auto, bit1=HV, bit3=lead_error, bit4=COMP, bit5=record)
/// - bytes 3-4: mode word (uint16 LE)
/// - byte 5:   range (0x00=auto, 0x01-0x08=manual)
///
/// After header, the format-dependent value section starts at byte 6.
///
/// Full value = 13 bytes: float32(4) + precision(1) + unit_string(8)
/// Short value = 5 bytes: float32(4) + precision(1)
/// Parse a 13-byte "full value": float32(4) + precision(1) + unit_string(8).
fn parse_full_value(data: &[u8]) -> Result<(MeasuredValue, Option<String>, String)> {
    if data.len() < 13 {
        return Err(Error::invalid_response_msg(format!(
            "ut181a full value too short: {} bytes, need 13",
            data.len()
        )));
    }
    let float = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let precision = data[4];
    let unit = parse_unit_string(&data[5..13]);
    let is_overload = precision & 0x01 != 0 || precision & 0x02 != 0;
    let dp = ((precision >> 4) & 0x0F) as usize;

    if is_overload || float.is_nan() || float.is_infinite() {
        Ok((MeasuredValue::Overload, None, unit))
    } else {
        let v = float as f64;
        Ok((MeasuredValue::Normal(v), Some(format!("{v:.dp$}")), unit))
    }
}

/// Parse a 5-byte "short value": float32(4) + precision(1).
fn parse_short_value(data: &[u8]) -> Result<(MeasuredValue, Option<String>)> {
    if data.len() < 5 {
        return Err(Error::invalid_response_msg(format!(
            "ut181a short value too short: {} bytes, need 5",
            data.len()
        )));
    }
    let float = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let precision = data[4];
    let is_overload = precision & 0x01 != 0 || precision & 0x02 != 0;
    let dp = ((precision >> 4) & 0x0F) as usize;

    if is_overload || float.is_nan() || float.is_infinite() {
        Ok((MeasuredValue::Overload, None))
    } else {
        let v = float as f64;
        Ok((MeasuredValue::Normal(v), Some(format!("{v:.dp$}"))))
    }
}

/// Build an `AuxValue` from a full value parse result.
fn make_aux(
    label: &'static str,
    value: MeasuredValue,
    unit: &str,
    display_raw: Option<String>,
    elapsed_secs: Option<u32>,
) -> AuxValue {
    AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Owned(unit.to_string()),
        display_raw,
        elapsed_secs,
    }
}

pub fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    // Minimum header: type(1) + misc(1) + misc2(1) + mode(2) + range(1) = 6
    check_len("ut181a", payload, 6)?;

    let misc = payload[1];
    let misc2 = payload[2];
    let mode_word = u16::from_le_bytes([payload[3], payload[4]]);
    let range = payload[5];

    let format_type = (misc >> 4) & 0x07;
    let hold = misc & 0x80 != 0;
    let auto_range = misc2 & 0x01 != 0;
    let hv_warning = misc2 & 0x02 != 0;
    let lead_error = misc2 & 0x08 != 0;
    let comp_active = misc2 & 0x10 != 0;
    let record = misc2 & 0x20 != 0;

    let mode = decode_mode_word(mode_word);
    let data = &payload[6..]; // format-dependent value section

    let (value, display_raw, unit, mut aux_values) = match format_type {
        // Normal format (0x00)
        0x00 => {
            if data.len() < 13 {
                return Err(Error::invalid_response(
                    format!("ut181a normal format too short: {} bytes", payload.len()),
                    payload,
                ));
            }
            let (val, disp, unit) = parse_full_value(data)?;
            let mut aux = Vec::new();
            let mut offset = 13;
            let (aux1_label, aux2_label) = aux_labels(mode_word);

            // Aux1 (optional, misc bit 1)
            if misc & 0x02 != 0 && data.len() >= offset + 13 {
                let (av, ad, au) = parse_full_value(&data[offset..])?;
                aux.push(make_aux(aux1_label, av, &au, ad, None));
                offset += 13;
            }
            // Aux2 (optional, misc bit 2)
            if misc & 0x04 != 0 && data.len() >= offset + 13 {
                let (av, ad, au) = parse_full_value(&data[offset..])?;
                aux.push(make_aux(aux2_label, av, &au, ad, None));
                offset += 13;
            }
            // Bargraph (optional, misc bit 3) — skip for now, just advance offset
            if misc & 0x08 != 0 && data.len() >= offset + 12 {
                offset += 12; // float32(4) + unit(8)
            }

            // COMP extension (when misc2 bit 4 set)
            if comp_active && data.len() >= offset + 7 {
                let comp_mode = data[offset];
                let comp_result = data[offset + 1];
                let comp_prec = data[offset + 2];
                let high_float = f32::from_le_bytes([
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                ]);
                // COMP digits live in the LOW nibble, unshifted — unlike
                // the other precision fields (sigrok protocol.c:112
                // "1 byte digits, not shifted as in other precision
                // fields"; decode at protocol.c:2123).
                let dp = (comp_prec & 0x0F) as usize;
                let comp_mode_str = match comp_mode {
                    0 => "INNER",
                    1 => "OUTER",
                    2 => "BELOW",
                    3 => "ABOVE",
                    _ => "?",
                };
                let result_str = if comp_result == 0 { "PASS" } else { "FAIL" };
                let high_v = high_float as f64;
                aux.push(make_aux(
                    "COMP High",
                    MeasuredValue::Normal(high_v),
                    &unit,
                    Some(format!("{high_v:.dp$}")),
                    None,
                ));

                // Low limit present for INNER/OUTER modes
                if (comp_mode == 0 || comp_mode == 1) && data.len() >= offset + 11 {
                    let low_float = f32::from_le_bytes([
                        data[offset + 7],
                        data[offset + 8],
                        data[offset + 9],
                        data[offset + 10],
                    ]);
                    let low_v = low_float as f64;
                    aux.push(make_aux(
                        "COMP Low",
                        MeasuredValue::Normal(low_v),
                        &unit,
                        Some(format!("{low_v:.dp$}")),
                        None,
                    ));
                }

                debug!("ut181a: COMP {comp_mode_str} {result_str} high={high_float}");
            }

            (val, disp, unit, aux)
        }

        // Relative format (0x10 >> 4 = 1)
        0x01 => {
            // 3 full values: relative (delta), reference, absolute
            if data.len() < 39 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a relative format too short: {} bytes, need >= 45",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (rel_val, rel_disp, rel_unit) = parse_full_value(data)?;
            let (ref_val, ref_disp, ref_unit) = parse_full_value(&data[13..])?;
            let (abs_val, abs_disp, abs_unit) = parse_full_value(&data[26..])?;

            let aux = vec![
                make_aux("Reference", ref_val, &ref_unit, ref_disp, None),
                make_aux("Absolute", abs_val, &abs_unit, abs_disp, None),
            ];
            // Main value = delta (matches meter display)
            (rel_val, rel_disp, rel_unit, aux)
        }

        // Min/Max format (0x20 >> 4 = 2)
        0x02 => {
            // current(5) + max(5)+ts(4) + avg(5)+ts(4) + min(5)+ts(4) + unit(8) = 40
            if data.len() < 40 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a minmax format too short: {} bytes, need >= 46",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (cur_val, cur_disp) = parse_short_value(data)?;

            let (max_val, max_disp) = parse_short_value(&data[5..])?;
            let max_ts = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);

            let (avg_val, avg_disp) = parse_short_value(&data[14..])?;
            let avg_ts = u32::from_le_bytes([data[19], data[20], data[21], data[22]]);

            let (min_val, min_disp) = parse_short_value(&data[23..])?;
            let min_ts = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

            let unit = parse_unit_string(&data[32..40]);

            let aux = vec![
                make_aux("Max", max_val, &unit, max_disp, Some(max_ts)),
                make_aux("Average", avg_val, &unit, avg_disp, Some(avg_ts)),
                make_aux("Min", min_val, &unit, min_disp, Some(min_ts)),
            ];
            (cur_val, cur_disp, unit, aux)
        }

        // Peak format (0x40 >> 4 = 4)
        0x04 => {
            // 2 full values: peak max, peak min
            if data.len() < 26 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a peak format too short: {} bytes, need >= 32",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (pmax_val, pmax_disp, pmax_unit) = parse_full_value(data)?;
            let (pmin_val, pmin_disp, pmin_unit) = parse_full_value(&data[13..])?;

            let aux = vec![make_aux("Peak Min", pmin_val, &pmin_unit, pmin_disp, None)];
            (pmax_val, pmax_disp, pmax_unit, aux)
        }

        // Unknown format — try to parse as normal
        _ => {
            debug!("ut181a: unknown format_type {format_type:#x}, treating as normal");
            if data.len() < 13 {
                return Err(Error::invalid_response(
                    format!("ut181a unknown format too short: {} bytes", payload.len()),
                    payload,
                ));
            }
            let (val, disp, unit) = parse_full_value(data)?;
            (val, disp, unit, vec![])
        }
    };

    // COMP extension can also apply to relative/peak, but only documented for
    // normal format. Parse it there only; for other formats, just set the flag.
    let _ = &mut aux_values; // suppress unused_mut if no COMP

    let flags = StatusFlags {
        hold,
        auto_range,
        hv_warning,
        lead_error,
        comp: comp_active,
        record,
        min: format_type == 0x02,
        max: format_type == 0x02,
        rel: format_type == 0x01,
        peak_max: format_type == 0x04,
        peak_min: format_type == 0x04,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: mode_word,
        range_raw: range,
        value,
        unit: Cow::Owned(unit),
        range_label: Cow::Borrowed(lookup_range_label(mode_word, range)),
        display_raw,
        flags,
        aux_values,
        ..Measurement::from_payload(payload)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(
        mode: u16,
        value: f32,
        precision: u8,
        unit: &[u8; 8],
        misc: u8,
        misc2: u8,
    ) -> Vec<u8> {
        let vbytes = value.to_le_bytes();
        let mbytes = mode.to_le_bytes();
        let mut p = vec![
            0x02,  // type
            misc,  // misc
            misc2, // misc2
            mbytes[0], mbytes[1], // mode word LE
            0x00,      // range
            vbytes[0], vbytes[1], vbytes[2], vbytes[3], // value
            precision, // precision
        ];
        p.extend_from_slice(unit); // 8 bytes
        p
    }

    #[test]
    fn parse_vdc() {
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V DC");
        assert_eq!(m.unit, "VDC");
        assert!(m.flags.auto_range);
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 12.345).abs() < 0.01);
        } else {
            panic!("expected Normal value");
        }
    }

    #[test]
    fn parse_vac() {
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V AC");
        assert_eq!(m.unit, "VAC");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x5111, 470.0, 0x20, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "~");
    }

    #[test]
    fn parse_overload_precision() {
        // Precision bit 0 = +OL
        let payload = make_payload(0x5111, 0.0, 0x01, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x80, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_hv_warning() {
        let payload = make_payload(0x3111, 500.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hv_warning);
    }

    #[test]
    fn decode_mode_word_known() {
        assert_eq!(decode_mode_word(0x1111), "V AC");
        assert_eq!(decode_mode_word(0x3111), "V DC");
        assert_eq!(decode_mode_word(0x5111), "Ω");
        assert_eq!(decode_mode_word(0x6211), "Capacitance");
        assert_eq!(decode_mode_word(0x7111), "Hz");
        assert_eq!(decode_mode_word(0x8111), "µA DC");
        assert_eq!(decode_mode_word(0xA111), "A DC");
    }

    #[test]
    fn decode_mode_word_variants() {
        assert_eq!(decode_mode_word(0x1121), "V AC Hz");
        assert_eq!(decode_mode_word(0x1131), "V AC Peak");
        assert_eq!(decode_mode_word(0x1141), "V AC LPF");
        assert_eq!(decode_mode_word(0x3121), "V DC AC+DC");
        assert_eq!(decode_mode_word(0x1112), "V AC REL");
        // DC currents with n1=2 are AC+DC (sigrok MODE_*_DC_ACDC), not Hz;
        // Hz applies only to the AC sub-function.
        assert_eq!(decode_mode_word(0x8121), "µA DC AC+DC");
        assert_eq!(decode_mode_word(0x9121), "mA DC AC+DC");
        assert_eq!(decode_mode_word(0xA121), "A DC AC+DC");
        assert_eq!(decode_mode_word(0x8221), "µA AC Hz");
        // 0x4121 = mV DC Peak (sigrok/antage)
        assert_eq!(decode_mode_word(0x4121), "mV DC Peak");
        // Non-REL exceptions to the n0=2 rule
        assert_eq!(decode_mode_word(0x5212), "Continuity (open)");
        assert_eq!(decode_mode_word(0x6112), "Diode Alarm");
        // Temperature display arrangements
        assert_eq!(decode_mode_word(0x4211), "°C");
        assert_eq!(decode_mode_word(0x4221), "°C T2");
        assert_eq!(decode_mode_word(0x4231), "°C T1-T2");
        assert_eq!(decode_mode_word(0x4241), "°C T2-T1");
        assert_eq!(decode_mode_word(0x4321), "°F T2");
    }

    #[test]
    fn parse_unit_string_latin1_degree() {
        // 0xB0 = '°' in Latin-1; from_utf8_lossy would produce U+FFFD.
        assert_eq!(parse_unit_string(&[0xB0, b'C', 0, 0, 0, 0, 0, 0]), "°C");
        assert_eq!(parse_unit_string(&[0xB0, b'F', 0, 0, 0, 0, 0, 0]), "°F");
    }

    #[test]
    fn aux_labels_by_mode() {
        // One probe on the main display, the other in aux1. The n1 = 1
        // arrangement is hardware-confirmed (issue #5); n1 = 2 is its
        // documented mirror.
        assert_eq!(aux_labels(0x4211).0, "T2");
        assert_eq!(aux_labels(0x4221).0, "T1");
        assert_eq!(aux_labels(0x4311).0, "T2");
        // Differential arrangements: no source says which probe feeds the aux
        // slot, so the label stays positional.
        assert_eq!(aux_labels(0x4231).0, "Aux1");
        assert_eq!(aux_labels(0x4241).0, "Aux1");
        // The modes decode_mode_word suffixes with " Hz" carry the frequency
        // and its period.
        assert_eq!(aux_labels(0x1121), ("Frequency", "Period"));
        assert_eq!(aux_labels(0x2121), ("Frequency", "Period"));
        assert_eq!(aux_labels(0x8221), ("Frequency", "Period"));
        // Everything else keeps the positional labels.
        assert_eq!(aux_labels(0x3111), ("Aux1", "Aux2"));
        assert_eq!(aux_labels(0x8121), ("Aux1", "Aux2"));
    }

    /// Hex as a capture report writes it in `raw_hex` (spaces optional).
    fn hex(s: &str) -> Vec<u8> {
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(clean.len().is_multiple_of(2), "odd-length hex: {clean}");
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).expect("valid hex"))
            .collect()
    }

    /// Real UT181A frame: temperature with two thermocouples connected
    /// (@diego351, issue #5, 2026-09-02 — the first hardware confirmation of
    /// a UT181A mode other than V DC).
    ///
    /// Reaches the normal-format aux walk that the synthetic `make_payload`
    /// frames never do: 26 payload bytes after the 6-byte header = 2 x 13, so
    /// the aux1 slot is what makes the frame add up.
    #[test]
    fn parse_real_frame_temp_dual_probe() {
        let payload = hex(
            "02 02 01 11 42 01 F0 ED CA 41 10 B0 43 00 43 00 00 00 00 26 FC C4 41 \
             10 B0 43 00 00 00 00 00 5A",
        );
        assert_eq!(payload.len(), 32);

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x4211);
        assert_eq!(m.mode, "°C");
        // Latin-1: the meter sends 0xB0 for the degree sign.
        assert_eq!(m.unit, "°C");
        // Precision byte 0x10 => 1 decimal place, matching the LCD.
        assert_eq!(m.display_raw.as_deref(), Some("25.4"));
        // Fixed-range family, so no label even though the meter sent range 0x01.
        assert_eq!(m.range_label, "");
        assert!(m.flags.auto_range);
        assert!(!m.flags.hv_warning);

        assert_eq!(m.aux_values.len(), 1);
        let t2 = &m.aux_values[0];
        assert_eq!(t2.label, "T2");
        assert_eq!(t2.unit, "°C");
        assert_eq!(t2.display_raw.as_deref(), Some("24.6"));
    }

    /// Real UT181A frame: V AC with the Hz secondary display, mains on the
    /// 600 V range (@diego351, issue #5, 2026-09-02).
    ///
    /// The 51 payload bytes after the header only add up as 13 + 13 + 13 + 12:
    /// main, aux1 and aux2 each carry a precision byte, the bargraph does not
    /// (spec §5.3). Get the bargraph field's size wrong and this frame
    /// desynchronises.
    #[test]
    fn parse_real_frame_vac_hz_bargraph() {
        let payload = hex(
            "02 0E 03 21 11 03 52 38 6F 43 20 56 41 43 00 00 00 00 00 F6 08 48 42 \
             20 48 7A 00 00 00 00 00 5A D5 F8 9F 41 20 6D 73 00 43 00 00 00 00 3D \
             06 71 43 56 41 43 00 00 00 00 00",
        );
        assert_eq!(payload.len(), 57);

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x1121);
        assert_eq!(m.mode, "V AC Hz");
        assert_eq!(m.unit, "VAC");
        assert_eq!(m.display_raw.as_deref(), Some("239.22"));
        // misc2 bit 1: the meter was flagging mains voltage.
        assert!(m.flags.hv_warning);
        assert!(m.flags.auto_range);
        // Auto-range settled on 600V (range byte 0x03).
        assert_eq!(m.range_label, "600V");

        assert_eq!(m.aux_values.len(), 2);
        assert_eq!(m.aux_values[0].label, "Frequency");
        assert_eq!(m.aux_values[0].unit, "Hz");
        assert_eq!(m.aux_values[0].display_raw.as_deref(), Some("50.01"));
        assert_eq!(m.aux_values[1].label, "Period");
        assert_eq!(m.aux_values[1].unit, "ms");
        assert_eq!(m.aux_values[1].display_raw.as_deref(), Some("20.00"));
    }

    #[test]
    fn decode_mode_word_unknown() {
        let s = decode_mode_word(0xFFFF);
        assert!(s.starts_with("Unknown"));
    }

    #[test]
    fn parse_nan_overload() {
        let payload = make_payload(0x5111, f32::NAN, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x02, 0x00, 0x00, 0x11, 0x31]; // 5 bytes, need >= 19
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x7211, 50.0, 0x00, b"%\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x7211);
        assert_eq!(m.mode, "Duty %");
    }

    #[test]
    fn display_raw_uses_precision_decimal_places() {
        // precision 0x40 => bits 4-7 = 4 decimal places
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.3450"));

        // precision 0x20 => bits 4-7 = 2 decimal places
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("230.50"));

        // precision 0x00 => 0 decimal places
        let payload = make_payload(0x5111, 470.0, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("470"));
    }

    #[test]
    fn display_raw_none_on_overload() {
        let payload = make_payload(0x5111, 0.0, 0x01, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.display_raw.is_none());
    }

    #[test]
    fn build_command_hold() {
        // Button-press (0x12) with HOLD button code (0x5A), per antage.
        let frame = build_command(&[0x12, 0x5A]);
        // len = 2 + 2 = 4; checksum = 04 + 00 + 12 + 5A = 0x70
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x12, 0x5A, 0x70, 0x00]);
    }

    #[test]
    fn build_command_set_range_auto() {
        let frame = build_command(&[0x02, 0x00]);
        // AB CD 04 00 02 00 06 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x02, 0x00, 0x06, 0x00]);
    }

    #[test]
    fn build_command_set_minmax_on() {
        let frame = build_command(&[0x04, 0x01]);
        // AB CD 04 00 04 01 09 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x04, 0x01, 0x09, 0x00]);
    }

    #[test]
    fn build_command_monitor_on() {
        let frame = build_command(&[0x05, 0x01]);
        // AB CD 04 00 05 01 0A 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x05, 0x01, 0x0A, 0x00]);
    }

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

    #[test]
    fn range_label_auto() {
        assert_eq!(lookup_range_label(0x3111, 0x00), "Auto");
        assert_eq!(lookup_range_label(0x5111, 0x00), "Auto");
    }

    #[test]
    fn range_label_voltage() {
        assert_eq!(lookup_range_label(0x3111, 1), "6V");
        assert_eq!(lookup_range_label(0x3111, 2), "60V");
        assert_eq!(lookup_range_label(0x3111, 3), "600V");
        assert_eq!(lookup_range_label(0x3111, 4), "1000V");
        // V AC uses same ranges
        assert_eq!(lookup_range_label(0x1111, 2), "60V");
    }

    #[test]
    fn range_label_millivolt() {
        assert_eq!(lookup_range_label(0x4111, 1), "60mV");
        assert_eq!(lookup_range_label(0x4111, 2), "600mV");
        assert_eq!(lookup_range_label(0x2111, 1), "60mV");
    }

    #[test]
    fn range_label_resistance() {
        assert_eq!(lookup_range_label(0x5111, 1), "600\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 3), "60k\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 6), "60M\u{2126}");
    }

    #[test]
    fn range_label_capacitance() {
        assert_eq!(lookup_range_label(0x6211, 1), "6nF");
        assert_eq!(lookup_range_label(0x6211, 4), "6\u{00B5}F");
        assert_eq!(lookup_range_label(0x6211, 8), "60mF");
    }

    #[test]
    fn range_label_frequency() {
        assert_eq!(lookup_range_label(0x7111, 1), "60Hz");
        assert_eq!(lookup_range_label(0x7111, 5), "600kHz");
        assert_eq!(lookup_range_label(0x7111, 7), "60MHz");
    }

    #[test]
    fn range_label_current() {
        assert_eq!(lookup_range_label(0x8111, 1), "600\u{00B5}A");
        assert_eq!(lookup_range_label(0x9111, 2), "600mA");
        // A current: fixed range
        assert_eq!(lookup_range_label(0xA111, 1), "");
    }

    #[test]
    fn range_label_fixed_range_modes() {
        // Temperature, continuity, conductance, diode: no range label
        assert_eq!(lookup_range_label(0x4211, 1), ""); // Temp C
        assert_eq!(lookup_range_label(0x5211, 1), ""); // Continuity
        assert_eq!(lookup_range_label(0x5311, 1), ""); // Conductance
        assert_eq!(lookup_range_label(0x6111, 1), ""); // Diode
        assert_eq!(lookup_range_label(0x7211, 1), ""); // Duty cycle
    }

    #[test]
    fn range_raw_populated() {
        let payload = make_payload(0x3111, 12.0, 0x20, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        // range byte is at payload[5] which make_payload sets to 0x00
        assert_eq!(m.range_raw, 0x00);
        assert_eq!(m.range_label, "Auto");
    }

    /// Build a full value block (13 bytes): float32 LE + precision + unit(8).
    fn full_value(val: f32, precision: u8, unit: &[u8; 8]) -> Vec<u8> {
        let mut v = val.to_le_bytes().to_vec();
        v.push(precision);
        v.extend_from_slice(unit);
        v
    }

    /// Build a short value block (5 bytes): float32 LE + precision.
    fn short_value(val: f32, precision: u8) -> Vec<u8> {
        let mut v = val.to_le_bytes().to_vec();
        v.push(precision);
        v
    }

    /// Build a relative format payload (format 0x10).
    fn make_relative_payload(mode: u16, delta: f32, reference: f32, absolute: f32) -> Vec<u8> {
        let mbytes = mode.to_le_bytes();
        let mut p = vec![
            0x02, // type
            0x10, // misc: format_type=1 (relative) in bits 4-6
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        p.extend_from_slice(&full_value(delta, 0x30, b"VDC\0\0\0\0\0"));
        p.extend_from_slice(&full_value(reference, 0x30, b"VDC\0\0\0\0\0"));
        p.extend_from_slice(&full_value(absolute, 0x30, b"VDC\0\0\0\0\0"));
        p
    }

    #[test]
    fn parse_relative_format() {
        let payload = make_relative_payload(0x3112, 2.345, 10.0, 12.345);
        let m = parse_measurement(&payload).unwrap();

        assert_eq!(m.mode, "V DC REL");
        assert!(m.flags.rel);
        // Main value is the delta
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 2.345).abs() < 0.01);
        } else {
            panic!("expected Normal value");
        }
        // Two aux values: Reference and Absolute
        assert_eq!(m.aux_values.len(), 2);
        assert_eq!(m.aux_values[0].label, "Reference");
        assert_eq!(m.aux_values[1].label, "Absolute");
        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v - 10.0).abs() < 0.01);
        } else {
            panic!("expected Normal ref value");
        }
        if let MeasuredValue::Normal(v) = m.aux_values[1].value {
            assert!((v - 12.345).abs() < 0.01);
        } else {
            panic!("expected Normal abs value");
        }
    }

    #[test]
    fn parse_relative_too_short() {
        // 6 header + only 26 bytes of data (need 39)
        let mut payload = vec![0x02, 0x10, 0x01, 0x11, 0x31, 0x00];
        payload.extend_from_slice(&full_value(1.0, 0x20, b"VDC\0\0\0\0\0"));
        payload.extend_from_slice(&full_value(2.0, 0x20, b"VDC\0\0\0\0\0"));
        // Missing third value
        assert!(parse_measurement(&payload).is_err());
    }

    /// A MIN/MAX-format payload (misc format_type 2) — how the meter reports
    /// while SET_MIN_MAX is on, which is what sets the MIN and MAX flags.
    fn minmax_payload(mode: u16) -> Vec<u8> {
        let mbytes = mode.to_le_bytes();
        let mut payload = vec![
            0x02, // type
            0x20, // misc: format_type=2 (minmax)
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // current: 5.0
        payload.extend_from_slice(&short_value(5.0, 0x30));
        // max: 10.0, timestamp 120s
        payload.extend_from_slice(&short_value(10.0, 0x30));
        payload.extend_from_slice(&120u32.to_le_bytes());
        // avg: 7.5, timestamp 60s
        payload.extend_from_slice(&short_value(7.5, 0x30));
        payload.extend_from_slice(&60u32.to_le_bytes());
        // min: 3.0, timestamp 30s
        payload.extend_from_slice(&short_value(3.0, 0x30));
        payload.extend_from_slice(&30u32.to_le_bytes());
        // shared unit
        payload.extend_from_slice(b"VDC\0\0\0\0\0");
        payload
    }

    #[test]
    fn parse_minmax_format() {
        let payload = minmax_payload(0x3111);
        let m = parse_measurement(&payload).unwrap();

        assert_eq!(m.mode, "V DC");
        assert!(m.flags.min);
        assert!(m.flags.max);
        // Main value = current
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 5.0).abs() < 0.01);
        } else {
            panic!("expected Normal current value");
        }
        assert_eq!(m.unit, "VDC");

        // 3 aux values: Max, Average, Min
        assert_eq!(m.aux_values.len(), 3);
        assert_eq!(m.aux_values[0].label, "Max");
        assert_eq!(m.aux_values[0].elapsed_secs, Some(120));
        assert_eq!(m.aux_values[1].label, "Average");
        assert_eq!(m.aux_values[1].elapsed_secs, Some(60));
        assert_eq!(m.aux_values[2].label, "Min");
        assert_eq!(m.aux_values[2].elapsed_secs, Some(30));

        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v - 10.0).abs() < 0.01);
        } else {
            panic!("expected Normal max value");
        }
    }

    #[test]
    fn parse_minmax_too_short() {
        let mbytes = 0x3111u16.to_le_bytes();
        let mut payload = vec![0x02, 0x20, 0x01, mbytes[0], mbytes[1], 0x00];
        // Only 10 bytes of data (need 40)
        payload.extend_from_slice(&short_value(5.0, 0x30));
        payload.extend_from_slice(&short_value(10.0, 0x30));
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn parse_peak_format() {
        let mbytes = 0x3131u16.to_le_bytes(); // V DC Peak
        let mut payload = vec![
            0x02, // type
            0x40, // misc: format_type=4 (peak)
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        payload.extend_from_slice(&full_value(15.0, 0x30, b"VDC\0\0\0\0\0"));
        payload.extend_from_slice(&full_value(-3.0, 0x30, b"VDC\0\0\0\0\0"));

        let m = parse_measurement(&payload).unwrap();

        assert_eq!(m.mode, "V DC Peak");
        assert!(m.flags.peak_max);
        assert!(m.flags.peak_min);
        // Main value = peak max
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 15.0).abs() < 0.01);
        } else {
            panic!("expected Normal peak max value");
        }
        // 1 aux: Peak Min
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "Peak Min");
        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v + 3.0).abs() < 0.01);
        } else {
            panic!("expected Normal peak min value");
        }
    }

    #[test]
    fn parse_peak_too_short() {
        let mbytes = 0x3131u16.to_le_bytes();
        let mut payload = vec![0x02, 0x40, 0x01, mbytes[0], mbytes[1], 0x00];
        // Only one full value (need two)
        payload.extend_from_slice(&full_value(15.0, 0x30, b"VDC\0\0\0\0\0"));
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn parse_lead_error_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x08);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.lead_error);
    }

    #[test]
    fn parse_comp_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x10);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.comp);
    }

    #[test]
    fn parse_record_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x20);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.record);
    }

    #[test]
    fn parse_comp_extension() {
        // Normal format with COMP active
        let mbytes = 0x3111u16.to_le_bytes();
        let mut payload = vec![
            0x02, // type
            0x00, // misc: normal format
            0x11, // misc2: auto_range + COMP (bit 4)
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // Main value
        payload.extend_from_slice(&full_value(5.0, 0x30, b"VDC\0\0\0\0\0"));
        // COMP extension: INNER mode, PASS, precision 0x30, high=10.0, low=1.0
        payload.push(0x00); // comp_mode = INNER
        payload.push(0x00); // result = PASS
        payload.push(0x30); // precision
        payload.extend_from_slice(&10.0f32.to_le_bytes()); // high limit
        payload.extend_from_slice(&1.0f32.to_le_bytes()); // low limit

        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.comp);
        // Should have COMP High and COMP Low aux values
        assert_eq!(m.aux_values.len(), 2);
        assert_eq!(m.aux_values[0].label, "COMP High");
        assert_eq!(m.aux_values[1].label, "COMP Low");
        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v - 10.0).abs() < 0.01);
        } else {
            panic!("expected Normal comp high");
        }
        if let MeasuredValue::Normal(v) = m.aux_values[1].value {
            assert!((v - 1.0).abs() < 0.01);
        } else {
            panic!("expected Normal comp low");
        }
    }

    #[test]
    fn parse_normal_with_aux1() {
        let mbytes = 0x4211u16.to_le_bytes(); // Temp C T1(T2)
        let mut payload = vec![
            0x02, // type
            0x02, // misc: bit 1 = has aux1
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // Main value: T1
        payload.extend_from_slice(&full_value(23.5, 0x10, b"\xB0C\0\0\0\0\0\0"));
        // Aux1: T2
        payload.extend_from_slice(&full_value(21.0, 0x10, b"\xB0C\0\0\0\0\0\0"));

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "\u{00B0}C");
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "T2");
        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v - 21.0).abs() < 0.01);
        } else {
            panic!("expected Normal aux1 value");
        }
    }

    // --- Remote control: mode selection, REL, range, replies ---------------
    //
    // Every command below is traced from the vendor Windows app and has never
    // run against a meter (research spec §6.1), so these tests pin the bytes
    // we chose to send, not behaviour anyone has observed.

    use crate::transport::mock::MockTransport;

    /// A protocol that has already parsed one measurement, so the
    /// mode-relative commands have a dial position to work from.
    fn proto_in(mode_word: u16, range: u8) -> (Ut181aProtocol, MockTransport) {
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

    #[test]
    fn an_ok_reply_completes_the_command() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        mock.push_response(build_command(&[0x01, b'O', b'K']));
        proto.send_command(&mock, "auto").unwrap();
    }

    #[test]
    fn an_er_reply_becomes_a_rejection() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        mock.push_response(build_command(&[0x01, b'E', b'R']));
        let err = proto.send_command(&mock, "auto").unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(_)),
            "got {err:?}, want CommandRejected"
        );
    }

    /// The reply framing is hardware-unverified, so a silent meter must not
    /// turn every command into an error.
    #[test]
    fn silence_is_treated_as_acceptance() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        proto.send_command(&mock, "auto").unwrap();
    }

    /// The meter keeps streaming through a command, so the reply can arrive
    /// behind a measurement frame.
    #[test]
    fn a_measurement_frame_before_the_reply_is_skipped() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        mock.push_response(build_command(&make_payload(
            0x3111,
            2.0,
            0x20,
            b"VDC\0\0\0\0\0",
            0x00,
            0x01,
        )));
        mock.push_response(build_command(&[0x01, b'E', b'R']));
        let err = proto.send_command(&mock, "auto").unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(_)),
            "got {err:?}, want CommandRejected"
        );
    }

    /// Bytes trailing the reply belong to the stream. The old blind drain
    /// threw them away; the next `request_measurement` must still see them.
    #[test]
    fn bytes_after_the_reply_stay_buffered_for_the_next_measurement() {
        let (mut proto, mock) = proto_in(0x3111, 0);
        let mut burst = build_command(&[0x01, b'O', b'K']);
        burst.extend_from_slice(&build_command(&make_payload(
            0x3111,
            7.5,
            0x20,
            b"VDC\0\0\0\0\0",
            0x00,
            0x01,
        )));
        mock.push_response(burst);

        proto.send_command(&mock, "auto").unwrap();
        // The mock has nothing left to hand out, so this can only come from
        // the bytes that arrived with the reply.
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("7.50"));
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
}
