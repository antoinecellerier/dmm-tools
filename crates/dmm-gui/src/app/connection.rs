use dmm_lib::error::ErrorKind;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::Stability;
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transport::Transport;
use eframe::egui;
use log::{error, info, warn};
use std::sync::mpsc;
use std::time::Duration;

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
    Disconnected {
        reason: String,
        kind: ErrorKind,
    },
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
    let mut stream = MeasurementStream::new(&mut dmm, tick);
    loop {
        if stop_rx.try_recv().is_ok() {
            info!("background thread: stop signal received");
            break;
        }

        // Process any pending remote commands. Goes through the stream's
        // `dmm_mut()` so the underlying `Dmm` stays owned by the stream
        // across command sends and doesn't reset its tick schedule.
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Err(e) = stream.dmm_mut().send_command(&cmd) {
                warn!("background thread: command failed: {e}");
            }
        }

        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                if msg_tx.send(DmmMessage::Measurement(m)).is_err() {
                    break;
                }
            }
            Ok(StreamEvent::Timeout { consecutive }) => {
                warn!("background thread: measurement timeout ({consecutive})");
                let _ = msg_tx.send(DmmMessage::WaitingForMeter(consecutive));
                ctx.request_repaint();
                if consecutive == 5 {
                    let _ = msg_tx.send(DmmMessage::Error(
                        "No response from meter \u{2014} check device selection and USB mode"
                            .to_string(),
                    ));
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                error!("background thread: device error: {e}");
                let kind = e.kind();
                let _ = msg_tx.send(DmmMessage::Disconnected {
                    reason: e.to_string(),
                    kind,
                });
                ctx.request_repaint();

                // Reconnection loop. Waits on the stop channel so disconnects
                // propagate within the retry interval instead of up to 2s later,
                // and reports each attempt to the UI so the user sees progress.
                //
                // End the stream's borrow on `dmm` before reassigning; we
                // rebuild the stream after reconnect so tick scheduling
                // restarts fresh from the post-reconnect instant.
                // (`drop()` would be clearer but clippy warns because the
                //  stream itself has no Drop impl — the borrow-release we
                //  actually need is what reassignment accomplishes here.)
                let _ = stream;
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
                stream = MeasurementStream::new(&mut dmm, tick);
            }
        }

        ctx.request_repaint();
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
