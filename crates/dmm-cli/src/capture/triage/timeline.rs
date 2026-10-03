//! `--timeline`: each step's frames in order as the protocol decoded them,
//! the keys the capture pressed in place, repeats folded.

use super::decode::{Event, EventKind, Wire};
use super::{Triage, describe};
use crate::capture::report::{FrameDir, FrameRecord, SampleData, hex_string};
use std::collections::HashSet;

/// The timelines of the steps named in `only`, or of every step when it is
/// empty, after the init and detection frames as recorded.
pub(super) fn render(triage: &Triage, only: &[String]) -> String {
    let mut out = String::from("\nTimeline\n");
    if only.is_empty() {
        raw("init frames", &triage.report.init_frames, &mut out);
        if let Some(d) = &triage.report.detection {
            raw("detection frames", &d.frames, &mut out);
        }
    }
    for step in &triage.steps {
        if !only.is_empty() && !only.contains(&step.result.id) {
            continue;
        }
        out.push_str(&format!("  {}\n", step.result.id));
        match &step.wire {
            Some(wire) => {
                for line in lines(wire) {
                    out.push_str(&format!("    {line}\n"));
                }
            }
            None if step.result.frames.is_empty() => out.push_str("    no frames\n"),
            None => out.push_str("    not decoded: no device\n"),
        }
    }
    out
}

/// Frames no step owns, as the report holds them.
fn raw(title: &str, frames: &[FrameRecord], out: &mut String) {
    let Some(t0) = frames.first().map(|f| f.at_ms) else {
        return;
    };
    out.push_str(&format!("  {title}\n"));
    for f in frames {
        let dir = match f.dir {
            FrameDir::Tx => "TX",
            FrameDir::Rx => "RX",
        };
        let what = match f.baud {
            Some(baud) => format!("baud {baud}"),
            None => f.hex.clone(),
        };
        out.push_str(&format!(
            "    {} {dir} {what}\n",
            offset(f.at_ms.saturating_sub(t0))
        ));
    }
}

fn offset(ms: u64) -> String {
    format!("{ms:>+8} ms")
}

/// What happened, in wire order.
enum Entry<'a> {
    Key(usize),
    /// A request the protocol wrote, shown only between a key and the next
    /// reading: whether it went out before the key's reply is what decides
    /// a read-back (issue #20).
    Poll(usize),
    Unread(usize),
    /// Received bytes no reading ended in: a key's acknowledgement, a frame
    /// the parser skipped, or the start of the next reading.
    Passed(usize),
    Event(&'a Event),
}

/// Consecutive identical readings, printed as one line.
struct Run {
    text: String,
    notes: Vec<String>,
    first: String,
    last: String,
    count: usize,
}

impl Run {
    fn flush(run: &mut Option<Run>, out: &mut Vec<String>) {
        let Some(run) = run.take() else {
            return;
        };
        out.push(if run.count == 1 {
            format!("{} {}", run.first, run.text)
        } else {
            format!(
                "{} {} \u{d7}{} (to {})",
                run.first,
                run.text,
                run.count,
                run.last.trim_start()
            )
        });
        out.extend(run.notes);
    }
}

