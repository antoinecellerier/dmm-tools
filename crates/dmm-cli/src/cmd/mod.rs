//! The subcommands, one module each; `completions` needs no module and is
//! answered in `main`.

pub(crate) mod command;
pub(crate) mod debug;
pub(crate) mod info;
pub(crate) mod list;
pub(crate) mod read;
pub(crate) mod settings;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set up a Ctrl+C handler that raises the returned flag: what
/// [`OpenOptions::stop`](dmm_lib::OpenOptions::stop) takes, and what a
/// command's loop ends on. Once only per run, and before a command's open, so
/// a Bluetooth search can be called off and takes its half-up peer down.
fn setup_ctrlc() -> Result<Arc<AtomicBool>, crate::l10n::Failure> {
    let stop = Arc::new(AtomicBool::new(false));
    let raised = stop.clone();
    ctrlc::set_handler(move || {
        raised.store(true, Ordering::SeqCst);
    })?;
    Ok(stop)
}

/// A link detail `info` and `debug` print after its label: the transport's
/// answer, nothing when it has none to give, or why asking for it failed.
fn link_detail(
    l: &dmm_shared::l10n::Localizer,
    query: dmm_lib::error::Result<Option<String>>,
) -> Option<String> {
    query.unwrap_or_else(|e| Some(unavailable(l, &e)))
}

/// A link detail the transport failed to give, and why.
fn unavailable(l: &dmm_shared::l10n::Localizer, e: &dmm_lib::error::Error) -> String {
    dmm_shared::tr!(l, "common-unavailable", error = l.error(e)).into_owned()
}

/// Whether a failed open was Ctrl+C calling it off: a quiet end to the
/// command, with nothing to report.
fn called_off(error: &crate::l10n::Failure) -> bool {
    matches!(error.lib(), Some(dmm_lib::error::Error::Cancelled))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A detail the link gave prints as it is, one it has none of prints
    /// nothing, and one it failed to give says why.
    #[test]
    fn a_link_detail_says_why_it_is_missing() {
        let l = crate::l10n::english();
        assert_eq!(
            link_detail(l, Ok(Some("MTU: 244 bytes".to_string()))).as_deref(),
            Some("MTU: 244 bytes")
        );
        assert_eq!(link_detail(l, Ok(None)), None);
        let e = dmm_lib::error::Error::Timeout;
        assert_eq!(
            link_detail(l, Err(dmm_lib::error::Error::Timeout)),
            Some(format!("unavailable ({})", l.error(&e)))
        );
    }
}
