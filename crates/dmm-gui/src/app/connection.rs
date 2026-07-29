use dmm_lib::error::ErrorKind;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::Stability;
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transport::Transport;
use eframe::egui;
use log::{error, info, warn};
use std::sync::mpsc;
use std::time::Duration;

/// Control messages from the UI to the background thread.
pub(crate) enum ThreadControl {
    /// Exit the loop and release the device.
    Stop,
    /// Halt (`true`) or resume (`false`) acquisition. Halting stops the meter
    /// being polled at all — it is not a display-side freeze.
    SetPaused(bool),
}

/// How often a paused thread wakes to look for work.
///
/// Nothing is read from the meter while paused, but device commands (HOLD,
/// REL, RANGE, …) are user actions rather than acquisition, so they must still
/// reach the meter promptly instead of queueing until resume.
const PAUSE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Apply pending control messages, blocking while paused.
///
/// Returns `false` when the thread should exit: an explicit `Stop`, or a
/// hung-up channel. The hang-up case matters — the UI dropping its sender
/// without stopping us (a panic on the UI thread, or a `connect()` that
/// replaced the channel) used to be indistinguishable from "no messages", so
/// the thread kept the USB handle open and polled the meter forever.
fn handle_control(ctrl_rx: &mpsc::Receiver<ThreadControl>, paused: &mut bool) -> bool {
    loop {
        let msg = if *paused {
            match ctrl_rx.recv_timeout(PAUSE_POLL_INTERVAL) {
                Ok(m) => m,
                Err(mpsc::RecvTimeoutError::Timeout) => return true,
                Err(mpsc::RecvTimeoutError::Disconnected) => return false,
            }
        } else {
            match ctrl_rx.try_recv() {
                Ok(m) => m,
                Err(mpsc::TryRecvError::Empty) => return true,
                Err(mpsc::TryRecvError::Disconnected) => return false,
            }
        };
        match msg {
            ThreadControl::Stop => return false,
            ThreadControl::SetPaused(p) => *paused = p,
        }
    }
}

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
    ctrl_rx: mpsc::Receiver<ThreadControl>,
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

    // How often to re-report an ongoing protocol error to the UI. The first
    // one is always reported; repeats are throttled so a meter parked in an
    // unparseable state doesn't flood the channel.
    const PROTOCOL_ERROR_REPORT_INTERVAL: u32 = 20;

    let tick = Duration::from_millis(sample_interval_ms as u64);
    let mut stream = MeasurementStream::new(&mut dmm, tick);
    let mut protocol_errors: u32 = 0;
    let mut paused = false;
    loop {
        if !handle_control(&ctrl_rx, &mut paused) {
            info!("background thread: stopping");
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

        // Pause halts acquisition itself, rather than letting the UI discard
        // measurements it asked for: the meter is not polled at all. Without
        // this the meter kept being read while "paused", so unplugging it then
        // dropped the GUI into its reconnect loop.
        if paused {
            continue;
        }

        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                protocol_errors = 0;
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
            Err(e) if e.kind() == ErrorKind::Protocol => {
                // A frame we couldn't parse is not a dead link. Either line
                // noise corrupted a checksum, or the meter is in a dial
                // position this family's tables don't cover — and reconnecting
                // fixes neither. It costs a 2 s stall plus an audible
                // identification beep, and for an unknown mode byte it flaps
                // forever: connect, read, fail, repeat.
                //
                // Report it and keep reading. The next good frame clears the
                // message (`DmmMessage::Measurement` resets `last_error`), so
                // one corrupt frame is a blip rather than a hole in the graph.
                protocol_errors = protocol_errors.saturating_add(1);
                warn!("background thread: protocol error ({protocol_errors}): {e}");
                if protocol_errors == 1
                    || protocol_errors.is_multiple_of(PROTOCOL_ERROR_REPORT_INTERVAL)
                {
                    let _ = msg_tx.send(DmmMessage::Error(e.to_string()));
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

                    // Sleep, but wake early on a control message. A pause that
                    // arrives mid-reconnect is recorded and takes effect once
                    // the link is back: there is nothing to halt until then.
                    match ctrl_rx.recv_timeout(retry_interval) {
                        Ok(ThreadControl::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                        Ok(ThreadControl::SetPaused(p)) => paused = p,
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
                protocol_errors = 0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_messages_keeps_running() {
        let (_tx, rx) = mpsc::channel::<ThreadControl>();
        let mut paused = false;
        assert!(handle_control(&rx, &mut paused));
        assert!(!paused);
    }

    #[test]
    fn stop_ends_the_loop() {
        let (tx, rx) = mpsc::channel();
        tx.send(ThreadControl::Stop).unwrap();
        let mut paused = false;
        assert!(!handle_control(&rx, &mut paused));
    }

    /// The UI dropping its sender without sending Stop (a panic on the UI
    /// thread, or a reconnect that replaced the channel) has to end the
    /// thread too — otherwise it keeps the USB handle and polls forever.
    #[test]
    fn hung_up_channel_ends_the_loop() {
        let (tx, rx) = mpsc::channel::<ThreadControl>();
        drop(tx);
        let mut paused = false;
        assert!(!handle_control(&rx, &mut paused));
    }

    #[test]
    fn pause_is_recorded_and_resume_returns_immediately() {
        let (tx, rx) = mpsc::channel();
        let mut paused = false;

        tx.send(ThreadControl::SetPaused(true)).unwrap();
        // Queue the resume too, so the paused branch has a message waiting
        // and the test doesn't sit through the poll interval.
        tx.send(ThreadControl::SetPaused(false)).unwrap();
        assert!(handle_control(&rx, &mut paused));
        assert!(!paused, "resume must clear the pause");
    }

    /// While paused the thread waits rather than spinning, but it still has
    /// to notice a Stop.
    #[test]
    fn stop_is_honoured_while_paused() {
        let (tx, rx) = mpsc::channel();
        tx.send(ThreadControl::Stop).unwrap();
        let mut paused = true;
        assert!(!handle_control(&rx, &mut paused));
    }

    #[test]
    fn hung_up_channel_ends_the_loop_while_paused() {
        let (tx, rx) = mpsc::channel::<ThreadControl>();
        drop(tx);
        let mut paused = true;
        assert!(!handle_control(&rx, &mut paused));
    }

    /// A paused thread with nothing to do returns to the caller so queued
    /// device commands still get sent, and stays paused.
    #[test]
    fn paused_thread_wakes_periodically_and_stays_paused() {
        let (_tx, rx) = mpsc::channel::<ThreadControl>();
        let mut paused = true;
        let start = std::time::Instant::now();
        assert!(handle_control(&rx, &mut paused));
        assert!(paused);
        assert!(
            start.elapsed() >= PAUSE_POLL_INTERVAL,
            "must wait rather than spin"
        );
    }
}