fn lines(wire: &Wire) -> Vec<String> {
    let t0 = wire.records.first().map_or(0, |r| r.at_ms);
    let at = |record: usize| offset(wire.at_ms(record).saturating_sub(t0));
    let hex = hex_string;
    let indent = " ".repeat(11);

    // Keyed by record, the key or unread reply before the event its bytes
    // ended in. An event with no record came before any bytes (init) or
    // after them all.
    let mut entries: Vec<((usize, u8), Entry)> = Vec::new();
    entries.extend(wire.commands.iter().map(|&i| ((i, 0), Entry::Key(i))));
    entries.extend(wire.polls.iter().map(|&i| ((i, 0), Entry::Poll(i))));
    entries.extend(wire.unread.iter().map(|&i| ((i, 1), Entry::Unread(i))));
    // Only where records hold whole frames: a CP2110 delivering one byte
    // per record would list every byte but each frame's last.
    let stamped: HashSet<usize> = wire.events.iter().filter_map(|e| e.record).collect();
    let unread: HashSet<usize> = wire.unread.iter().copied().collect();
    let passed: Vec<usize> = (0..wire.records.len())
        .filter(|i| {
            wire.records[*i].dir == FrameDir::Rx && !stamped.contains(i) && !unread.contains(i)
        })
        .collect();
    if passed.len() <= stamped.len() {
        entries.extend(passed.into_iter().map(|i| ((i, 1), Entry::Passed(i))));
    }
    for e in &wire.events {
        let key = match (e.record, &e.kind) {
            (Some(r), _) => r,
            (None, EventKind::End) => usize::MAX,
            (None, _) => 0,
        };
        entries.push(((key, 2), Entry::Event(e)));
    }
    entries.sort_by_key(|(key, _)| *key);

    // A line every event logged ("sending measurement request") says
    // nothing about any of them.
    let everywhere: Vec<&String> = match wire.events.split_first() {
        Some((first, rest)) if rest.len() >= 2 => first
            .logs
            .iter()
            .filter(|l| rest.iter().all(|e| e.logs.contains(l)))
            .collect(),
        _ => Vec::new(),
    };

    let mut out = Vec::new();
    let mut run: Option<Run> = None;
    let mut after_key = false;
    for (_, entry) in entries {
        let line =
            |i: usize, what: &str| format!("{} {what} {}", at(i), hex(&wire.records[i].bytes));
        let event = match entry {
            Entry::Key(i) => {
                let reply = super::reply_delay(wire, i)
                    .map_or(String::new(), |d| format!(" (bytes back after {d} ms)"));
                Run::flush(&mut run, &mut out);
                out.push(format!("{}{reply}", line(i, "TX key")));
                after_key = true;
                continue;
            }
            Entry::Poll(i) => {
                if after_key {
                    Run::flush(&mut run, &mut out);
                    out.push(line(i, "TX request"));
                }
                continue;
            }
            Entry::Unread(i) => {
                Run::flush(&mut run, &mut out);
                out.push(line(i, "RX unread"));
                continue;
            }
            Entry::Passed(i) => {
                Run::flush(&mut run, &mut out);
                out.push(line(i, "RX no reading"));
                continue;
            }
            Entry::Event(e) => e,
        };
        let when = event.record.map_or_else(|| indent.clone(), at);
        let text = match &event.kind {
            EventKind::Reading(m) => {
                after_key = false;
                let s = SampleData::from_measurement(m);
                format!("RX {} [{}]", describe(&s), s.raw_hex)
            }
            EventKind::Error(e) => format!("error: {e}"),
            EventKind::Timeout if wire.polled => "no reply to the request".to_string(),
            EventKind::Timeout => "no reading".to_string(),
            EventKind::End => "end of frames".to_string(),
        };
        let notes: Vec<String> = event
            .reports
            .iter()
            .map(|r| format!("{indent}   unrecognised: {r}"))
            .chain(
                event
                    .logs
                    .iter()
                    .filter(|l| !everywhere.contains(l))
                    .map(|l| format!("{indent}   {l}")),
            )
            .collect();
        if let Some(r) = &mut run
            && r.text == text
            && r.notes == notes
        {
            r.count += 1;
            r.last = when;
            continue;
        }
        Run::flush(&mut run, &mut out);
        run = Some(Run {
            text,
            notes,
            first: when.clone(),
            last: when,
            count: 1,
        });
    }
    Run::flush(&mut run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::super::decode::{decode, tests::frames};
    use super::*;
    use dmm_lib::protocol::registry::find_device;
    use dmm_lib::transport::Link;

    /// The request that went out before the key's reply came back is shown,
    /// the reply too, and identical readings fold into one line.
    #[test]
    fn a_key_shows_the_request_that_overtook_its_reply() {
        let reading = "AB CD 10 02 30 2D 30 2E 30 30 36 35 00 00 30 30 30 03 A0";
        let recorded = frames(&[
            (0, "tx", "AB CD 03 5E 01 D9"),
            (80, "rx", reading),
            (100, "tx", "AB CD 03 5E 01 D9"),
            (180, "rx", reading),
            (200, "tx", "AB CD 03 4A 01 C5"),
            (400, "tx", "AB CD 03 5E 01 D9"),
            (416, "rx", "AB CD 04 FF 00 02 7B"),
            (480, "rx", reading),
        ]);
        let wire = decode(
            &recorded,
            find_device("ut61eplus").unwrap(),
            Some(Link::UsbCable),
        );
        let lines: Vec<String> = lines(&wire).iter().map(|l| l.trim().to_string()).collect();
        let dcv = "RX DC V 2.2V: -0.0065 V [AUTO] [02 30 2D 30 2E 30 30 36 35 00 00 30 30 30]";
        assert_eq!(
            lines[..5],
            [
                format!("+80 ms {dcv} \u{d7}2 (to +180 ms)"),
                "+200 ms TX key AB CD 03 4A 01 C5 (bytes back after 216 ms)".to_string(),
                "+400 ms TX request AB CD 03 5E 01 D9".to_string(),
                "+416 ms RX no reading AB CD 04 FF 00 02 7B".to_string(),
                format!("+480 ms {dcv}"),
            ],
            "{lines:#?}"
        );
    }
}
