//! UT171 remote control: the key commands UNI-T's software sends (spec
//! §4.7), and the OK, ER or NO packet the meter answers them with (§5.11).
//!
//! The shared buttons go out in every function: the manual's lists of where
//! a key works stay the meter's to enforce, through its refusal. The context
//! keys follow the manual's lists, as every family's do.

use super::{Ut171Protocol, parse};
use crate::error::{Rejected, Result, Sent, Unsupported};
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, build_abcd_le16};
use crate::protocol::{ButtonLabel, MeterKey, MeterKeys, Protocol, command, unknown_mode};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;

pub(super) const UT171_COMMANDS: &[&str] = &[
    command::HOLD,
    command::REL,
    command::EXIT_REL,
    command::RANGE,
    command::AUTO,
    command::MINMAX,
    command::EXIT_MINMAX,
    command::PEAK,
    "hz",
    "save",
    "connect",
    "pause",
];

/// The functions the Hz% key works in: "V~, I~, Hz/% only" (manual p.11),
/// and the AC+DC pages, where it steps the aux display through AC, DC and
/// frequency (manual pp.12, 21). LoZ V~ and VFC are V~ functions.
const HZ_PERCENT_MODES: [u16; 14] = [
    0x01, 0x03, 0x04, 0x06, 0x07, 0x1E, 0x12, 0x13, 0x15, 0x16, 0x18, 0x19, 0x1C, 0x0F,
];

/// The Hz% key's short press (command `03 5A`, spec §4.7); its long press
/// opens the setup menu, which no command reaches.
static HZ_PERCENT: MeterKey = MeterKey {
    command: "hz",
    label: "Hz/%",
    in_words: false,
    hover: None,
    applies: |m| HZ_PERCENT_MODES.contains(&m.mode_raw),
};

/// STORE's short press (command `0B 5A`, spec §4.7), which the manual names
/// in no function, so it is offered in all of them.
static STORE: MeterKey = MeterKey {
    command: "save",
    label: "STORE",
    in_words: false,
    hover: Some("Save the reading to the meter's memory"),
    applies: |_| true,
};

/// MAX MIN steps MAX, AVG, MIN (manual p.10).
static MIN_MAX: ButtonLabel = ButtonLabel {
    command: command::MINMAX,
    label: None,
    hover: Some("Record maximum, average and minimum readings — click to cycle, × to exit"),
};

/// One command enters Peak and the other leaves it; which one goes out
/// follows the reading's Peak bit. The manual offers Peak in AC V and AC A.
static PEAK: ButtonLabel = ButtonLabel {
    command: command::PEAK,
    label: None,
    hover: Some("Peak hold in AC V and A — click to enter or leave"),
};

/// REL takes the reading on the display as its reference and shows it on
/// the aux display (manual p.10); the × leaves it.
static REL: ButtonLabel = ButtonLabel {
    command: command::REL,
    label: None,
    hover: Some("Take the reading as the reference — × to exit"),
};

pub(super) const METER_KEYS: MeterKeys = MeterKeys {
    context: &[HZ_PERCENT, STORE],
    button_labels: &[REL, MIN_MAX, PEAK],
    ..MeterKeys::NONE
};

/// How a command waits for its reply: OK accepts, ER and NO refuse, and the
/// stream's frames pass through the stream filter's own check.
const REPLY_WAIT: framing::ReplyWait = framing::ReplyWait {
    family: super::FAMILY,
    on_other: |payload| {
        super::is_measurement_frame(payload);
    },
    refusals: &[*b"ER", *b"NO"],
};

/// What the last reading said that a command depends on.
#[derive(Debug, Clone, Copy)]
pub(super) struct Shown {
    mode: u8,
    range: u8,
    auto: bool,
    rel: bool,
    peak: bool,
    /// The main float's bytes, when the display shows a number: what REL
    /// takes as its reference.
    reference: Option<[u8; 4]>,
}

impl Shown {
    pub(super) fn of(m: &Measurement) -> Self {
        let p = &m.raw_payload;
        Shown {
            mode: p.get(3).copied().unwrap_or(0),
            range: m.range_raw,
            auto: m.flags.auto_range,
            rel: m.flags.rel,
            peak: parse::peak_on(p),
            reference: match (&m.value, p.get(5..9)) {
                (MeasuredValue::Normal(_), Some(&[a, b, c, d])) => Some([a, b, c, d]),
                _ => None,
            },
        }
    }

    /// The mode's name, for a refusal; not reported again.
    fn mode_name(self) -> Cow<'static, str> {
        parse::known_mode(self.mode).map_or_else(|| unknown_mode(self.mode), Cow::Borrowed)
    }
}

