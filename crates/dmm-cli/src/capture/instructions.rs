//! Step instructions in the user's own translation: `--export-instructions`
//! writes the run's steps to a file ([`file_format`]),
//! and `--instructions` shows the file's text in each step's place. The rest
//! of the run is unchanged, and the report keeps the English beside the text
//! shown.
//!
//! One run reads one file, so its text is held for the process, as the
//! localizer is: the step header and the detection check's read-back, which
//! rebuilds its step from the protocol, both find it there.

mod file_format;

use super::CaptureStep;
use console::style;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_shared::tr;
use file_format::{Exported, Problem};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

/// The step texts of the run's `--instructions` file, by step id.
static SHOWN: OnceLock<HashMap<String, String>> = OnceLock::new();

/// What a step shows as its instruction: the user's text, or the English.
pub(super) fn shown(step: &CaptureStep) -> &str {
    translated(step.id, step.instruction).unwrap_or(step.instruction)
}

/// The user's text for step `id`, when it isn't `english` itself.
pub(super) fn translated(id: &str, english: &str) -> Option<&'static str> {
    SHOWN
        .get()?
        .get(id)
        .map(String::as_str)
        .filter(|text| *text != english)
}

/// The steps a file for this run may name: the plan's, else the device's.
fn run_steps(device: &'static SelectableDevice, plan: Option<&[CaptureStep]>) -> Vec<CaptureStep> {
    match plan {
        Some(steps) => steps.to_vec(),
        None => (device.new_protocol)().capture_steps(),
    }
}

/// The command that runs a capture with the file at `path`, for the meter
/// the user picked by `device_id`: a sold-as name (`et2517a`) keeps the
/// report's name for it.
fn run_command(device_id: &str, plan: Option<&str>, path: &str) -> String {
    let plan = plan
        .map(|p| format!(" --plan {}", word(p)))
        .unwrap_or_default();
    format!(
        "dmm-cli --device {device_id} capture{plan} --instructions {}",
        word(path)
    )
}

/// `path` as one word on a command line: quoted when it holds a space, as
/// sh, PowerShell and cmd all read it.
fn word(path: &str) -> Cow<'_, str> {
    if path.contains(char::is_whitespace) {
        Cow::Owned(format!("\"{path}\""))
    } else {
        Cow::Borrowed(path)
    }
}

/// What the steps quote from the meter itself, which a translation keeps as
/// the meter prints it: each word with two capitals or more (HOLD, AC+DC,
/// MAX-MIN, InEr, E.F), and the key a step says to press or hold, however
/// the meter writes it (Select, Tab⇌, ▼ Erase). The profiles' key names are
/// left out: some are descriptions the apps translate ("Current"), not what
/// the key shows. Listed in the file's header, where a translator, or a
/// machine given nothing but the file, reads it first.
fn markings(steps: &[CaptureStep]) -> Vec<String> {
    /// A word that is a symbol rather than a word: ▼, ▲, <.
    fn symbol(word: &str) -> bool {
        !word.chars().any(char::is_alphanumeric)
    }
    let mut marks = std::collections::BTreeSet::new();
    for step in steps {
        let words: Vec<&str> = step
            .instruction
            .split(|c: char| c.is_whitespace() || ",;:()\"'“”".contains(c))
            .map(|w| w.trim_end_matches(['.', '!', '?']))
            .filter(|w| !w.is_empty())
            .collect();
        for (i, word) in words.iter().enumerate() {
            let capitals = word.chars().filter(char::is_ascii_uppercase).count();
            // "Low-Pass" is two capitalised words, not a marking.
            let title_words = word.split(['-', '/']).all(|part| {
                let rest = || part.chars().skip(1);
                rest().any(char::is_lowercase) && !rest().any(char::is_uppercase)
            });
            if word.starts_with(|c: char| c.is_ascii_uppercase()) && capitals >= 2 && !title_words {
                marks.insert(word.to_string());
            }
            // The verb, lowercase or opening a sentence: "HOLD" is the key.
            let pressed = ["press", "hold", "long-press", "short-press"]
                .iter()
                .any(|p| {
                    *word == *p
                        || word.strip_prefix(p[..1].to_ascii_uppercase().as_str()) == Some(&p[1..])
                });
            let Some(&key) = words.get(i + 1).filter(|_| pressed) else {
                continue;
            };
            if !key.starts_with(|c: char| c.is_uppercase()) && !symbol(key) {
                continue;
            }
            // A symbol beside the key's word is part of its name: "Yes ▲",
            // "▼ Erase".
            let key = match words.get(i + 2) {
                Some(&next)
                    if (symbol(next) && !symbol(key))
                        || (symbol(key) && next.starts_with(|c: char| c.is_uppercase())) =>
                {
                    format!("{key} {next}")
                }
                _ => key.to_string(),
            };
            marks.insert(key);
        }
    }
    marks.into_iter().collect()
}

