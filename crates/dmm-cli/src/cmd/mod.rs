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

/// Set up a Ctrl+C handler that clears the returned flag when triggered.
fn setup_ctrlc() -> Result<Arc<AtomicBool>, Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    Ok(running)
}
