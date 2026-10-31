//! `debug`: raw frames beside what they decoded to, for protocol work.
//!
//! The frame lines on stdout are the dump itself and stay as they are; the
//! labels and errors on stderr are in the run's language (`cli/debug.ftl`).

use super::setup_ctrlc;
use crate::open::open_with_help;
use console::style;
use dmm_lib::protocol::registry::Selection;
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_shared::tr;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub(crate) fn cmd_debug(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    count: usize,
    interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;
    let l = crate::l10n::get();

    let (mut dmm, _device) = open_with_help(selection, opts)?;

    // Show transport info before entering measurement loop
    eprintln!(
        "{} {}",
        style(tr!(l, "debug-transport-label")).dim(),
        dmm.transport().transport_name()
    );
    if let Ok(info) = dmm.transport().transport_info() {
        eprintln!("{} {info}", style(tr!(l, "debug-bridge-label")).dim());
    }
    if let Ok(status) = dmm.transport().transport_status() {
        eprintln!("{} {status}", style(tr!(l, "debug-status-label")).dim());
    }

    let tick = Duration::from_millis(interval_ms);
    let mut i = 0;
    let cancel = running.clone();
    let mut stream =
        MeasurementStream::new(&mut dmm, tick).with_cancel(move || !cancel.load(Ordering::SeqCst));

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                // A frame without a main reading has its digits on the
                // sub-value it carries instead.
                let absent = !m.has_main_reading();
                let display = m
                    .display_raw
                    .as_deref()
                    .or_else(|| {
                        absent
                            .then(|| m.aux_values.iter().find_map(|a| a.display_raw.as_deref()))
                            .flatten()
                    })
                    .unwrap_or("(none)");
                println!(
                    "{} mode_raw={:04X} display={:?} progress={:?} flags={} raw={:02X?} \u{2192} {}",
                    style(format!("[{i}]")).dim(),
                    m.mode_raw,
                    display,
                    m.progress,
                    m.flags,
                    m.raw_payload,
                    style(format!("{m}")).green(),
                );
                // The secondary displays a UT181A or UT171 sends alongside
                // the reading; nothing else in the debug line shows them. A
                // frame without a main reading printed them after the arrow.
                if !m.aux_values.is_empty() && !absent {
                    println!("    {} {}", style("sub-values:").dim(), m.aux_summary());
                }
            }
            // Only a replay ends, and `debug` reads a meter.
            Ok(StreamEvent::Ended) => break,
            Ok(StreamEvent::Timeout { .. }) => {
                eprintln!(
                    "{} {}",
                    style(format!("[{i}]")).dim(),
                    style(tr!(l, "debug-timeout")).red()
                );
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    style(format!("[{i}]")).dim(),
                    style(tr!(l, "debug-frame-error", error = l.error(&e))).red()
                );
            }
        }
        i += 1;
    }

    Ok(())
}