/// `--export-instructions FILE`: write the run's step instructions for the
/// user to translate, beside an existing file rather than over it.
pub(crate) fn export(
    device: &'static SelectableDevice,
    plan_path: Option<&str>,
    path: &str,
) -> Result<(), crate::l10n::Failure> {
    let plan = plan_path.map(super::plan::load).transpose()?;
    let steps = run_steps(device, plan.as_deref());
    let l = crate::l10n::get();
    if steps.is_empty() {
        let meter = crate::open::meter_name(device);
        eprintln!("{}", tr!(l, "capture-instructions-none", meter = meter));
        return Ok(());
    }
    let picked = crate::open::meter_listing(device).id();
    // The name first: an existing file is kept, and the header must name
    // the file it is in.
    let (written, mut file) = crate::output::create_new(Path::new(path))?;
    let file_name = written
        .file_name()
        .map_or(Cow::Borrowed(path), |n| n.to_string_lossy());
    let mut header = vec![tr!(
        l,
        "capture-instructions-header",
        meter = crate::open::meter_name(device),
    )];
    let marks = markings(&steps);
    if !marks.is_empty() {
        header.push(tr!(
            l,
            "capture-instructions-keep",
            marks = marks.join(", ")
        ));
    }
    // Last, as what to do once the translation is done.
    header.push(tr!(
        l,
        "capture-instructions-run",
        command = run_command(picked, plan_path, &file_name),
    ));
    let header: Vec<String> = header
        .iter()
        .flat_map(|part| part.lines().map(str::to_string))
        .collect();
    let exported: Vec<Exported<'_>> = steps
        .iter()
        .map(|s| Exported {
            id: s.id,
            english: s.instruction,
        })
        .collect();
    let text = file_format::export(&header, device.id, &exported);
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    let written = written.display().to_string();
    eprintln!(
        "{}",
        tr!(
            l,
            "capture-instructions-written",
            count = steps.len(),
            path = style(&written).bold().to_string(),
        )
    );
    eprintln!(
        "{}",
        tr!(
            l,
            "capture-instructions-next",
            command = style(run_command(picked, plan_path, &written))
                .cyan()
                .to_string(),
        )
    );
    Ok(())
}

/// `--instructions FILE`: read the user's file and hold its text for the
/// run, before its first step. A file for another meter, or naming steps
/// neither the plan nor the device has (an id a translator changed), stops
/// the run;
/// steps whose English changed since the export are named, and show the
/// file's text all the same.
pub(super) fn install(
    path: &str,
    device: &'static SelectableDevice,
    plan: Option<&[CaptureStep]>,
) -> Result<(), crate::l10n::Failure> {
    let source =
        std::fs::read_to_string(path).map_err(|e| crate::output::at_path(Path::new(path), e))?;
    let file = file_format::read(&source).map_err(|problems| Unreadable {
        path: path.to_string(),
        problems,
    })?;
    if let Some(named) = file.device.filter(|d| d != device.id) {
        return Err(OtherDevice {
            path: path.to_string(),
            named,
            // Checked by registry entry, named as the user picked it.
            device: crate::open::meter_listing(device).id(),
        }
        .into());
    }
    // A plan's steps first, then the device's: a file translated for the
    // device still serves a plan run, and a plan step wins an id both have.
    let mut steps = plan.map(<[CaptureStep]>::to_vec).unwrap_or_default();
    for step in run_steps(device, None) {
        if !steps.iter().any(|s| s.id == step.id) {
            steps.push(step);
        }
    }
    let english = |id: &str| steps.iter().find(|s| s.id == id).map(|s| s.instruction);
    let unknown: Vec<&str> = file
        .steps
        .iter()
        .map(|s| s.id.as_str())
        .filter(|id| english(id).is_none())
        .collect();
    if !unknown.is_empty() {
        return Err(UnknownIds {
            path: path.to_string(),
            unknown: unknown.join(", "),
            known: steps.iter().map(|s| s.id).collect::<Vec<_>>().join(", "),
        }
        .into());
    }
    let changed: Vec<&str> = file
        .steps
        .iter()
        .filter(|s| {
            s.english
                .as_deref()
                .zip(english(&s.id))
                .is_some_and(|(was, now)| was != file_format::fingerprint(now))
        })
        .map(|s| s.id.as_str())
        .collect();
    if !changed.is_empty() {
        let l = crate::l10n::get();
        eprintln!(
            "{}",
            style(tr!(
                l,
                "capture-instructions-changed",
                path = path,
                steps = changed.join(", "),
            ))
            .yellow()
        );
    }
    let texts = file.steps.into_iter().map(|s| (s.id, s.text)).collect();
    // One file per run: a second call, which no run makes, keeps the first.
    let _ = SHOWN.set(texts);
    Ok(())
}

