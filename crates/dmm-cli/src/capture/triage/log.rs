//! The logger `triage` runs under: what `dmm_lib` logs while decoding a
//! frame is kept, so it prints beside that frame instead of scrolling past
//! on stderr.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::cell::RefCell;

thread_local! {
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct TriageLogger;

static LOGGER: TriageLogger = TriageLogger;

/// Where unrecognised data is reported. Its lines come back through
/// `capture_reports` already, and its once-per-process warning tells a
/// user to report the meter, which is what triage is answering.
const UNRECOGNISED: &str = "dmm_lib::protocol::unrecognised";

fn from_lib(target: &str) -> bool {
    target == "dmm_lib" || target.starts_with("dmm_lib::")
}

impl Log for TriageLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level()
            <= if from_lib(metadata.target()) {
                Level::Debug
            } else {
                Level::Error
            }
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) || record.target() == UNRECOGNISED {
            return;
        }
        if from_lib(record.target()) {
            let line = format!(
                "{}: {}",
                record.level().as_str().to_lowercase(),
                record.args()
            );
            LINES.with_borrow_mut(|lines| lines.push(line));
        } else {
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Install the triage logger in place of the usual one. Call once, before
/// anything logs; a second call leaves the first logger in place.
pub(crate) fn install() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
}

/// What `dmm_lib` logged on this thread since the last call.
pub(super) fn take() -> Vec<String> {
    LINES.with_borrow_mut(std::mem::take)
}
