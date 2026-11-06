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
fn setup_ctrlc() -> Result<Arc<AtomicBool>, Box<dyn std::error::Error>> {
    let stop = Arc::new(AtomicBool::new(false));
    let raised = stop.clone();
    ctrlc::set_handler(move || {
        raised.store(true, Ordering::SeqCst);
    })?;
    Ok(stop)
}

/// Whether a failed open was Ctrl+C calling it off: a quiet end to the
/// command, with nothing to report.
fn called_off(error: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        error.downcast_ref::<dmm_lib::error::Error>(),
        Some(dmm_lib::error::Error::Cancelled)
    )
}
