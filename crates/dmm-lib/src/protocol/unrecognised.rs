//! One place for "the meter sent something no parser knows".
//!
//! Every parser that meets data outside its spec — display text, a mode or
//! range code, an undefined bit, a frame type — calls [`report_unknown`]
//! instead of logging on its own. The first call of the process is kept
//! ([`first_unrecognised`]) for each app to ask for a report in its own
//! terms, and logged; every later call logs at DEBUG. One report per process
//! keeps a meter parked on an unknown value, or a GUI that reconnects, from
//! repeating it; the report it brings in carries the trace that shows the
//! rest. The state is process-wide rather than per driver because a
//! reconnect builds a new driver, which would report again; any thread can
//! read it, a replay import's as much as the meter's.

use super::REPO_ISSUES_URL;
use log::{debug, info};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::sync::OnceLock;

/// The process's first report of data a parser doesn't know.
static FIRST: OnceLock<Unrecognised> = OnceLock::new();

/// Data a parser met outside its spec, the first time in this process: for
/// each app to ask for a report, naming the meter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrecognised {
    /// The registry id of the entry being read, one that opens and decodes
    /// the meter (for the report's `--device`).
    pub device: &'static str,
    /// The kind of data, in English: `"display text"`, `"mode byte"`.
    pub what: &'static str,
    /// What the meter sent, as the parser formatted it.
    pub value: String,
}

/// The process's first [`Unrecognised`] report, once there is one.
pub fn first_unrecognised() -> Option<&'static Unrecognised> {
    FIRST.get()
}

thread_local! {
    /// Reports made on this thread while [`capture_reports`] runs.
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    /// Set while [`silently`] runs: reports are counted, not logged.
    static SILENT: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Report data the parser for `device_id` does not recognise.
///
/// `device_id` is the registry id of the entry being read: the apps' report
/// hint passes it to `--device`, so it must be one that opens and decodes
/// their meter, not a family label another model shares.
///
/// `what` names the kind of data (`"display text"`, `"mode byte"`), `value`
/// what the meter sent, formatted by the caller with `format_args!` so
/// nothing is allocated unless a line is written.
pub(crate) fn report_unknown(device_id: &'static str, what: &'static str, value: fmt::Arguments) {
    if let Some(count) = SILENT.get() {
        SILENT.set(Some(count + 1));
        return;
    }
    CAPTURED.with_borrow_mut(|captured| {
        if let Some(reports) = captured {
            reports.push(format!("{device_id}: unrecognised {what}: {value}"));
        }
    });
    // Whichever thread gets there first keeps its report; the rest log it.
    let mut first = false;
    FIRST.get_or_init(|| {
        first = true;
        Unrecognised {
            device: device_id,
            what,
            value: value.to_string(),
        }
    });
    if !first {
        debug!("{device_id}: unrecognised {what}: {value}");
        return;
    }
    // A fact for the log: each app asks for the report in its own terms.
    info!(
        "{device_id}: unrecognised {what}: {value}; further unrecognised data is logged at debug \
         level; reports go to {REPO_ISSUES_URL}"
    );
}

/// Run `f` and return what it reported on this thread, one line per call.
///
/// For tests, where the process-wide report is kept once whichever test
/// gets there first, and for `dmm-cli triage`, which files each report
/// beside the payload that raised it. Captures do not nest.
#[doc(hidden)]
pub fn capture_reports<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    CAPTURED.with_borrow_mut(|captured| *captured = Some(Vec::new()));
    let result = f();
    let reports = CAPTURED.with_borrow_mut(Option::take).unwrap_or_default();
    (result, reports)
}

/// Run `f` with its reports neither logged nor warned about, and say
/// whether it made any.
///
/// For the label lists (`crate::text`), which feed a family's own decoder
/// inputs it may not know, and keep only what it decodes without a report.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn silently<R>(f: impl FnOnce() -> R) -> (R, bool) {
    let outer = SILENT.replace(Some(0));
    let result = f();
    let reported = SILENT.replace(outer).is_some_and(|count| count > 0);
    (result, reported)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first report of the process is kept for the apps, whichever
    /// thread made it.
    #[test]
    fn the_first_report_is_kept() {
        report_unknown("ut61eplus", "mode byte", format_args!("0x7f"));
        let first = first_unrecognised().expect("a report is kept");
        assert!(
            !first.device.is_empty() && !first.what.is_empty(),
            "{first:?}"
        );
    }

    #[test]
    fn captures_every_report_on_this_thread() {
        let (value, reports) = capture_reports(|| {
            report_unknown("test", "display text", format_args!("{:?}", "CUT"));
            report_unknown("test", "display text", format_args!("{:?}", "CUT"));
            7
        });
        assert_eq!(value, 7);
        assert_eq!(
            reports,
            [
                "test: unrecognised display text: \"CUT\"",
                "test: unrecognised display text: \"CUT\"",
            ]
        );
    }

    #[test]
    fn silently_counts_without_capturing() {
        let ((), reports) = capture_reports(|| {
            let ((), reported) = silently(|| {
                report_unknown("test", "mode byte", format_args!("0x7f"));
            });
            assert!(reported);
            let ((), reported) = silently(|| {});
            assert!(!reported);
        });
        assert!(reports.is_empty());
    }

    #[test]
    fn nothing_is_kept_outside_a_capture() {
        report_unknown("test", "mode byte", format_args!("0x7f"));
        let ((), reports) = capture_reports(|| {});
        assert!(reports.is_empty());
    }
}
