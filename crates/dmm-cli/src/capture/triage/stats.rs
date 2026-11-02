//! The per-step summary and the run's stats: readings, reply delays and
//! what the wire carried.

use super::*;

impl Triage<'_> {
    pub(super) fn summary(&self, out: &mut String) {
        for step in &self.steps {
            let r = step.result;
            let status = format!("{:?}", r.status).to_lowercase();
            out.push_str(&format!(
                "  {} [{status}, {} samples, {} frames]\n",
                r.id,
                r.samples.len(),
                r.frames.len()
            ));
            for (text, n) in tally(step.readings().iter().map(describe)) {
                out.push_str(&format!("      {text} \u{d7}{n}\n"));
            }
            if let Some(wire) = &step.wire {
                out.push_str("    on the wire:\n");
                for line in wire_states(wire) {
                    out.push_str(&format!("      {line}\n"));
                }
            }
        }
    }

    pub(super) fn stats(&self, out: &mut String) {
        let readings: Vec<SampleData> =
            self.steps.iter().flat_map(StepView::all_readings).collect();
        if readings.is_empty() {
            out.push_str("  no readings\n");
            return;
        }
        let flags: Vec<StatusFlags> = readings
            .iter()
            .map(|s| StatusFlags::from(&s.flags))
            .collect();
        let names = |pick: &dyn Fn(Flag) -> bool| {
            let names: Vec<&str> = Flag::ALL
                .into_iter()
                .filter(|&f| pick(f))
                .map(Flag::name)
                .collect();
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        };
        out.push_str(&format!(
            "  flags never set: {}\n",
            names(&|f| flags.iter().all(|s| !s.get(f)))
        ));
        out.push_str(&format!(
            "  flags always set: {}\n",
            names(&|f| flags.iter().all(|s| s.get(f)))
        ));

        // Mode by mode, in the order the run met them.
        let mut modes: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
        for s in &readings {
            let i = match modes.iter().position(|(m, _, _)| *m == s.mode) {
                Some(i) => i,
                None => {
                    modes.push((s.mode.clone(), vec![], vec![]));
                    modes.len() - 1
                }
            };
            let (_, units, ranges) = &mut modes[i];
            if !units.contains(&s.unit) {
                units.push(s.unit.clone());
            }
            if !s.range_label.is_empty() && !ranges.contains(&s.range_label) {
                ranges.push(s.range_label.clone());
            }
        }
        for (mode, units, ranges) in modes {
            let ranges = if ranges.is_empty() {
                String::new()
            } else {
                format!("; ranges {}", ranges.join(" "))
            };
            let units: Vec<String> = units.into_iter().filter(|u| !u.is_empty()).collect();
            let units = if units.is_empty() {
                "no unit".to_string()
            } else {
                format!("units {}", units.join(" "))
            };
            out.push_str(&format!("  mode {mode}: {units}{ranges}\n"));
        }
        self.wire_stats(out);
    }

    /// Timing and shape of what came over the wire, from every step's
    /// decoded frames.
    fn wire_stats(&self, out: &mut String) {
        let wires: Vec<&Wire> = self.steps.iter().filter_map(|s| s.wire.as_ref()).collect();
        if wires.is_empty() {
            return;
        }
        let lengths = tally(
            wires
                .iter()
                .flat_map(|w| w.readings())
                .map(|(_, m)| m.raw_payload.len().to_string()),
        );
        let lengths: Vec<String> = lengths
            .iter()
            .map(|(len, n)| format!("{len} bytes \u{d7}{n}"))
            .collect();
        out.push_str(&format!("  payloads: {}\n", lengths.join(", ")));

        // Only between readings that each had a record to themselves: the
        // capture merged reads under 50 ms apart into one record, dated by
        // its first, so several packets in one say nothing of their spacing.
        let mut intervals = Vec::new();
        let mut poll_delays = Vec::new();
        let mut command_delays = Vec::new();
        let mut unanswered = 0;
        for wire in &wires {
            // In record order, so a record shared with another reading is a
            // neighbour's.
            let dated: Vec<usize> = wire.readings().filter_map(|(r, _)| r).collect();
            let alone = |i: usize| {
                (i == 0 || dated[i - 1] != dated[i])
                    && dated.get(i + 1).is_none_or(|next| *next != dated[i])
            };
            intervals.extend(
                (1..dated.len())
                    .filter(|&i| alone(i - 1) && alone(i))
                    .map(|i| {
                        wire.at_ms(dated[i])
                            .saturating_sub(wire.at_ms(dated[i - 1]))
                    }),
            );
            for &r in &dated {
                // The request this reading answered: the last one before it.
                let before = wire.polls.partition_point(|&p| p < r);
                if let Some(&poll) = before.checked_sub(1).and_then(|i| wire.polls.get(i)) {
                    poll_delays.push(wire.at_ms(r).saturating_sub(wire.at_ms(poll)));
                }
            }
            command_delays.extend(wire.commands.iter().filter_map(|&c| reply_delay(wire, c)));
            if wire.polled {
                unanswered += wire
                    .events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::Timeout))
                    .count();
            }
        }
        if let Some(line) = spread(&intervals) {
            out.push_str(&format!("  reading interval: {line}\n"));
        }
        if let Some(line) = spread(&poll_delays) {
            out.push_str(&format!("  request to reading: {line}\n"));
        }
        if unanswered > 0 {
            out.push_str(&format!("  requests unanswered: {unanswered}\n"));
        }
        let commands: usize = wires.iter().map(|w| w.commands.len()).sum();
        if commands > 0 {
            let replies = spread(&command_delays).unwrap_or_else(|| "none".to_string());
            out.push_str(&format!(
                "  keys sent: {commands}, first bytes back after: {replies}\n"
            ));
        }

        let logs = tally(
            wires
                .iter()
                .flat_map(|w| &w.events)
                .flat_map(|e| e.logs.iter().map(|l| log_kind(l))),
        );
        for (line, n) in logs {
            out.push_str(&format!("  log {n}\u{d7} {line}\n"));
        }
    }
}

