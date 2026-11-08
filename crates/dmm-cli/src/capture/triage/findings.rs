//! The findings: what in a report needs a maintainer's look — readings this
//! build parses differently, an LCD the operator corrected, commands that
//! changed nothing, missed expectations and stale starts.

use super::*;

impl Triage<'_> {
    pub(super) fn findings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.report.no_terminal {
            out.push(
                "[warn] all or part of this run had no terminal: check which readings \
                 were confirmed at the meter, and the detection check"
                    .to_string(),
            );
        }
        for step in &self.steps {
            let r = step.result;
            match r.status {
                StepStatus::Error | StepStatus::Timeout => out.push(format!(
                    "[{}] {}: {}",
                    if r.status == StepStatus::Error {
                        "error"
                    } else {
                        "timeout"
                    },
                    r.id,
                    r.error.as_deref().unwrap_or("no reason recorded")
                )),
                _ if r.needs_attention => {
                    out.push(format!("[attention] {}: flagged by the capture", r.id))
                }
                _ => {}
            }
        }
        for step in &self.steps {
            if step.result.confirmed == Some(false) {
                out.push(lcd_finding(step));
            }
        }
        out.extend(self.stale_starts());
        out.extend(self.commands_without_effect());
        out.extend(self.expectations());
        if let Some(line) = self.reparse_summary() {
            out.push(line);
        }
        for step in &self.steps {
            let Some(wire) = &step.wire else { continue };
            let errors = wire.events.iter().filter_map(|e| match &e.kind {
                EventKind::Error(text) => Some(text.clone()),
                EventKind::Timeout if wire.polled => Some("a request went unanswered".to_string()),
                EventKind::Timeout => Some("a gap with no reading".to_string()),
                _ => None,
            });
            for (text, n) in tally(errors) {
                out.push(format!("[wire] {}: {n}\u{d7} {text}", step.id()));
            }
        }
        for step in &self.steps {
            let wire_reports = step
                .wire
                .iter()
                .flat_map(|w| &w.events)
                .flat_map(|e| e.reports.iter().cloned());
            let reports = step.unrecognised.iter().cloned().chain(wire_reports);
            for (text, n) in tally(reports) {
                out.push(format!("[unrecognised] {}: {n}\u{d7} {text}", step.id()));
            }
        }
        out
    }

    /// One line for every sample this build reads differently, naming the
    /// fields: a report from before a parser fix differs on most steps, and
    /// the details have a section of their own.
    fn reparse_summary(&self) -> Option<String> {
        let mut steps = 0;
        let mut fields: Vec<(&str, usize)> = Vec::new();
        for step in &self.steps {
            let mut differs = false;
            for (recorded, parsed) in step.result.samples.iter().zip(&step.parsed) {
                let names = match parsed {
                    Ok(m) => differences(recorded, &SampleData::from_measurement(m))
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect(),
                    Err(_) => vec!["no longer parses"],
                };
                differs |= !names.is_empty();
                for name in names {
                    match fields.iter_mut().find(|(f, _)| *f == name) {
                        Some((_, n)) => *n += 1,
                        None => fields.push((name, 1)),
                    }
                }
            }
            steps += usize::from(differs);
        }
        (steps > 0).then(|| {
            let fields: Vec<String> = fields
                .iter()
                .map(|(f, n)| format!("{f} in {n} samples"))
                .collect();
            format!(
                "[reparse] {steps} steps read differently by this build: {}",
                fields.join(", ")
            )
        })
    }

    /// Captured steps whose key left no state the readings before it had
    /// not shown. A step's own command is its last key, after any mode
    /// switch the tool made first, judged as this build's capture judges a
    /// key ([`StateWatcher::for_command`]): on a ZT-5B two keys that did
    /// nothing passed on a −0.6 mA flicker (issue #31). A sub-step's presses
    /// run from its first key to its last, and the tool filed its sample as
    /// soon as the setting showed, often on a blank display, so only its
    /// last reading is asked for a state the readings before had not shown.
    fn commands_without_effect(&self) -> Vec<String> {
        let mut out = Vec::new();
        for step in self.steps.iter().filter(|s| s.captured()) {
            let definition = self.definition(step.id());
            let sub_step = step.is_sub_step();
            if !sub_step && definition.is_none_or(|d| d.command.is_none()) {
                continue;
            }
            let Some(wire) = &step.wire else { continue };
            let (Some(&first), Some(&last)) = (wire.commands.first(), wire.commands.last()) else {
                if step.result.frames_dropped > 0 {
                    out.push(format!(
                        "[command] {}: its key is not in the frames the report kept",
                        step.id()
                    ));
                }
                continue;
            };
            let from = if sub_step { first } else { last };
            let dated: Vec<(usize, &Measurement)> =
                wire.readings().filter_map(|(r, m)| Some((r?, m))).collect();
            // The readings just before the key, as many as the capture took.
            let earlier: Vec<Measurement> = dated
                .iter()
                .filter(|(r, _)| *r < from)
                .map(|(_, m)| (*m).clone())
                .collect();
            let before = &earlier[earlier.len().saturating_sub(STABLE_FRAMES)..];
            if before.is_empty() {
                // Nothing to measure the key against: the cap trimmed the
                // readings before it, or none decoded. A sub-step's key
                // opens its frames, and its setting is judged by its id.
                if !sub_step {
                    out.push(format!(
                        "[command] {}: no reading before its key was kept, so it is not judged",
                        step.id()
                    ));
                }
                continue;
            }
            // A `--sniff` run trusted no parse: the capture judged the key by
            // the payload bytes, with no expectation.
            let reads_parse = self.report.tier != Some(Tier::Sniff);
            let expect = if sub_step || !reads_parse {
                None
            } else {
                definition.and_then(|d| d.expect)
            };
            let after: Vec<&Measurement> = dated
                .iter()
                .filter(|(r, _)| *r > last)
                .map(|(_, m)| *m)
                .collect();
            let took = if sub_step {
                let seen: Vec<State> = before.iter().map(State::of).collect();
                after.last().is_some_and(|m| !seen.contains(&State::of(m)))
            } else {
                let mut watcher = StateWatcher::for_command(expect, before, reads_parse);
                after.iter().any(|m| watcher.feed(m) == Verdict::Ready)
            };
            if took {
                continue;
            }
            let shows = after
                .last()
                .map(|m| describe(&SampleData::from_measurement(m)))
                .unwrap_or_else(|| "no reading".to_string());
            out.push(format!(
                "[command] {}: no new state held after the key; the meter shows {shows}",
                step.id()
            ));
        }
        out
    }

    /// Captured steps whose last sample this build's step would not accept,
    /// and a step this build no longer has. A `mode:X` or `range:X`
    /// sub-step expects X; other sub-steps are not judged, as a setting can
    /// rename the mode (a UT181A's REL makes "nS" "nS REL").
    fn expectations(&self) -> Vec<String> {
        if self.definitions.is_none() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for step in self.steps.iter().filter(|s| s.captured()) {
            let id = step.id();
            if id.starts_with(crate::capture::FREEFORM_STEP_ID) {
                continue;
            }
            let Some(definition) = self.definition(id) else {
                out.push(format!("[expect] {id}: this build has no such step"));
                continue;
            };
            let Some(Ok(last)) = step.parsed.last() else {
                continue;
            };
            let verdict = match id.split_once('/').map(|(_, sub)| sub.split_once(':')) {
                Some(Some(("mode", mode))) => {
                    (last.mode != mode).then(|| format!("mode is {:?}, want {mode:?}", last.mode))
                }
                // The id holds the choice's label: "Auto", or the rung's.
                Some(Some(("range", "Auto"))) => Expect::new()
                    .range(RangeExpect::Auto)
                    .check(last)
                    .err()
                    .map(|e| e.to_string()),
                Some(Some(("range", range))) => (last.range_label != range)
                    .then(|| format!("range is {:?}, want {range:?}", last.range_label)),
                Some(_) => None,
                None => definition
                    .expect
                    .and_then(|e| e.check(last).err())
                    .map(|e| e.to_string()),
            };
            if let Some(reason) = verdict {
                out.push(format!(
                    "[expect] {id}: {reason} (this build's step): {}",
                    describe(&SampleData::from_measurement(last))
                ));
            }
        }
        out
    }

    /// Steps whose first sample shows the state the step before them ended
    /// in, as if the operator's action never reached the meter: the UT804's
    /// `rel` filed what `max_min` left (issue #16).
    ///
    /// "Before" is the previous step the operator ran: sub-steps are the
    /// tool's, and it puts the meter back where their base step was. Steps
    /// the tool commanded are judged on their command, and a step at the
    /// dial position the previous one ended at shows nothing arriving
    /// whatever was done ([`enter_only`]).
    fn stale_starts(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut previous: Option<&StepView> = None;
        for step in self.steps.iter().filter(|s| !s.is_sub_step()) {
            let prev = previous.replace(step);
            let Some(prev) = prev.filter(|p| p.captured()) else {
                continue;
            };
            // Without its definition, a key step or a step at the previous
            // one's dial position cannot be told apart: a plan run triaged
            // without its plan, a freeform step, a step this build dropped.
            let Some(definition) = self.definition(step.id()) else {
                continue;
            };
            if !step.captured() || definition.command.is_some() {
                continue;
            }
            // Out of order on one clock: a retake or a resume put them here.
            if let (Some(p), Some(s)) = (prev.clock(), step.clock())
                && p.1 > s.0
            {
                continue;
            }
            let (Some(Ok(last)), Some(Ok(first))) = (prev.parsed.last(), step.parsed.first())
            else {
                continue;
            };
            if enter_only(definition.expect, Some(last)) {
                continue;
            }
            // The sub-values' numbers too: a step that only changes a
            // secondary display (a 121GW's setup steps) did arrive.
            let sub_values = |m: &Measurement| -> Vec<String> {
                m.aux_values
                    .iter()
                    .map(|a| a.value_str().into_owned())
                    .collect()
            };
            if State::of(first) == State::of(last) && sub_values(first) == sub_values(last) {
                let digits = if first.display_raw == last.display_raw {
                    ", digits too"
                } else {
                    ""
                };
                out.push(format!(
                    "[stale] {}: starts in the state {} ended in{digits}: {}",
                    step.id(),
                    prev.id(),
                    describe(&SampleData::from_measurement(first))
                ));
            }
        }
        out
    }
}