/// The file isn't one `--export-instructions` wrote, or a translation broke
/// its syntax: the run's closing `Error:` line ([`crate::l10n::Localized`]).
#[derive(Debug)]
pub(crate) struct Unreadable {
    path: String,
    problems: Vec<Problem>,
}

impl crate::l10n::Localized for Unreadable {
    fn text<'a>(&'a self, l: &'a dmm_shared::l10n::Localizer) -> Cow<'a, str> {
        let problems: Vec<String> = self
            .problems
            .iter()
            .map(|p| {
                match p {
                    Problem::Syntax { line } => {
                        tr!(l, "capture-instructions-problem-syntax", line = *line)
                    }
                    Problem::NotText { id } => {
                        tr!(l, "capture-instructions-problem-not-text", id = id.as_str())
                    }
                    Problem::Twice { id } => {
                        tr!(l, "capture-instructions-problem-twice", id = id.as_str())
                    }
                    Problem::Unparsable => tr!(l, "capture-instructions-problem-unparsable"),
                }
                .into_owned()
            })
            .collect();
        tr!(
            l,
            "capture-error-instructions-unreadable",
            path = self.path.as_str(),
            problems = problems.join("\n"),
        )
    }
}

crate::l10n::localized_error!(Unreadable);

/// The file was exported for another meter.
#[derive(Debug)]
pub(crate) struct OtherDevice {
    path: String,
    named: String,
    device: &'static str,
}

impl crate::l10n::Localized for OtherDevice {
    fn text<'a>(&'a self, l: &'a dmm_shared::l10n::Localizer) -> Cow<'a, str> {
        tr!(
            l,
            "capture-error-instructions-device",
            path = self.path.as_str(),
            named = self.named.as_str(),
            device = self.device,
            command = format!(
                "`dmm-cli --device {} capture --export-instructions FILE`",
                self.device
            ),
        )
    }
}

crate::l10n::localized_error!(OtherDevice);

/// The file names steps the run doesn't have: most likely an id a
/// translator translated.
#[derive(Debug)]
pub(crate) struct UnknownIds {
    path: String,
    unknown: String,
    known: String,
}

impl crate::l10n::Localized for UnknownIds {
    fn text<'a>(&'a self, l: &'a dmm_shared::l10n::Localizer) -> Cow<'a, str> {
        tr!(
            l,
            "capture-error-instructions-ids",
            path = self.path.as_str(),
            ids = self.unknown.as_str(),
            known = self.known.as_str(),
        )
    }
}

