//! One place for "the meter sent something no parser knows".
//!
//! Every parser that meets data outside its spec — display text, a mode or
//! range code, an undefined bit, a frame type — calls [`report_unknown`]
//! instead of logging on its own. The first call of the process warns and
//! says how to report it; every later call logs at DEBUG. One warning per
//! process keeps a meter parked on an unknown value, or a GUI that
//! reconnects, from repeating it; the report it brings in carries the trace
//! that shows the rest.

use super::REPO_ISSUES_URL;
use log::{debug, warn};
use std::cell::RefCell;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

static WARNED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Reports made on this thread while [`capture_reports`] runs.
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Report data the parser for `family` does not recognise.
///
/// `what` names the kind of data (`"display text"`, `"mode byte"`), `value`
/// what the meter sent, formatted by the caller with `format_args!` so
/// nothing is allocated unless a line is written.
pub(crate) fn report_unknown(family: &'static str, what: &'static str, value: fmt::Arguments) {
    CAPTURED.with_borrow_mut(|captured| {
        if let Some(reports) = captured {
            reports.push(format!("{family}: unrecognised {what}: {value}"));
        }
    });
    if !first(&WARNED) {
        debug!("{family}: unrecognised {what}: {value}");
        return;
    }
    warn!(
        "{family}: unrecognised {what}: {value} \u{2014} further unrecognised data is logged at debug level"
    );
    // Named `--device`, not left to auto-detection: the reporter is running
    // a meter we do not fully parse, which is where detection is least sure.
    warn!(
        "report it at {REPO_ISSUES_URL}, with the meter in this state, attaching the output of: \
         RUST_LOG=dmm_lib=trace dmm-cli --device {family} debug --count 20"
    );
    warn!(
        "RUST_LOG=dmm_lib=debug logs every occurrence; RUST_LOG=dmm_lib=error hides this warning"
    );
}

/// Whether this is the first call to flip `flag`.
fn first(flag: &AtomicBool) -> bool {
    !flag.swap(true, Ordering::Relaxed)
}

/// Run `f` and return what it reported on this thread, one line per call.
///
/// For tests: the process-wide warning fires once whichever test gets there
/// first, so tests check this list instead. Captures do not nest.
#[doc(hidden)]
pub fn capture_reports<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    CAPTURED.with_borrow_mut(|captured| *captured = Some(Vec::new()));
    let result = f();
    let reports = CAPTURED.with_borrow_mut(Option::take).unwrap_or_default();
    (result, reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_call_warns() {
        let flag = AtomicBool::new(false);
        assert!(first(&flag));
        assert!(!first(&flag));
        assert!(!first(&flag));
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
    fn nothing_is_kept_outside_a_capture() {
        report_unknown("test", "mode byte", format_args!("0x7f"));
        let ((), reports) = capture_reports(|| {});
        assert!(reports.is_empty());
    }
}