/// The operator said the reading was wrong: what they saw beside what this
/// build makes of the step's last sample.
fn lcd_finding(step: &StepView) -> String {
    let id = step.id();
    let lcd = step.result.lcd.as_deref();
    let shown = lcd.map_or("(nothing typed)".to_string(), |t| format!("{t:?}"));
    let Some(reading) = step.readings().pop() else {
        return format!("[lcd] {id}: meter showed {shown}; no samples");
    };
    let tag = match lcd {
        Some(t) if same_words(t, &reading.summary()) => " (matches this build)",
        Some(t) if unit_only(t, &reading) => " (same digits, unit differs)",
        _ => "",
    };
    format!(
        "[lcd] {id}: meter showed {shown}; decoded {}{tag}",
        describe(&reading)
    )
}

/// Whether what the operator typed says what the reading says: the same
/// value and unit, and only flags the reading has. v0.6.0 reports filed
/// whatever the operator typed, a matching reading included, often without
/// the AUTO they took for granted.
fn same_words(typed: &str, summary: &str) -> bool {
    let (typed_main, typed_flags) = words_and_flags(typed);
    let (main, flags) = words_and_flags(summary);
    typed_main == main && typed_flags.iter().all(|f| flags.contains(f))
}

/// The words outside brackets, sorted, and the flag words inside them.
fn words_and_flags(text: &str) -> (Vec<&str>, Vec<&str>) {
    let (mut main, mut flags) = (Vec::new(), Vec::new());
    let mut inside = false;
    for part in text.split_inclusive(['[', ']']) {
        let words = part.trim_end_matches(['[', ']']).split_whitespace();
        if inside {
            flags.extend(words);
        } else {
            main.extend(words);
        }
        inside = part.ends_with('[');
    }
    main.sort_unstable();
    (main, flags)
}

