//! The report file: its YAML, where a run writes it, and how a run
//! resumes, overwrites or starts one.

use super::{CaptureReport, StepStatus};
use crate::capture::input::Input;
use dmm_shared::tr;
use serde::Serialize;

/// Bytes as a report writes them: "02 30 20".
pub(in crate::capture) fn hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// "02 30 20" back to the payload it was written from.
pub(in crate::capture) fn hex_bytes(hex: &str) -> Option<Vec<u8>> {
    hex.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).ok())
        .collect()
}

pub(crate) fn save_report(
    report: &CaptureReport,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = to_yaml(report)?;
    // Atomic write (.tmp + fsync + rename), so a crash mid-write doesn't
    // corrupt the existing report.
    dmm_shared::write_atomic(std::path::Path::new(path), yaml.as_bytes())?;
    Ok(())
}

/// `value` as the YAML a report or plan file holds.
///
/// Every string stays on one line: frame hex runs to thousands of
/// characters and is grepped, so no folding into `>` blocks. Strings a
/// YAML 1.1 reader would type otherwise (`'on'`, a date) are quoted, so
/// older builds and other tools read them back as strings.
pub(crate) fn to_yaml<T: Serialize>(value: &T) -> Result<String, serde_saphyr::SerializeError> {
    serde_saphyr::to_string_with_options(
        value,
        serde_saphyr::ser_options! { prefer_block_scalars: false },
    )
}

/// A report or plan file's YAML as `T`.
///
/// Errors stay one line without the source snippet: they follow a file
/// name, and a quoted report line can carry a meter's Bluetooth address.
///
/// No cap on how much the file holds: a long run's report passes the
/// parser's default node count, and limits that grow with the file's
/// length guard nothing once it is in memory. The depth and alias limits,
/// which bound the work a small hostile file can cause, stay.
pub(crate) fn from_yaml<T: serde::de::DeserializeOwned>(
    yaml: &str,
) -> Result<T, serde_saphyr::Error> {
    serde_saphyr::from_str_with_options(
        yaml,
        serde_saphyr::options! {
            with_snippet: false,
            budget: serde_saphyr::budget! {
                max_events: usize::MAX,
                max_nodes: usize::MAX,
                max_total_scalar_bytes: usize::MAX,
            },
        },
    )
}

/// A name reduced to what a file name can safely carry.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// The plan file's name without its directory or extension.
fn plan_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "plan".to_string())
}

/// Where the run's report goes: named after the registry id the run was
/// started for, as the no-response report already is. The meter's own name
/// would be "unknown" for every family that answers no name query, so a
/// UT804, a UT171 and a UT181A all wrote `capture-unknown.yaml`.
pub(in crate::capture) fn capture_path(
    output_override: Option<String>,
    device_id: &str,
    plan_path: Option<&str>,
) -> String {
    output_override.unwrap_or_else(|| match plan_path {
        // A plan run covers a handful of steps nobody else asked for, so it
        // gets its own file rather than resuming into the full report.
        Some(plan) => format!(
            "capture-{}-{}.yaml",
            slug(device_id),
            slug(&plan_stem(plan))
        ),
        None => format!("capture-{}.yaml", slug(device_id)),
    })
}

