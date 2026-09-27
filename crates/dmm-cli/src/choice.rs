//! How `set` reads the choice it was given: a label or a unique fragment of
//! one, typed on any keyboard, and the shortest fragment a listing offers.

use dmm_lib::protocol::Choice;

/// The shortest run of words from a choice's label that [`resolve_choice`]
/// maps back to that same choice — what the listing shows as the thing to type.
///
/// Runs are tried shortest first, measured in characters and, at equal length,
/// leftmost first. A label whose every fragment is shared with a longer label
/// ("V AC" beside "V AC Hz") has no shorter form: the whole label comes back,
/// which the resolver takes as an exact match.
pub(crate) fn shortest_fragment(choices: &[Choice], target: &Choice) -> String {
    let label = typeable(&target.label);
    let words: Vec<&str> = label.split_whitespace().collect();
    let mut runs: Vec<(usize, usize, String)> = Vec::new();
    for len in 1..=words.len() {
        for start in 0..=words.len() - len {
            let run = words[start..start + len].join(" ");
            runs.push((run.chars().count(), start, run));
        }
    }
    runs.sort_by_key(|(chars, start, _)| (*chars, *start));
    runs.into_iter()
        .map(|(_, _, run)| run)
        .find(|run| resolve_choice(choices, run).is_ok_and(|hit| hit.id == target.id))
        // Only reachable for a label the runs above cannot reproduce (empty,
        // or oddly spaced); the label itself is always an exact match.
        .unwrap_or(label)
}

/// A label as it can be typed on any keyboard: lower-case, with the symbols
/// the meters use spelled out (Ω → ohm, µ → u) or dropped (°). Both sides of
/// a match go through this, so "ohm" finds "Ω" and "temp c" finds "Temp °C".
fn typeable(label: &str) -> String {
    label
        .to_lowercase()
        .chars()
        .filter(|&c| c != '°')
        .map(|c| match c {
            // Both U+03A9 (Greek omega) and U+2126 (ohm sign) lower-case to ω.
            'ω' => "ohm".to_string(),
            'µ' | 'μ' => "u".to_string(),
            other => other.to_string(),
        })
        .collect()
}

/// A label or fragment as it has to be typed back on a shell command line:
/// bare when the shell would leave it alone, double-quoted otherwise.
pub(crate) fn quote_for_shell(text: &str) -> String {
    let bare = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if bare {
        text.to_string()
    } else {
        format!("\"{text}\"")
    }
}

/// Why a `mode` argument picked out no single choice.
#[derive(Debug)]
pub(crate) enum NoMatch<'a> {
    /// The input is a fragment of these labels, and equal to none of them.
    Ambiguous(Vec<&'a str>),
    /// The input is a fragment of no label at all.
    Unknown,
}