crate::l10n::localized_error!(UnknownIds);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::Localized;
    use dmm_lib::protocol::registry;

    fn device(id: &str) -> &'static SelectableDevice {
        registry::find_device(id).unwrap()
    }

    fn file_for(device: &'static SelectableDevice) -> String {
        let steps = run_steps(device, None);
        let exported: Vec<Exported<'_>> = steps
            .iter()
            .map(|s| Exported {
                id: s.id,
                english: s.instruction,
            })
            .collect();
        file_format::export(&["Header".to_string()], device.id, &exported)
    }

    /// `text` in a file of this process's own, named after the test.
    fn write(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "dmm-instructions-{}-{name}.ftl",
            std::process::id()
        ));
        std::fs::write(&path, text).unwrap();
        path.display().to_string()
    }

    #[test]
    fn every_meters_steps_export_and_read_back() {
        for device in registry::DEVICES {
            let steps = run_steps(device, None);
            let read = file_format::read(&file_for(device)).unwrap();
            assert_eq!(read.device.as_deref(), Some(device.id));
            let back: Vec<(&str, &str)> = read
                .steps
                .iter()
                .map(|s| (s.id.as_str(), s.text.as_str()))
                .collect();
            let want: Vec<(&str, &str)> = steps.iter().map(|s| (s.id, s.instruction)).collect();
            assert_eq!(back, want, "{}", device.id);
        }
    }

    #[test]
    fn a_file_for_another_meter_is_refused() {
        let path = write("other-meter", &file_for(device("ut61eplus")));
        let e = install(&path, device("bm82x"), None).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        let text = e.to_string();
        assert!(text.contains("ut61eplus, not bm82x"), "{text}");
    }

    #[test]
    fn a_translated_id_is_named_with_the_ids_the_run_has() {
        let file = file_for(device("ut61eplus")).replace("\ndcv = ", "\nvcc = ");
        let path = write("translated-id", &file);
        let e = install(&path, device("ut61eplus"), None).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        let text = e.to_string();
        assert!(text.contains(": vcc."), "{text}");
        assert!(text.contains("dcv, "), "{text}");
    }

    #[test]
    fn a_broken_line_is_named_by_its_number() {
        let e = Unreadable {
            path: "steps.ftl".into(),
            problems: vec![
                Problem::Syntax { line: 7 },
                Problem::Twice { id: "dcv".into() },
            ],
        };
        let text = e.text(crate::l10n::english());
        assert!(text.contains("line 7"), "{text}");
        assert!(text.contains("dcv"), "{text}");
    }

    #[test]
    fn the_meters_own_words_are_listed_to_keep() {
        let d = device("ut61eplus");
        let marks = markings(&run_steps(d, None));
        for kept in ["AC", "AC+DC", "HOLD", "MIN/MAX", "SELECT"] {
            assert!(marks.iter().any(|m| m == kept), "{kept} in {marks:?}");
        }
        // Words, not sentences: nothing lowercase-led, no trailing stop.
        assert!(
            marks
                .iter()
                .all(|m| !m.starts_with(char::is_lowercase) && !m.ends_with('.')),
            "{marks:?}"
        );
    }

    #[test]
    fn the_keys_a_step_names_are_kept_however_the_meter_writes_them() {
        let steps = [
            CaptureStep::basic("a", "Ω mode: press Select for AC, then hold Range."),
            CaptureStep::basic("b", "Press ▼ Erase, then press Yes ▲ to confirm."),
            CaptureStep::basic("c", "Press it again. Hold one probe tip; press < or >."),
            // The key, then a sentence starting with the verb; prose words.
            CaptureStep::basic(
                "d",
                "Press HOLD. Press REL. Set ACV Low-Pass Filter, T1-T2.",
            ),
        ];
        assert_eq!(
            markings(&steps),
            [
                "<",
                "AC",
                "ACV",
                "HOLD",
                "REL",
                "Range",
                "Select",
                "T1-T2",
                "Yes ▲",
                "▼ Erase"
            ]
        );
    }

    #[test]
    fn the_run_command_names_the_plan_when_there_is_one() {
        assert_eq!(
            run_command("et2517a", None, "pt.ftl"),
            "dmm-cli --device et2517a capture --instructions pt.ftl"
        );
        assert_eq!(
            run_command("bm82x", Some("plan.yaml"), "pt.ftl"),
            "dmm-cli --device bm82x capture --plan plan.yaml --instructions pt.ftl"
        );
        // Paths with a space stay one word in any shell.
        assert_eq!(
            run_command("bm82x", Some("my plan.yaml"), r"C:\Users\Ana Silva\pt.ftl"),
            r#"dmm-cli --device bm82x capture --plan "my plan.yaml" --instructions "C:\Users\Ana Silva\pt.ftl""#
        );
    }
}