/// The range byte RANGE sends: a press in auto range takes the meter to
/// manual on the row it is on (the manual's "click to switch over
/// automatic→manual range"); in manual it steps to the next row, after the
/// last back to the first. `None` for a mode with no rows.
fn next_range(shown: Shown) -> Option<u8> {
    let rows = parse::range_rows(shown.mode).len();
    let current = usize::from(shown.range);
    let next = match (rows, shown.auto, (1..=rows).contains(&current)) {
        (0, ..) => return None,
        (_, true, true) => current,
        (_, false, true) => current % rows + 1,
        (_, _, false) => 1,
    };
    u8::try_from(next).ok()
}

impl Ut171Protocol {
    /// What the last reading showed, reading one first when there is none
    /// yet (`dmm-cli command` sends with no reading taken).
    fn require_shown(&mut self, transport: &dyn Transport) -> Result<Shown> {
        if let Some(shown) = self.last {
            return Ok(shown);
        }
        Ok(Shown::of(&self.request_measurement(transport)?))
    }

    pub(super) fn send_key(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let frame = match command {
            // The stream's own switch (spec §4.3), written as it always was.
            "connect" | "pause" => {
                let frame = if command == "connect" {
                    super::connect_frame()
                } else {
                    super::pause_frame()
                };
                debug!("ut171: sending command {command}: {frame:02X?}");
                return transport.write(&frame);
            }
            command::HOLD => build_abcd_le16(&[0x07, 0x5A]),
            command::RANGE => {
                let shown = self.require_shown(transport)?;
                let Some(row) = next_range(shown) else {
                    return Err(Unsupported::NoRanges {
                        mode: shown.mode_name(),
                    }
                    .into());
                };
                build_abcd_le16(&[0x02, row])
            }
            command::AUTO => build_abcd_le16(&[0x02, 0x00]),
            // The software's REL takes a typed reference; the meter's key
            // takes the reading on the display, so that is what goes.
            command::REL => {
                let shown = self.require_shown(transport)?;
                if shown.rel {
                    return Err(Rejected::RelIsOn.into());
                }
                let Some([a, b, c, d]) = shown.reference else {
                    return Err(Rejected::RelNeedsReading.into());
                };
                build_abcd_le16(&[0x04, 0x01, a, b, c, d])
            }
            command::EXIT_REL => build_abcd_le16(&[0x04, 0x00]),
            command::MINMAX => build_abcd_le16(&[0x05, 0x01]),
            command::EXIT_MINMAX => build_abcd_le16(&[0x05, 0x00]),
            command::PEAK => {
                let shown = self.require_shown(transport)?;
                build_abcd_le16(&[0x06, u8::from(!shown.peak)])
            }
            "hz" => build_abcd_le16(&[0x03, 0x5A]),
            "save" => build_abcd_le16(&[0x0B, 0x5A]),
            _ => return Err(Unsupported::Command(command.to_string()).into()),
        };
        framing::send_le16_command(
            &mut self.rx_buf,
            transport,
            &frame,
            &Sent::Command(command.to_string()),
            &REPLY_WAIT,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::tests::{payload, with_aux};
    use super::*;
    use crate::error::Error;
    use crate::transport::mock::MockTransport;

    fn reply(word: &[u8; 2]) -> Vec<u8> {
        build_abcd_le16(&[0x01, word[0], word[1]])
    }

    /// A protocol that has read `payload`, and the mock it read it from,
    /// with an OK queued for the next write.
    fn proto_after(payload: Vec<u8>) -> (Ut171Protocol, MockTransport) {
        let mock = MockTransport::new(vec![build_abcd_le16(&payload)]);
        let mut proto = Ut171Protocol::new();
        proto.request_measurement(&mock).unwrap();
        mock.push_reply(reply(b"OK"));
        (proto, mock)
    }

    /// V DC on range 2 in manual, 1.2345 V shown.
    fn vdc_manual() -> Vec<u8> {
        let mut p = payload(0x02, 2, 1.2345, 0x40, 0x00);
        p[2] = 0x00;
        p
    }

    fn sent(command: &str, payload: Vec<u8>) -> Vec<u8> {
        let (mut proto, mock) = proto_after(payload);
        proto.send_command(&mock, command).unwrap();
        let written = mock.written.borrow();
        assert_eq!(written.len(), 1, "{written:02X?}");
        written[0].clone()
    }

    #[test]
    fn the_key_frames_are_the_software_s() {
        for (command, bytes) in [
            (command::HOLD, vec![0x07, 0x5A]),
            (command::AUTO, vec![0x02, 0x00]),
            (command::EXIT_REL, vec![0x04, 0x00]),
            (command::MINMAX, vec![0x05, 0x01]),
            (command::EXIT_MINMAX, vec![0x05, 0x00]),
            ("hz", vec![0x03, 0x5A]),
            ("save", vec![0x0B, 0x5A]),
        ] {
            assert_eq!(
                sent(command, vdc_manual()),
                build_abcd_le16(&bytes),
                "{command}"
            );
        }
        // Spot checks against spec §4.7's written frames.
        assert_eq!(
            sent(command::HOLD, vdc_manual()),
            [0xAB, 0xCD, 0x04, 0x00, 0x07, 0x5A, 0x65, 0x00]
        );
        assert_eq!(
            sent("save", vdc_manual()),
            [0xAB, 0xCD, 0x04, 0x00, 0x0B, 0x5A, 0x69, 0x00]
        );
    }

    #[test]
    fn range_steps_in_manual_and_takes_the_row_from_auto() {
        let manual = |range| {
            let mut p = payload(0x0A, range, 1.0, 0x20, 0x10);
            p[2] = 0x00;
            p
        };
        assert_eq!(sent(command::RANGE, manual(2)), build_abcd_le16(&[0x02, 3]));
        assert_eq!(sent(command::RANGE, manual(6)), build_abcd_le16(&[0x02, 1]));
        // Auto range on row 3: manual, on row 3.
        assert_eq!(
            sent(command::RANGE, payload(0x0A, 3, 1.0, 0x20, 0x10)),
            build_abcd_le16(&[0x02, 3])
        );
        // A range byte off the rows starts at the first.
        assert_eq!(sent(command::RANGE, manual(0)), build_abcd_le16(&[0x02, 1]));
    }

    #[test]
    fn range_in_a_mode_with_no_rows_is_refused_unsent() {
        let (mut proto, mock) = proto_after(payload(0x0D, 0, 0.5, 0x40, 0x1B));
        let err = proto.send_command(&mock, command::RANGE).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(Unsupported::NoRanges { mode }) if mode == "Diode"),
            "{err:?}"
        );
        assert!(mock.written.borrow().is_empty());
    }