/// How long after the key at `command` the next bytes came.
pub(super) fn reply_delay(wire: &Wire, command: usize) -> Option<u64> {
    let reply = wire.records[command + 1..]
        .iter()
        .find(|r| r.dir == crate::capture::report::FrameDir::Rx)?;
    Some(reply.at_ms.saturating_sub(wire.at_ms(command)))
}

/// Median, minimum and maximum of `ms`, and how many.
fn spread(ms: &[u64]) -> Option<String> {
    let mut sorted = ms.to_vec();
    sorted.sort_unstable();
    let median = *sorted.get(sorted.len() / 2)?;
    Some(format!(
        "median {median} ms, min {} ms, max {} ms over {}",
        sorted[0],
        sorted[sorted.len() - 1],
        sorted.len()
    ))
}

/// A log line with its numbers and byte lists blanked, so the lines one
/// kind of event logs count as one.
fn log_kind(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0;
    for c in line.chars() {
        match c {
            '[' => {
                depth += 1;
                if depth == 1 {
                    out.push_str("[\u{2026}]");
                }
            }
            ']' => depth = (depth - 1).max(0),
            _ if depth > 0 => {}
            c if c.is_ascii_digit() => {
                if !out.ends_with('#') {
                    out.push('#');
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// A step's decoded readings grouped by the state they show, with how many
/// and the first and last digits: the wire's readings run to hundreds.
fn wire_states(wire: &Wire) -> Vec<String> {
    let mut groups: Vec<(String, usize, String, String)> = Vec::new();
    for (_, m) in wire.readings() {
        let s = SampleData::from_measurement(m);
        let flags = StatusFlags::from(&s.flags).to_string();
        let range = if s.range_label.is_empty() {
            String::new()
        } else {
            format!(" {}", s.range_label)
        };
        let state = if flags.is_empty() {
            format!("{}{range}", s.mode)
        } else {
            format!("{}{range} [{flags}]", s.mode)
        };
        let shown = format!("{} {}", s.shown(), s.unit).trim_end().to_string();
        match groups.iter_mut().find(|(g, ..)| *g == state) {
            Some((_, n, _, last)) => {
                *n += 1;
                *last = shown;
            }
            None => groups.push((state, 1, shown.clone(), shown)),
        }
    }
    let mut lines: Vec<String> = groups
        .into_iter()
        .map(|(state, n, first, last)| {
            if first == last {
                format!("{state}: {first} \u{d7}{n}")
            } else {
                format!("{state}: {first} \u{2026} {last} \u{d7}{n}")
            }
        })
        .collect();
    let errors = wire
        .events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Error(_) | EventKind::Timeout))
        .count();
    if errors > 0 {
        lines.push(format!("errors \u{d7}{errors}"));
    }
    if lines.is_empty() {
        lines.push("nothing decoded".to_string());
    }
    lines
}
