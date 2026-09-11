//! UT181A remote control: the AB CD command frames, the OK/ER reply the
//! meter answers them with, and the flag toggles (HOLD, REL, MIN/MAX) that
//! confirm themselves by reading the stream back.

use super::Ut181aProtocol;
use super::mode;
use super::parse::decode_mode_word;
use crate::error::{Error, Result};
use crate::protocol::cycle::{self, FlagSetting};
use crate::protocol::framing;
use crate::protocol::{Protocol, unsupported_setting};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::{Duration, Instant};

pub(super) const UT181A_COMMANDS: &[&str] = &[
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
pub(super) fn build_command(payload: &[u8]) -> Vec<u8> {
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

/// SET_MONITOR (opcode `0x05`, enable = 1): the meter is silent until it
/// arrives, then streams measurement frames
/// (`docs/research/ut181/reverse-engineered-protocol.md` §7). Verified against
/// real UT181A hardware: bytes AB CD 04 00 05 01 0A 00.
///
/// `pub(crate)` because detection sends it too, and a UT181A that answers the
/// probe is left in the state opening it would have produced anyway.
pub(crate) fn set_monitor_frame() -> Vec<u8> {
    build_command(&[0x05, 0x01])
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
pub(super) fn build_set_mode(word: u16) -> Vec<u8> {
    let [lo, hi] = word.to_le_bytes();
    build_command(&[0x01, lo, hi])
}

impl Ut181aProtocol {
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
    pub(super) fn send_frame(
        &mut self,
        transport: &dyn Transport,
        frame: &[u8],
        what: &str,
    ) -> Result<()> {
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

    /// The states `setting` offers in `mode`, [`cycle::OFF_STATE`] first.
    ///
    /// HOLD is a button press; REL is a mode-word variant and only exists
    /// where the vendor app enables it (`mode::rel_supported`); MIN/MAX is
    /// SET_MIN_MAX's on/off byte, not the MAX/MIN ring the cycling meters
    /// walk. Peak is a mode variant on this meter, reached through
    /// [`Setting::Mode`], so it is not offered here.
    pub(super) fn flag_states(setting: FlagSetting, mode: u16) -> &'static [u16] {
        match setting {
            FlagSetting::Hold | FlagSetting::MinMax => &[0, 1],
            FlagSetting::Rel if mode::rel_supported(mode) => &[0, 1],
            FlagSetting::Rel | FlagSetting::Peak => &[],
        }
    }

    /// Display name of one state. MIN/MAX is a plain switch here, so it is
    /// named off/on rather than after the MAX and MIN badges.
    pub(super) fn flag_label(setting: FlagSetting, id: u16) -> Cow<'static, str> {
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
    pub(super) fn select_flag(
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

#[cfg(test)]
mod tests {
    use super::super::parse::tests::make_payload;
    use super::super::tests::proto_in;
    use super::*;

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
}
