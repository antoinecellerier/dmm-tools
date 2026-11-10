//! `get` and `set`: what the meter's settings can be switched to from where
//! it sits, as a listing or JSON, and the switch itself, confirmed from the
//! readings that follow.
//!
//! The prose is in the run's language (`cli/settings.ftl`). What the user
//! types back into `set` stays as typed: the setting names, and a listing's
//! choices with the fragment beside each, which `set` matches against the
//! labels as the library spells them. The JSON stays as it is too.

use crate::choice::{NoMatch, quote_for_shell, resolve_choice, shortest_fragment};
use crate::cli::{SettingArg, SettingsFormat};
use crate::l10n::note_label;
use crate::open::{open_any, prose_name};
use console::style;
use dmm_lib::error::{ErrorKind, Rejected};
use dmm_lib::protocol::registry::Selection;
use dmm_lib::protocol::{AUTO_RANGE_ID, Choice, Setting};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use std::time::Duration;

/// How long to wait for a switched setting to show up in the measurement
/// stream. The meter acknowledges the command before the frame carrying the
/// new value arrives, so "accepted" and "switched" are two separate answers.
const SWITCH_TIMEOUT: Duration = Duration::from_secs(2);

/// Gap between polls while waiting for that frame.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Shown in place of the live value when the meter is on something the list
/// does not name — a manual range outside the family's ladder, say.
const UNKNOWN_LIVE: &str = "?";

/// The kind of library text `setting`'s choices are, for the lookups that
/// put one in a sentence.
fn choice_kind(setting: Setting) -> TextKind {
    match setting {
        Setting::Mode => TextKind::Mode,
        Setting::Range => TextKind::Range,
        _ => TextKind::Toggle,
    }
}

/// A choice of `setting` as a sentence names it, in the run's language.
fn choice_text<'a>(
    l: &'a Localizer,
    setting: Setting,
    label: &'a str,
) -> std::borrow::Cow<'a, str> {
    l.label(choice_kind(setting), label)
}

/// Whether the meter has a value to switch *to* from where it sits.
///
/// A single choice is the live value on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to list, and nothing to switch.
fn offers_a_switch(choices: &[Choice]) -> bool {
    choices.len() > 1
}

/// The value the meter sits on, or [`UNKNOWN_LIVE`] when the list marks none.
fn live_label(choices: &[Choice]) -> &str {
    choices
        .iter()
        .find(|c| c.current)
        .map_or(UNKNOWN_LIVE, |c| c.label.as_ref())
}

/// The rung autoranging picked, for the note beside an Auto row — "Auto" on
/// its own never says what the meter settled on. `None` unless this is the
/// range list and the meter is autoranging.
fn auto_range_rung<'a>(
    setting: Setting,
    choices: &[Choice],
    reading: &'a dmm_lib::measurement::Measurement,
) -> Option<&'a str> {
    (setting == Setting::Range && choices.iter().any(|c| c.current && c.id == AUTO_RANGE_ID))
        .then(|| reading.range_label.as_ref())
}

/// The note a setting — or, with `None`, the whole meter — with nothing to
/// switch prints before exiting 0. Only the dial changes what modes and
/// ranges a position offers; a button setting the meter lacks is not
/// something the dial can fix, so it gets no such advice.
fn print_nothing_to_switch(model_name: &'static str, setting: Option<Setting>, mode: &str) {
    let l = crate::l10n::get();
    let note = tr!(
        l,
        "settings-nothing-to-switch",
        setting = setting.map_or("all", Setting::name),
        meter = prose_name(model_name),
        mode = l.label(TextKind::Mode, mode),
    );
    eprintln!("{} {note}", note_label());
}

/// Which listing a [`print_tip`] closes.
#[derive(Clone, Copy)]
enum Tip {
    /// The listing of every setting, with no fragments to type.
    ByName,
    /// One setting's listing, its fragments in the right column.
    RightColumn,
}