/// Determine the output path and load an existing report (with resume/overwrite prompt)
/// or create a fresh one. Returns `None` if the user chose to abort.
pub(in crate::capture) fn load_or_create_report(
    output_override: Option<String>,
    device_id: &str,
    plan_path: Option<&str>,
    input: &Input,
) -> Result<Option<(CaptureReport, String)>, Box<dyn std::error::Error>> {
    let output_path = capture_path(output_override, device_id, plan_path);
    let l = crate::l10n::get();

    let report = match std::fs::read_to_string(&output_path) {
        Ok(contents) => match from_yaml::<CaptureReport>(&contents) {
            Ok(mut r) => {
                for step in &mut r.steps {
                    step.normalize_legacy();
                }
                let captured = r
                    .steps
                    .iter()
                    .filter(|s| s.status == StepStatus::Captured)
                    .count();
                let skipped = r
                    .steps
                    .iter()
                    .filter(|s| s.status == StepStatus::Skipped)
                    .count();
                eprintln!(
                    "{}",
                    tr!(
                        l,
                        "capture-found-existing",
                        path = &output_path,
                        captured = captured,
                        skipped = skipped,
                    )
                );
                let ch = input.key(&format!(
                    "{} ",
                    tr!(
                        l,
                        "capture-resume-prompt",
                        resume = "r",
                        fresh = "n",
                        abort = "q",
                    )
                ))?;
                if ch == 'q' || ch == 'Q' {
                    eprintln!("{}", tr!(l, "capture-aborted"));
                    return Ok(None);
                }
                if ch == 'n' || ch == 'N' {
                    let confirm = input.key(&format!(
                        "{} ",
                        tr!(l, "capture-overwrite-confirm", yes = "y", no = "n")
                    ))?;
                    if confirm != 'y' && confirm != 'Y' {
                        eprintln!("{}", tr!(l, "capture-aborted"));
                        return Ok(None);
                    }
                    CaptureReport::default()
                } else if ch == 'r' || ch == 'R' {
                    eprintln!("{}\n", tr!(l, "capture-resuming"));
                    r
                } else {
                    eprintln!("{}", tr!(l, "capture-aborted"));
                    return Ok(None);
                }
            }
            Err(_) => {
                eprintln!("{}", tr!(l, "capture-unreadable", path = &output_path));
                let ch = input.key(&format!(
                    "{} ",
                    tr!(l, "capture-overwrite-unreadable", yes = "y")
                ))?;
                if ch != 'y' && ch != 'Y' {
                    eprintln!("{}", tr!(l, "capture-aborted"));
                    return Ok(None);
                }
                CaptureReport::default()
            }
        },
        Err(_) => CaptureReport::default(),
    };

    Ok(Some((report, output_path)))
}

/// Where the report of a meter that never answered goes: beside the path the
/// run would have written, so `-o` still picks the directory, under its own
/// name so a report in progress is never replaced by an empty one.
pub(in crate::capture) fn no_response_path(
    output_override: Option<&str>,
    device_id: &str,
) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(
        output_override
            .map(str::to_string)
            .unwrap_or_else(|| format!("capture-{}.yaml", slug(device_id))),
    );
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{stem}-no-response.yaml"))
}

/// The version a report section was written by, with the commit.
pub(in crate::capture) fn tool_version() -> String {
    format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

/// Populate report metadata (date, version, device info).
pub(in crate::capture) fn populate_report_metadata(
    report: &mut CaptureReport,
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    device_name: String,
    supported: bool,
) {
    report.date = chrono::Local::now().to_rfc3339();
    report.tool_version = tool_version();
    report.device_name = device_name;
    report.transport_name = Some(dmm.transport().transport_name().to_string());
    if let Ok(info) = dmm.transport().transport_info() {
        report.transport_info = Some(info);
    }
    report.supported = supported;
}
#[cfg(test)]
mod tests {
    use super::*;

    /// The families that answer no name query — UT803/UT804, UT171, UT181A —
    /// all reported themselves as "unknown", so every one of their runs wrote
    /// `capture-unknown.yaml` and resumed into the last meter's report.
    #[test]
    fn the_report_is_named_after_the_device_the_run_was_started_for() {
        assert_eq!(capture_path(None, "ut804", None), "capture-ut804.yaml");
        assert_eq!(
            capture_path(None, "ut804", Some("plans/hz-walk.yaml")),
            "capture-ut804-hz-walk.yaml"
        );
        assert_eq!(
            capture_path(Some("runs/bench.yaml".to_string()), "ut804", None),
            "runs/bench.yaml"
        );
    }

    #[test]
    fn save_and_load_report_file() {
        let report = CaptureReport {
            date: "2026-01-01".to_string(),
            tool_version: "test".to_string(),
            device_name: "UT61E+".to_string(),
            transport_name: None,
            transport_info: None,
            supported: true,
            steps: vec![],
            ..CaptureReport::default()
        };

        let dir = std::env::temp_dir();
        let path = dir
            .join("dmm-cli-test-capture.yaml")
            .to_string_lossy()
            .to_string();

        save_report(&report, &path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: CaptureReport = from_yaml(&contents).unwrap();
        assert_eq!(parsed.device_name, "UT61E+");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}