    #[test]
    fn rel_sends_the_reading_s_own_float() {
        let mut expected = vec![0x04, 0x01];
        expected.extend_from_slice(&1.2345f32.to_le_bytes());
        assert_eq!(sent(command::REL, vdc_manual()), build_abcd_le16(&expected));
    }

    #[test]
    fn rel_is_refused_on_ol_and_while_on() {
        let (mut proto, mock) = proto_after(payload(0x0A, 1, 0.0, 0x21, 0x0F));
        let err = proto.send_command(&mock, command::REL).unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(Rejected::RelNeedsReading)),
            "{err:?}"
        );

        let mut rel = with_aux(vdc_manual(), 1.0, 0x40, 0x00);
        rel[1] |= 0x10;
        let (mut proto, mock) = proto_after(rel);
        let err = proto.send_command(&mock, command::REL).unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(Rejected::RelIsOn)),
            "{err:?}"
        );
        assert!(mock.written.borrow().is_empty());
    }

    #[test]
    fn peak_enters_or_leaves_by_the_peak_bit() {
        let vac = || payload(0x03, 2, 230.08, 0x20, 0x01);
        assert_eq!(sent(command::PEAK, vac()), build_abcd_le16(&[0x06, 0x01]));
        let mut peak = vac();
        peak[1] |= 0x40;
        assert_eq!(sent(command::PEAK, peak), build_abcd_le16(&[0x06, 0x00]));
    }

    #[test]
    fn the_meter_s_refusals() {
        for word in [b"ER", b"NO"] {
            let mock = MockTransport::new(vec![build_abcd_le16(&vdc_manual())]);
            let mut proto = Ut171Protocol::new();
            proto.request_measurement(&mock).unwrap();
            mock.push_reply(reply(word));
            let err = proto.send_command(&mock, command::HOLD).unwrap_err();
            assert!(
                matches!(&err, Error::CommandRejected(Rejected::ByMeter(Sent::Command(c))) if c == "hold"),
                "{err:?}"
            );
        }
    }

    /// `dmm-cli command` sends with no reading taken: one is read first.
    #[test]
    fn a_key_with_no_reading_yet_reads_one() {
        let mock = MockTransport::new(vec![build_abcd_le16(&payload(0x03, 2, 1.0, 0x20, 0x01))]);
        mock.push_reply(reply(b"OK"));
        let mut proto = Ut171Protocol::new();
        proto.send_command(&mock, command::PEAK).unwrap();
        assert_eq!(mock.written.borrow()[0], build_abcd_le16(&[0x06, 0x01]));
    }

    /// The context keys' functions, against the manual's lists.
    #[test]
    fn the_context_keys_apply_where_the_manual_says() {
        let applies = |key: &MeterKey, mode: u8| {
            (key.applies)(&parse::parse_measurement(&payload(mode, 1, 1.0, 0x10, 0x00)).unwrap())
        };
        let hz: Vec<u8> = (0x01..=0x1F).filter(|&m| applies(&HZ_PERCENT, m)).collect();
        assert_eq!(
            hz,
            [
                0x01, 0x03, 0x04, 0x06, 0x07, 0x0F, 0x12, 0x13, 0x15, 0x16, 0x18, 0x19, 0x1C, 0x1E
            ]
        );
        assert!((0x01..=0x1F).all(|m| applies(&STORE, m)));
    }
}