/// Resolve a `mode` argument against the choices the meter just reported: a
/// label, case-insensitively, or a fragment of exactly one of them.
///
/// An exact match wins outright — a label that is also a substring of longer
/// ones ("V AC" beside "V AC Hz") stays reachable by typing it in full.
pub(crate) fn resolve_choice<'a>(
    choices: &'a [Choice],
    input: &str,
) -> Result<&'a Choice, NoMatch<'a>> {
    let needle = typeable(input.trim());
    if needle.is_empty() {
        return Err(NoMatch::Unknown);
    }
    if let Some(exact) = choices.iter().find(|c| typeable(&c.label) == needle) {
        return Ok(exact);
    }
    let hits: Vec<_> = choices
        .iter()
        .filter(|c| typeable(&c.label).contains(&needle))
        .collect();
    match hits[..] {
        [one] => Ok(one),
        [] => Err(NoMatch::Unknown),
        _ => Err(NoMatch::Ambiguous(
            hits.iter().map(|c| c.label.as_ref()).collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{fake_mode_id, mode_choice};

    /// A user retypes what the listing printed, in whatever case they like.
    #[test]
    fn resolve_choice_matches_a_label_case_insensitively() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ];
        for input in ["V AC Hz", "v ac hz", "  V Ac hZ  "] {
            assert_eq!(
                resolve_choice(&choices, input).ok().map(|c| c.id),
                Some(0x1121),
                "{input}"
            );
        }
    }

    /// The symbols the meters print are not on a keyboard, so their spelled
    /// out forms match too.
    #[test]
    fn resolve_choice_accepts_typeable_spellings() {
        let choices = [
            mode_choice(0x06, "Ω", true),
            mode_choice(0x0C, "DC µA", false),
            mode_choice(0x14, "°C", false),
        ];
        for (input, id) in [("ohm", 0x06), ("Ω", 0x06), ("dc ua", 0x0C), ("c", 0x14)] {
            assert_eq!(
                resolve_choice(&choices, input).ok().map(|c| c.id),
                Some(id),
                "{input}"
            );
        }
    }

    /// Typing a whole label is tedious, so a fragment of exactly one of them
    /// is enough.
    #[test]
    fn resolve_choice_matches_a_unique_label_fragment() {
        let temps = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
        ];
        assert_eq!(
            resolve_choice(&temps, "t1-t2").ok().map(|c| c.id),
            Some(0x4231)
        );
        let volts = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_choice(&volts, "hz").ok().map(|c| c.id),
            Some(0x1121)
        );
    }

    /// A label that is also a fragment of longer ones stays reachable: typed
    /// in full it is an exact match, and an exact match wins outright.
    #[test]
    fn resolve_choice_prefers_an_exact_label_over_a_fragment() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_choice(&choices, "v ac").ok().map(|c| c.id),
            Some(0x1111)
        );
    }

    /// A fragment of several labels picks none of them, and says which ones
    /// it was torn between — that is what the user has to narrow down.
    #[test]
    fn resolve_choice_reports_an_ambiguous_fragment() {
        let choices = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
            mode_choice(0x4241, "Temp °C T2-T1", false),
        ];
        match resolve_choice(&choices, "temp") {
            Err(NoMatch::Ambiguous(labels)) => assert_eq!(
                labels,
                ["Temp °C", "Temp °C T2", "Temp °C T1-T2", "Temp °C T2-T1"]
            ),
            other => panic!("expected an ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn resolve_choice_rejects_anything_else() {
        let choices = [mode_choice(0x1111, "V AC", true)];
        // Another mode's label, the id the listing no longer prints, and an
        // empty argument — which matches nothing rather than everything.
        for input in ["V DC", "0x1111", "4369", "", "   "] {
            assert!(
                matches!(resolve_choice(&choices, input), Err(NoMatch::Unknown)),
                "{input}"
            );
        }
    }

    /// Label groups a meter really offers, each with the fragment the listing
    /// is expected to print beside every one of its labels: the mock's
    /// temperature dial, the UT181A's V AC and temperature dials, and the
    /// mock's AC V dial.
    fn fragment_cases() -> [(&'static [&'static str], &'static [&'static str]); 5] {
        [
            (
                &[
                    "Temp °C",
                    "Temp °C T1 (T2)",
                    "Temp °C T1-T2",
                    "Temp °C T2-T1",
                ],
                &["temp c", "(t2)", "t1-t2", "t2-t1"],
            ),
            (
                &[
                    "V AC",
                    "V AC Hz",
                    "V AC Peak",
                    "V AC LPF",
                    "V AC dBV",
                    "V AC dBm",
                ],
                &["v ac", "hz", "peak", "lpf", "dbv", "dbm"],
            ),
            (
                &["°C", "°C T2", "°C T1-T2", "°C T2-T1"],
                &["c", "c t2", "t1-t2", "t2-t1"],
            ),
            (&["AC V", "AC V Hz"], &["ac v", "hz"]),
            (
                &["Ω", "Continuity", "Diode", "Capacitance"],
                &["ohm", "continuity", "diode", "capacitance"],
            ),
        ]
    }

    fn fragment_choices(labels: &[&'static str]) -> Vec<Choice> {
        labels
            .iter()
            .enumerate()
            .map(|(i, label)| mode_choice(fake_mode_id(i), label, i == 0))
            .collect()
    }

    /// What the second column of the listing says, group by group.
    #[test]
    fn shortest_fragment_is_the_least_that_picks_a_mode_out() {
        for (labels, expected) in fragment_cases() {
            let choices = fragment_choices(labels);
            let fragments: Vec<_> = choices
                .iter()
                .map(|c| shortest_fragment(&choices, c))
                .collect();
            assert_eq!(fragments, expected, "{labels:?}");
        }
    }

    /// The column is only useful if what it prints comes back to the line it
    /// is printed on — including for a label every fragment of which is
    /// shared, where the whole label is the answer.
    #[test]
    fn shortest_fragment_always_selects_its_own_mode() {
        // The groups above, plus the shorter lists the resolver tests use.
        let extra: [&[&str]; 3] = [
            &["V AC", "V AC Hz"],
            &["Temp °C", "Temp °C T2", "Temp °C T1-T2"],
            &["V AC"],
        ];
        for labels in fragment_cases().iter().map(|(l, _)| *l).chain(extra) {
            let choices = fragment_choices(labels);
            for c in &choices {
                let fragment = shortest_fragment(&choices, c);
                assert_eq!(
                    resolve_choice(&choices, &fragment).ok().map(|hit| hit.id),
                    Some(c.id),
                    "{fragment:?} for {}",
                    c.label
                );
            }
        }
    }

    /// A fragment is meant to be typed back, so anything a shell would split
    /// or mangle is printed quoted.
    #[test]
    fn quote_for_shell_quotes_what_a_shell_would_not_take_bare() {
        for bare in ["hz", "t1-t2", "dbv", "peak_2", "1.5"] {
            assert_eq!(quote_for_shell(bare), bare, "{bare}");
        }
        for quoted in ["ac v", "(t2)", "temp °c", "°c", "ac+dc", ""] {
            assert_eq!(quote_for_shell(quoted), format!("\"{quoted}\""), "{quoted}");
        }
    }
}