/// The dim line under a listing. Both listings end in one, so both name a
/// switch that can actually be made — and preferably one that would change
/// something.
fn print_tip(tip: Tip, setting: Setting, choices: &[Choice]) {
    let l = crate::l10n::get();
    let example = choices.iter().find(|c| !c.current).unwrap_or(&choices[0]);
    let command = format!(
        "dmm-cli set {} {}",
        setting.name(),
        quote_for_shell(&shortest_fragment(choices, example))
    );
    let line = match tip {
        Tip::ByName => tr!(l, "settings-tip-by-name", command = command),
        Tip::RightColumn => tr!(l, "settings-tip-right-column", command = command),
    };
    eprintln!("\n{}", style(line).dim());
}

/// List what one setting, or every setting, reaches from where the meter sits.
///
/// Both need a reading first: the choices are relative to what the meter is
/// measuring now, so there is nothing to list until one frame has arrived.
pub(crate) fn cmd_get(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    setting: Option<SettingArg>,
    format: SettingsFormat,
    mock_mode: Option<dmm_lib::mock::MockMode>,
) -> Result<(), crate::l10n::Failure> {
    let setting = setting.map(Setting::from);
    let mut dmm = open_any(selection, opts, mock_mode)?;
    run_get(&mut dmm, setting, format)
}

/// Switch one setting, or list what it reaches when no choice was named.
pub(crate) fn cmd_set(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    setting: SettingArg,
    choice: Option<String>,
    mock_mode: Option<dmm_lib::mock::MockMode>,
) -> Result<(), crate::l10n::Failure> {
    let setting = Setting::from(setting);
    let mut dmm = open_any(selection, opts, mock_mode)?;
    run_set(&mut dmm, setting, choice)
}

fn run_get<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    setting: Option<Setting>,
    format: SettingsFormat,
) -> Result<(), crate::l10n::Failure> {
    let model_name = crate::open::profile_name(dmm.profile());
    let reading = dmm.request_measurement()?;

    let Some(setting) = setting else {
        let offered = offered_settings(dmm, &reading);
        return match format {
            SettingsFormat::Json => print_json(settings_json(model_name, &reading, &offered)),
            SettingsFormat::Text => {
                if offered.is_empty() {
                    print_nothing_to_switch(model_name, None, &reading.mode);
                    return Ok(());
                }
                print_settings_table(model_name, &reading, &offered);
                let (setting, choices) = offered
                    .iter()
                    .find(|(_, choices)| choices.iter().any(|c| !c.current))
                    .unwrap_or(&offered[0]);
                print_tip(Tip::ByName, *setting, choices);
                Ok(())
            }
        };
    };

    let choices = dmm.choices(setting, &reading);
    match format {
        SettingsFormat::Json => {
            print_json(one_setting_json(model_name, &reading, setting, &choices))
        }
        SettingsFormat::Text => {
            if !offers_a_switch(&choices) {
                print_nothing_to_switch(model_name, Some(setting), &reading.mode);
                return Ok(());
            }
            print_choices(model_name, setting, &reading, &choices);
            print_tip(Tip::RightColumn, setting, &choices);
            Ok(())
        }
    }
}

fn run_set<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    setting: Setting,
    choice: Option<String>,
) -> Result<(), crate::l10n::Failure> {
    run_set_within(dmm, setting, choice, SWITCH_TIMEOUT)
}

