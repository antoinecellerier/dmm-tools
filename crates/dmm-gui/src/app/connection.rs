use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::Stability;
use dmm_lib::transport::Transport;
use eframe::egui;
use log::{error, info, warn};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Messages from the background thread to the UI.
pub(crate) enum DmmMessage {
    Measurement(Measurement),
    Connected {
        name: String,
        experimental: bool,
        /// URL for reporting feedback on experimental protocols.
        feedback_url: String,
        supported_commands: Vec<String>,
    },
    Disconnected(String),
    /// Reconnect attempt in progress — `attempt` is 1-based.
    /// `last_error` is the most recent reconnect failure, if any.
    Reconnecting {
        attempt: u32,
        last_error: Option<String>,
    },
    Error(String),
    /// USB cable/adapter not detected on the bus.
    DeviceNotFound,
    /// Waiting for meter response (consecutive timeout count).
    WaitingForMeter(u32),
}

/// Extract profile info from a newly opened device, optionally query its name,
/// and send a `Connected` message to the UI.
fn establish_connection<T: Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    query_name: bool,
    msg_tx: &mpsc::Sender<DmmMessage>,
    ctx: &egui::Context,
) {
    let profile = dmm.profile();
    let experimental = profile.stability == Stability::Experimental;
    let feedback_url = profile.feedback_url();
    let cmds: Vec<String> = profile
        .supported_commands
        .iter()
        .map(|s| s.to_string())
        .collect();
    let name = if query_name {
        dmm.get_name().ok().flatten().unwrap_or_default()
    } else {
        String::new()
    };
    let _ = msg_tx.send(DmmMessage::Connected {
        name,
        experimental,
        feedback_url,
        supported_commands: cmds,
    });
    ctx.request_repaint();
}

/// Run the measurement loop on a background thread, generic over transport type.
pub(super) fn run_device_thread<T, F>(
    open_fn: F,
    msg_tx: mpsc::Sender<DmmMessage>,
    stop_rx: mpsc::Receiver<()>,
    cmd_rx: mpsc::Receiver<String>,
    ctx: egui::Context,
    query_name: bool,
    sample_interval_ms: u32,
) where
    T: Transport + Send + 'static,
    F: Fn() -> dmm_lib::error::Result<dmm_lib::Dmm<T>> + Send + 'static,
{
    info!("background thread: connecting to device");
    let mut dmm = match open_fn() {
        Ok(mut d) => {
            establish_connection(&mut d, query_name, &msg_tx, &ctx);
            d
        }
        Err(e) => {
            let msg = if e.is_device_not_found() {
                DmmMessage::DeviceNotFound
            } else {
                DmmMessage::Error(e.to_string())
            };
            let _ = msg_tx.send(msg);
            ctx.request_repaint();
            return;
        }
    };

    let tick = Duration::from_millis(sample_interval_ms as u64);
    let mut next_tick = Instant::now() + tick;
    let mut consecutive_timeouts: u32 = 0;
    loop {
        if stop_rx.try_recv().is_ok() {
            info!("background thread: stop signal received");
            break;
        }

        // Process any pending remote commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Err(e) = dmm.send_command(&cmd) {
                warn!("background thread: command failed: {e}");
            }
        }

        match dmm.request_measurement() {
            Ok(m) => {
                consecutive_timeouts = 0;
                if msg_tx.send(DmmMessage::Measurement(m)).is_err() {
                    break;
                }
            }
            Err(dmm_lib::error::Error::Timeout) => {
                consecutive_timeouts += 1;
                warn!("background thread: measurement timeout ({consecutive_timeouts})");
                let _ = msg_tx.send(DmmMessage::WaitingForMeter(consecutive_timeouts));
                ctx.request_repaint();
                if consecutive_timeouts == 5 {
                    let _ = msg_tx.send(DmmMessage::Error(
                        "No response from meter \u{2014} check device selection and USB mode"
                            .to_string(),
                    ));
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                error!("background thread: device error: {e}");
                let _ = msg_tx.send(DmmMessage::Disconnected(e.to_string()));
                ctx.request_repaint();

                // Reconnection loop. Waits on the stop channel so disconnects
                // propagate within the retry interval instead of up to 2s later,
                // and reports each attempt to the UI so the user sees progress.
                let retry_interval = Duration::from_secs(2);
                let mut attempt: u32 = 0;
                let mut last_error: Option<String> = None;
                loop {
                    attempt += 1;
                    let _ = msg_tx.send(DmmMessage::Reconnecting {
                        attempt,
                        last_error: last_error.clone(),
                    });
                    ctx.request_repaint();

                    // Sleep, but wake early on stop signal.
                    match stop_rx.recv_timeout(retry_interval) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }

                    match open_fn() {
                        Ok(mut d) => {
                            info!("background thread: reconnected on attempt {attempt}");
                            establish_connection(&mut d, query_name, &msg_tx, &ctx);
                            dmm = d;
                            break;
                        }
                        Err(err) => {
                            warn!("background thread: reconnect attempt {attempt} failed: {err}");
                            last_error = Some(err.to_string());
                        }
                    }
                }
            }
        }

        ctx.request_repaint();
        if tick > Duration::ZERO {
            // Absolute-tick pacing: sleep until `next_tick`, then advance. This
            // keeps the sample period equal to `tick` regardless of how long
            // `request_measurement` took, and catches up cleanly after long
            // stalls (reconnect, suspend).
            let now = Instant::now();
            if let Some(wait) = next_tick.checked_duration_since(now) {
                std::thread::sleep(wait);
            }
            next_tick += tick;
            let now = Instant::now();
            if next_tick < now {
                next_tick = now + tick;
            }
        }
    }
}

pub(super) fn handle_thread_panic(
    panic: Box<dyn std::any::Any + Send>,
    tx: &mpsc::Sender<DmmMessage>,
    ctx: &egui::Context,
) {
    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    };
    error!("background thread panicked: {msg}");
    let _ = tx.send(DmmMessage::Error(format!("internal error: {msg}")));
    ctx.request_repaint();
}