/// Whether what the operator typed has the reading's digits and not its
/// unit: the 1000x mistake, a range labelled mV on a meter showing V
/// (issue #19).
fn unit_only(lcd: &str, reading: &SampleData) -> bool {
    let digits = |t: &str| {
        t.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .trim_start_matches('0')
            .to_string()
    };
    let want = digits(reading.shown());
    // The units typed: the letters of each word, "0.037V" and "0.037 V"
    // alike. Nothing typed is no disagreement.
    let (words, _) = words_and_flags(lcd);
    let units: Vec<&str> = words
        .iter()
        .flat_map(|w| w.split(|c: char| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')))
        .filter(|u| !u.is_empty())
        .collect();
    !want.is_empty()
        && digits(lcd) == want
        && !units.is_empty()
        && !units.contains(&reading.unit.as_str())
}

/// Where this build reads a step's samples differently from the build that
/// wrote the report, one line per distinct set of differences.
pub(super) fn reparse_findings(step: &StepView) -> Vec<String> {
    let total = step.parsed.len();
    let lines =
        step.result.samples.iter().zip(&step.parsed).filter_map(
            |(recorded, parsed)| match parsed {
                Ok(m) => {
                    let diffs = differences(recorded, &SampleData::from_measurement(m));
                    (!diffs.is_empty()).then(|| {
                        diffs
                            .into_iter()
                            .map(|(_, d)| d)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                }
                Err(e) => Some(format!("no longer parses ({e}): {}", recorded.raw_hex)),
            },
        );
    tally(lines)
        .into_iter()
        .map(|(line, n)| format!("{} ({n} of {total} samples): {line}", step.id()))
        .collect()
}

/// The fields that differ: each one's name, and `name "before" → "after"`.
fn differences(recorded: &SampleData, now: &SampleData) -> Vec<(&'static str, String)> {
    let mut diffs = Vec::new();
    let mut field = |name: &'static str, a: &str, b: &str| {
        if a != b {
            diffs.push((name, format!("{name} {a:?} \u{2192} {b:?}")));
        }
    };
    field("mode", &recorded.mode, &now.mode);
    field("mode byte", &recorded.mode_byte, &now.mode_byte);
    field("display", &recorded.display_raw, &now.display_raw);
    field("value", &recorded.value, &now.value);
    field("unit", &recorded.unit, &now.unit);
    field("range", &recorded.range_label, &now.range_label);
    field(
        "progress",
        &recorded.progress.to_string(),
        &now.progress.to_string(),
    );
    field(
        "flags",
        &flag_names(&StatusFlags::from(&recorded.flags)),
        &flag_names(&StatusFlags::from(&now.flags)),
    );
    field(
        "label",
        recorded.main_label.as_deref().unwrap_or(""),
        now.main_label.as_deref().unwrap_or(""),
    );
    // A UT61+ secondary display arrives with the next frame, which a
    // payload parsed on its own never has.
    if !now.aux.is_empty() {
        let aux = |s: &SampleData| {
            s.aux
                .iter()
                .map(|a| format!("{} {} {}", a.label, a.value, a.unit))
                .collect::<Vec<_>>()
                .join(", ")
        };
        field("sub-values", &aux(recorded), &aux(now));
    }
    diffs
}

fn flag_names(flags: &StatusFlags) -> String {
    flags.active().map(Flag::name).collect::<Vec<_>>().join(" ")
}