/// [`run_set`], giving up on the switch after `switch_timeout`: a test of a
/// meter that never switches has no reason to sit out the real deadline.
fn run_set_within<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    setting: Setting,
    choice: Option<String>,
    switch_timeout: Duration,
) -> Result<(), crate::l10n::Failure> {
    let model_name = crate::open::profile_name(dmm.profile());
    let reading = dmm.request_measurement()?;
    let choices = dmm.choices(setting, &reading);

    if !offers_a_switch(&choices) {
        print_nothing_to_switch(model_name, Some(setting), &reading.mode);
        return Ok(());
    }

    let Some(input) = choice else {
        print_choices(model_name, setting, &reading, &choices);
        print_tip(Tip::RightColumn, setting, &choices);
        return Ok(());
    };

    let target = match resolve_choice(&choices, &input) {
        Ok(target) => target,
        Err(NoMatch::Ambiguous(labels)) => {
            let labels = labels.into_iter().map(String::from).collect();
            return Err(SetFailed::Ambiguous {
                setting,
                input,
                labels,
            }
            .into());
        }
        Err(NoMatch::Unknown) => {
            print_choices(model_name, setting, &reading, &choices);
            return Err(SetFailed::Unknown { setting, input }.into());
        }
    };
    let (id, label) = (target.id, target.label.to_string());

    if target.current {
        println!(
            "{}",
            style(switch_message(
                false,
                setting,
                id,
                &label,
                &reading.range_label
            ))
            .green()
        );
        return Ok(());
    }
    // Everything below reads from `choices` again, so drop the borrow.
    let mut live = choices
        .iter()
        .find(|c| c.current)
        .map(|c| c.label.to_string());

    if let Err(e) = dmm.select(setting, id) {
        // A refusal is the meter answering, not a fault: say so, and say what
        // the user can do about it.
        return Err(match e {
            dmm_lib::error::Error::CommandRejected(reason) => SetFailed::Refused {
                setting,
                choice: label,
                reason,
            }
            .into(),
            other => other.into(),
        });
    }

    let started = std::time::Instant::now();
    loop {
        match dmm.request_measurement() {
            Ok(reading) => {
                let choices = dmm.choices(setting, &reading);
                if choices.iter().any(|c| c.id == id && c.current) {
                    println!(
                        "{}",
                        style(switch_message(
                            true,
                            setting,
                            id,
                            &label,
                            &reading.range_label
                        ))
                        .green()
                    );
                    return Ok(());
                }
                if let Some(c) = choices.iter().find(|c| c.current) {
                    live = Some(c.label.to_string());
                }
            }
            // The frame straddling the switch can be unreadable, and the meter
            // can go quiet across it altogether — the vendor app sleeps 100 ms
            // after every SET_MODE. Neither ends the wait: the next frame
            // parses, and the deadline below is what gives up. Anything
            // else is a real fault.
            Err(e) if matches!(e.kind(), ErrorKind::Protocol | ErrorKind::Timeout) => {
                log::warn!("waiting for the {setting} switch: {e}");
            }
            Err(e) => return Err(e.into()),
        }
        if std::time::Instant::now()
            .checked_duration_since(started)
            .unwrap_or_default()
            >= switch_timeout
        {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    Err(SetFailed::NotSwitched {
        setting,
        still: live,
    }
    .into())
}

/// Why `set` failed: the closing `Error:` line
/// ([`crate::l10n::Localized`]).
#[derive(Debug)]
pub(crate) enum SetFailed {
    /// `input` is a fragment of several of the setting's choices, `labels`.
    Ambiguous {
        setting: Setting,
        input: String,
        labels: Vec<String>,
    },
    /// `input` is a fragment of none of them.
    Unknown { setting: Setting, input: String },
    /// The meter would not switch to `choice`, for `reason`.
    Refused {
        setting: Setting,
        choice: String,
        reason: Rejected,
    },
    /// The meter took the command but never showed the choice, and is
    /// `still` on the one it was on, when a reading said.
    NotSwitched {
        setting: Setting,
        still: Option<String>,
    },
}

impl crate::l10n::Localized for SetFailed {
    fn text<'a>(&'a self, l: &'a Localizer) -> std::borrow::Cow<'a, str> {
        match self {
            // What was typed, and the labels it matches as the listing
            // spells them: both are typed back.
            Self::Ambiguous {
                setting,
                input,
                labels,
            } => tr!(
                l,
                "settings-ambiguous",
                setting = setting.name(),
                input = input.as_str(),
                choices = labels.join(", "),
            ),
            Self::Unknown { setting, input } => tr!(
                l,
                "settings-unknown",
                setting = setting.name(),
                input = input.as_str()
            ),
            // Not the dial: the meter is another model, so say which to pick.
            Self::Refused {
                setting,
                choice,
                reason,
            } if let Some(pick) = dmm_shared::help::MeterChoice::for_rejected(reason) => {
                tr!(
                    l,
                    "settings-refused-other-model",
                    choice = choice_text(l, *setting, choice),
                    reason = l.rejected(reason),
                    remedy = l.help().cli_choose_meter(pick, crate::open::picked_row()),
                )
            }
            Self::Refused {
                setting,
                choice,
                reason,
            } => tr!(
                l,
                "settings-refused",
                setting = setting.name(),
                choice = choice_text(l, *setting, choice),
                reason = l.rejected(reason),
            ),
            Self::NotSwitched {
                setting,
                still: Some(live),
            } => tr!(
                l,
                "settings-not-switched-still",
                setting = setting.name(),
                live = choice_text(l, *setting, live),
            ),
            Self::NotSwitched {
                setting,
                still: None,
            } => tr!(l, "settings-not-switched", setting = setting.name()),
        }
    }
}

crate::l10n::localized_error!(SetFailed);

/// What `set` prints once the meter is on `label`, in that setting's own
/// words. `now` picks between the line a switch prints and the one a meter
/// that was already there prints, so the pair cannot drift apart.
fn switch_message(now: bool, setting: Setting, id: u16, label: &str, range_label: &str) -> String {
    let l = crate::l10n::get();
    let case = match setting {
        Setting::Mode => "mode",
        // "Auto" alone leaves the user guessing which rung that is.
        Setting::Range if id == AUTO_RANGE_ID => "range-auto",
        Setting::Range => "range",
        Setting::Hold => "hold",
        Setting::Rel => "rel",
        // Off is not a state the meter is "in", it is one it has left.
        Setting::MinMax if id == 0 => "minmax-off",
        Setting::MinMax => "minmax",
        Setting::Peak if id == 0 => "peak-off",
        Setting::Peak => "peak",
    };
    let choice = choice_text(l, setting, label);
    let range = l.label(TextKind::Range, range_label);
    if now {
        tr!(
            l,
            "settings-switched",
            case = case,
            choice = choice,
            range = range
        )
    } else {
        tr!(
            l,
            "settings-already",
            case = case,
            choice = choice,
            range = range
        )
    }
    .into_owned()
}

/// The listing a single setting gets: a header naming the meter and, for
/// everything but the mode itself, what it is measuring.
fn choices_listing(
    model_name: &'static str,
    setting: Setting,
    reading: &dmm_lib::measurement::Measurement,
    choices: &[Choice],
) -> Vec<String> {
    let l = crate::l10n::get();
    let header = tr!(
        l,
        "settings-heading",
        setting = setting.name(),
        meter = style(prose_name(model_name)).bold().to_string(),
        mode = l.label(TextKind::Mode, &reading.mode),
    )
    .into_owned();
    let rung = auto_range_rung(setting, choices, reading);
    let width = choices
        .iter()
        .map(|c| c.label.chars().count())
        .max()
        .unwrap_or(0);
    let rows = choices.iter().map(|c| {
        // Pad the bare label: styling it first would count escape bytes
        // toward the width and misalign the column.
        let pad = " ".repeat(width - c.label.chars().count());
        let note = match rung {
            Some(rung) if c.id == AUTO_RANGE_ID => {
                let rung = l.label(TextKind::Range, rung);
                format!("  {}", tr!(l, "settings-auto-now", range = rung))
            }
            _ => String::new(),
        };
        format!(
            "{} {}{pad}  {}{}",
            if c.current {
                style("*").green().bold()
            } else {
                style(" ")
            },
            c.label,
            style(quote_for_shell(&shortest_fragment(choices, c))).dim(),
            style(note).dim(),
        )
    });
    std::iter::once(header).chain(rows).collect()
}

/// One line per choice, `*` on the live one, and the least that has to be
/// typed to reach it in a second column — so the fragment form is on screen
/// rather than something to guess at.
fn print_choices(
    model_name: &'static str,
    setting: Setting,
    reading: &dmm_lib::measurement::Measurement,
    choices: &[Choice],
) {
    for line in choices_listing(model_name, setting, reading, choices) {
        println!("{line}");
    }
}

/// The whole-meter listing: one row per setting, its name, the live value
/// behind a `*`, then what else it reaches. The labels themselves stand in
/// for the per-setting listing's fragment column.
fn settings_listing(
    model_name: &'static str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) -> Vec<String> {
    let l = crate::l10n::get();
    let header = tr!(
        l,
        "settings-all-heading",
        meter = style(prose_name(model_name)).bold().to_string(),
        mode = l.label(TextKind::Mode, &reading.mode),
    )
    .into_owned();
    let name_width = offered
        .iter()
        .map(|(s, _)| s.name().chars().count())
        .max()
        .unwrap_or(0);
    let live_width = offered
        .iter()
        .map(|(_, choices)| live_label(choices).chars().count())
        .max()
        .unwrap_or(0);
    let rows = offered.iter().map(|(setting, choices)| {
        let live = live_label(choices);
        let others: Vec<&str> = choices
            .iter()
            .filter(|c| !c.current)
            .map(|c| c.label.as_ref())
            .collect();
        let note = match auto_range_rung(*setting, choices, reading) {
            Some(rung) => {
                let rung = l.label(TextKind::Range, rung);
                format!("  {}", tr!(l, "settings-auto-in", range = rung))
            }
            None => String::new(),
        };
        format!(
            "  {:name_width$}  {} {}{}  {}{}",
            setting.name(),
            if choices.iter().any(|c| c.current) {
                style("*").green().bold()
            } else {
                style(" ")
            },
            live,
            " ".repeat(live_width - live.chars().count()),
            others.join("  "),
            style(note).dim(),
        )
    });
    std::iter::once(header).chain(rows).collect()
}

fn print_settings_table(
    model_name: &'static str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) {
    for line in settings_listing(model_name, reading, offered) {
        println!("{line}");
    }
}

/// Every setting the meter offers a real choice in, in [`Setting::ALL`]
/// order. One that offers nothing — an unimplemented Peak, a dial with a
/// single mode — is simply absent, from both the table and the JSON.
fn offered_settings<T: dmm_lib::transport::Transport>(
    dmm: &dmm_lib::Dmm<T>,
    reading: &dmm_lib::measurement::Measurement,
) -> Vec<(Setting, Vec<Choice>)> {
    Setting::ALL
        .iter()
        .map(|&s| (s, dmm.choices(s, reading)))
        .filter(|(_, choices)| offers_a_switch(choices))
        .collect()
}

/// The three fields every `get --format json` object leads with: which meter
/// answered, and what it was measuring when it did.
fn json_header(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
) -> serde_json::Map<String, serde_json::Value> {
    let mut header = serde_json::Map::new();
    header.insert("device".into(), model_name.into());
    header.insert("mode".into(), reading.mode.as_ref().into());
    header.insert("range".into(), reading.range_label.as_ref().into());
    header
}

/// One setting's block, used flat for `get <SETTING>` and as an element of
/// the `settings` array for `get`.
fn setting_json(setting: Setting, choices: &[Choice]) -> serde_json::Value {
    serde_json::json!({
        "setting": setting.name(),
        "current": choices.iter().find(|c| c.current).map(|c| c.label.as_ref()),
        "choices": choices
            .iter()
            .map(|c| serde_json::json!({"label": c.label, "current": c.current}))
            .collect::<Vec<_>>(),
    })
}

/// What `get <SETTING> --format json` prints. Flat, not nested: the query
/// asked about one setting, so its fields sit beside the header's.
fn one_setting_json(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    setting: Setting,
    choices: &[Choice],
) -> serde_json::Map<String, serde_json::Value> {
    let mut doc = json_header(model_name, reading);
    if let serde_json::Value::Object(fields) = setting_json(setting, choices) {
        doc.extend(fields);
    }
    doc
}

/// What `get --format json` prints: the header, then a block per setting the
/// meter offers a choice in.
fn settings_json(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) -> serde_json::Map<String, serde_json::Value> {
    let mut doc = json_header(model_name, reading);
    doc.insert(
        "settings".into(),
        offered
            .iter()
            .map(|(s, choices)| setting_json(*s, choices))
            .collect(),
    );
    doc
}

/// One object per invocation, and nothing else on stdout — `get --format json`
/// answers a question rather than streaming, unlike `read`.
fn print_json(doc: serde_json::Map<String, serde_json::Value>) -> Result<(), crate::l10n::Failure> {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(doc))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{FakeList, Quirks, fake_meter, fake_meter_with, mode_choice};
    use dmm_lib::error::Rejected;
    use dmm_lib::text::TextKind;

    /// A meter on the V AC dial: two modes, four ranges, HOLD, and no Peak.
    fn a_full_meter() -> [FakeList; 4] {
        [
            (Setting::Mode, &["V AC", "V AC Hz"], 0),
            (Setting::Range, &["Auto", "2.2V", "22V", "220V"], 0),
            (Setting::Hold, &["off", "on"], 0),
            (Setting::MinMax, &["off", "max", "min"], 0),
        ]
    }

    /// The lines a listing is made of, with the styling stripped so the test
    /// reads the same whether or not colour is on.
    fn plain(lines: Vec<String>) -> Vec<String> {
        lines
            .into_iter()
            .map(|l| console::strip_ansi_codes(&l).into_owned())
            .collect()
    }

    /// A single-variant dial (the UT181A Ohm, nS, Cap, Hz, Duty and Pulse
    /// Width positions) reports one choice: the value the meter is already
    /// on. Listing it, and tipping the user to switch to it, is noise — it
    /// counts as nothing to switch, exactly as an empty list does.
    #[test]
    fn only_the_live_value_is_not_a_switch_to_offer() {
        assert!(!offers_a_switch(&[]));
        assert!(!offers_a_switch(&[mode_choice(0x5111, "Resistance", true)]));
        assert!(offers_a_switch(&[
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ]));
    }

    /// And both commands say so and stop: exit 0, meter untouched, whether or
    /// not a choice was asked for.
    #[test]
    fn a_setting_with_only_the_live_value_switches_nothing() {
        let lists: [FakeList; 1] = [(Setting::Mode, &["Resistance"], 0)];
        for arg in [None, Some("Resistance".to_string())] {
            let (mut dmm, selected) = fake_meter(&lists);
            run_set(&mut dmm, Setting::Mode, arg).expect("one choice is not a failure");
            assert!(
                selected.lock().expect("poisoned").is_empty(),
                "the meter was switched"
            );
        }
        let (mut dmm, selected) = fake_meter(&lists);
        run_get(&mut dmm, Some(Setting::Mode), SettingsFormat::Text).expect("nothing to list");
        run_get(&mut dmm, None, SettingsFormat::Text).expect("nothing to list");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// The whole-meter listing: one row per setting that offers a choice, the
    /// live value behind a `*`, and the rung autoranging picked.
    #[test]
    fn the_settings_table_lists_every_offered_setting() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let offered = offered_settings(&dmm, &reading);
        assert_eq!(
            offered.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            [
                Setting::Mode,
                Setting::Range,
                Setting::Hold,
                Setting::MinMax
            ],
            "Peak offers nothing, so it is absent"
        );
        let lines = plain(settings_listing("Fake meter", &reading, &offered));
        assert_eq!(lines[0], "Settings for Fake meter (V AC):");
        assert_eq!(
            &lines[1..],
            [
                "  mode    * V AC  V AC Hz",
                "  range   * Auto  2.2V  22V  220V  (auto-ranging in 22V)",
                "  hold    * off   on",
                "  minmax  * off   max  min",
            ]
        );
    }

    /// The per-setting listing keeps the mode header it always had, names the
    /// live mode for every other setting, and says which rung Auto picked.
    #[test]
    fn a_setting_listing_names_the_meter_and_the_live_mode() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");

        let modes = dmm.choices(Setting::Mode, &reading);
        let lines = plain(choices_listing(
            "Fake meter",
            Setting::Mode,
            &reading,
            &modes,
        ));
        assert_eq!(lines[0], "Modes for Fake meter:");
        assert_eq!(&lines[1..], ["* V AC     \"v ac\"", "  V AC Hz  hz"]);

        let ranges = dmm.choices(Setting::Range, &reading);
        let lines = plain(choices_listing(
            "Fake meter",
            Setting::Range,
            &reading,
            &ranges,
        ));
        assert_eq!(lines[0], "Range for Fake meter in V AC:");
        assert_eq!(lines[1], "* Auto  auto  (now 22V)");
    }

    /// `get <SETTING> --format json` is one flat object: which meter, what it
    /// is measuring, and the setting's own choices by label — the label is
    /// what `set` takes, so the library's ids stay out of the contract.
    #[test]
    fn one_setting_json_carries_the_choices_by_label() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let choices = dmm.choices(Setting::Range, &reading);
        let doc = serde_json::Value::Object(one_setting_json(
            "Fake meter",
            &reading,
            Setting::Range,
            &choices,
        ));
        let parsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(parsed["device"], "Fake meter");
        assert_eq!(parsed["mode"], "V AC");
        assert_eq!(parsed["range"], "22V");
        assert_eq!(parsed["setting"], "range");
        let printed = serde_json::to_string(&doc).unwrap();
        assert!(
            printed.starts_with(r#"{"device":"Fake meter","mode":"V AC","range":"22V","setting""#),
            "the header leads, as the reference shows: {printed}"
        );
        assert_eq!(parsed["current"], "Auto");
        assert!(parsed["choices"][0].get("id").is_none(), "no ids: {parsed}");
        assert_eq!(parsed["choices"][0]["label"], "Auto");
        assert_eq!(parsed["choices"][0]["current"], true);
        assert_eq!(parsed["choices"][2]["label"], "22V");
        assert_eq!(parsed["choices"][2]["current"], false);
        assert_eq!(parsed["choices"].as_array().unwrap().len(), 4);
    }

    /// `get --format json` is one object per invocation, not one per line —
    /// and a setting the meter offers nothing in is absent, not empty.
    #[test]
    fn the_settings_json_omits_a_setting_with_nothing_to_offer() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let offered = offered_settings(&dmm, &reading);
        let printed = serde_json::to_string(&serde_json::Value::Object(settings_json(
            "Fake meter",
            &reading,
            &offered,
        )))
        .unwrap();
        assert_eq!(printed.lines().count(), 1, "one object, not a stream");
        let parsed: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(parsed["device"], "Fake meter");
        let settings = parsed["settings"].as_array().unwrap();
        let names: Vec<&str> = settings
            .iter()
            .map(|s| s["setting"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["mode", "range", "hold", "minmax"]);
        assert!(!printed.contains("peak"), "{printed}");
        assert_eq!(settings[2]["current"], "off");
        assert_eq!(settings[2]["choices"][1]["label"], "on");
    }

    /// `set hold on`: the meter is asked for id 1 and confirms it.
    #[test]
    fn set_switches_a_button_setting_by_label() {
        let (mut dmm, selected) = fake_meter(&a_full_meter());
        run_set(&mut dmm, Setting::Hold, Some("on".to_string())).expect("the meter takes it");
        assert_eq!(*selected.lock().expect("poisoned"), [(Setting::Hold, 1)]);
    }

    /// Asking for what the meter is already on costs no command at all.
    #[test]
    fn set_to_the_live_value_touches_nothing() {
        let (mut dmm, selected) = fake_meter(&a_full_meter());
        run_set(&mut dmm, Setting::Hold, Some("off".to_string())).expect("already there");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// A refusal is the meter answering: the error repeats what it said and
    /// what the user can do about it.
    #[test]
    fn a_refused_switch_repeats_the_meters_reason() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                refusal: Some(Rejected::DidNothing {
                    button: "HOLD",
                    state: "off".into(),
                    kind: TextKind::Toggle,
                }),
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Hold, Some("on".to_string()))
            .expect_err("the meter refused")
            .to_string();
        assert!(err.contains("the meter refused on"), "{err}");
        assert!(err.contains("HOLD did nothing in off"), "{err}");
        assert!(err.contains("check the dial position"), "{err}");
    }

    /// A meter that refuses because it is another model says which to pass,
    /// not to check the dial.
    #[test]
    fn a_refusal_from_another_model_says_what_to_pass() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                refusal: Some(Rejected::OtherModel {
                    code: 61,
                    model: "OWON CMS061",
                    opened: "OWON B35T+",
                    id: "cms061",
                }),
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Hold, Some("on".to_string()))
            .expect_err("the meter refused")
            .to_string();
        assert!(
            err.ends_with("so remote keys are off. Pass --device cms061."),
            "{err}"
        );
        assert!(!err.contains("dial"), "{err}");
    }

    /// A meter that takes the command and never reports the new value fails
    /// after the deadline, naming what it is still on.
    #[test]
    fn a_switch_the_meter_never_confirms_fails() {
        let (mut dmm, selected) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                deaf: true,
                ..Default::default()
            },
        );
        let err = run_set_within(
            &mut dmm,
            Setting::Hold,
            Some("on".to_string()),
            Duration::ZERO,
        )
        .expect_err("never confirmed")
        .to_string();
        assert!(err.contains("Meter did not switch (still off)"), "{err}");
        assert!(err.contains("check the dial position"), "{err}");
        assert_eq!(*selected.lock().expect("poisoned"), [(Setting::Hold, 1)]);
    }

    /// A range is refused for one reason the dial does not cover, so its hint
    /// names that reason too.
    #[test]
    fn a_range_that_will_not_take_says_to_check_the_input() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                refusal: Some(Rejected::AutoDidNothing {
                    range: "22V".into(),
                }),
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Range, Some("22V".to_string()))
            .expect_err("the meter refused")
            .to_string();
        assert!(err.contains("within the range"), "{err}");
    }

    /// `set range auto` from a manual rung: Auto is id 0, and the meter is
    /// asked for exactly that.
    #[test]
    fn set_range_auto_asks_for_the_autorange_id() {
        let lists: [FakeList; 1] = [(Setting::Range, &["Auto", "2.2V", "22V"], 2)];
        let (mut dmm, selected) = fake_meter(&lists);
        run_set(&mut dmm, Setting::Range, Some("auto".to_string())).expect("the meter takes it");
        assert_eq!(
            *selected.lock().expect("poisoned"),
            [(Setting::Range, AUTO_RANGE_ID)]
        );
    }

    /// Each setting says what the meter now is in its own words, and the
    /// "already" line mirrors it word for word.
    #[test]
    fn switch_messages_speak_each_settings_own_language() {
        for (setting, id, label, expected) in [
            (Setting::Mode, 0x1121, "V AC Hz", "in V AC Hz"),
            (Setting::Range, AUTO_RANGE_ID, "Auto", "auto-ranging (22V)"),
            (Setting::Range, 2, "22V", "in 22V (manual range)"),
            (Setting::Hold, 1, "on", "HOLD on"),
            (Setting::Rel, 0, "off", "REL off"),
            (Setting::MinMax, 1, "max", "in max"),
            (Setting::MinMax, 0, "off", "out of MIN/MAX"),
            (Setting::Peak, 2, "P-MIN", "in P-MIN"),
            (Setting::Peak, 0, "off", "out of Peak"),
        ] {
            assert_eq!(
                switch_message(true, setting, id, label, "22V"),
                format!("Meter now {expected}")
            );
            assert_eq!(
                switch_message(false, setting, id, label, "22V"),
                format!("Meter is already {expected}")
            );
        }
    }

    /// A choice that matches several, or none, names the setting it was
    /// asked about — the wording is shared by all six.
    #[test]
    fn an_unresolvable_choice_names_the_setting() {
        let lists: [FakeList; 1] = [(
            Setting::Mode,
            &["Temp °C", "Temp °C T2", "Temp °C T1-T2"],
            0,
        )];
        let (mut dmm, selected) = fake_meter(&lists);
        let err = run_set(&mut dmm, Setting::Mode, Some("temp".to_string()))
            .expect_err("several match")
            .to_string();
        assert!(
            err.starts_with("ambiguous mode: temp matches Temp"),
            "{err}"
        );

        let (mut dmm, _) = fake_meter(&a_full_meter());
        let err = run_set(&mut dmm, Setting::Range, Some("500V".to_string()))
            .expect_err("none match")
            .to_string();
        assert_eq!(err, "unknown range: 500V");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// The vendor app sleeps 100 ms after every SET_MODE, so a meter that
    /// goes quiet across the switch is expected. One timeout must not end the
    /// 2 s wait the switch just started.
    #[test]
    fn a_switch_waits_through_a_quiet_meter() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                post_switch_errors: vec![dmm_lib::error::Error::Timeout],
                ..Default::default()
            },
        );
        run_set(&mut dmm, Setting::Mode, Some("V AC Hz".to_string()))
            .expect("a timeout must not end the wait");
    }

    /// Everything that is not a garbled frame or a quiet meter still ends the
    /// wait: a dead link is not something more polling will fix.
    #[test]
    fn a_switch_gives_up_on_a_lost_link() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                post_switch_errors: vec![dmm_lib::error::Error::NoTransportFound {
                    cables: Vec::new(),
                    bluetooth_searched: false,
                }],
                ..Default::default()
            },
        );
        assert!(run_set(&mut dmm, Setting::Mode, Some("V AC Hz".to_string())).is_err());
    }
}
