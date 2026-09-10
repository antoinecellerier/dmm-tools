//! The capture run's keyboard, and the log of parse rejections a step
//! collects while it is sampling.

use console::Key;
use std::io::Write;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// The one message for a keyboard reader that has gone away, so a dead thread
/// ends the run with a reason instead of hanging on a channel nobody feeds.
fn input_gone() -> Box<dyn std::error::Error> {
    "keyboard input stopped working; finish the capture and rerun".into()
}

/// The capture run's keyboard.
///
/// `Term::read_key` blocks, so it runs on its own thread and the watcher polls
/// the channel between readings. Every prompt goes through here too: a second
/// reader would race this one for the operator's keystrokes.
pub(crate) struct Input {
    /// `None` when stderr is not a terminal (a piped run): `read_key` needs
    /// one, so input falls back to whole lines from stdin.
    keys: Option<Receiver<Key>>,
}

impl Input {
    pub(crate) fn start() -> Self {
        if !console::Term::stderr().is_term() {
            return Input { keys: None };
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            // catch_unwind: a panic here must close the channel rather than
            // leave every later prompt waiting on a thread that is gone.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let term = console::Term::stderr();
                while let Ok(key) = term.read_key() {
                    if tx.send(key).is_err() {
                        break;
                    }
                }
            }));
        });
        Input { keys: Some(rx) }
    }

    /// A keyboard nobody is at, for tests: `start` would read the real
    /// terminal whenever the test binary keeps one (`--nocapture`).
    #[cfg(test)]
    pub(crate) fn piped() -> Self {
        Input { keys: None }
    }

    /// Whether keys can be polled without blocking — false for a piped run,
    /// which has to be asked rather than watched.
    pub(crate) fn is_tty(&self) -> bool {
        self.keys.is_some()
    }

    /// Throw away keys typed before now.
    ///
    /// The reader thread buffers every keystroke, so a second Enter at a
    /// confirmation prompt would still be waiting when the next step starts
    /// watching — and would end that wait at once, filing the state the meter
    /// was in before the operator touched the dial.
    pub(crate) fn drain_keys(&self) {
        let Some(keys) = &self.keys else {
            return;
        };
        while keys.try_recv().is_ok() {}
    }

    /// The key waiting, if any. Never blocks, so the watcher keeps reading.
    pub(crate) fn try_key(&self) -> Result<Option<Key>, Box<dyn std::error::Error>> {
        let Some(keys) = &self.keys else {
            return Ok(None);
        };
        match keys.try_recv() {
            Ok(key) => Ok(Some(key)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(input_gone()),
        }
    }

    /// Ask for one keystroke. Enter reads as `'\n'`, as it did when this was a
    /// direct `read_char`.
    pub(crate) fn key(&self, msg: &str) -> Result<char, Box<dyn std::error::Error>> {
        eprint!("{msg}");
        std::io::stderr().flush()?;
        let Some(keys) = &self.keys else {
            // No terminal: a whole line, of which the first character answers.
            let line = read_stdin_line()?;
            return Ok(line.chars().next().unwrap_or('\n'));
        };
        loop {
            match keys.recv().map_err(|_| input_gone())? {
                Key::Char(c) => {
                    eprintln!();
                    return Ok(c);
                }
                Key::Enter => {
                    eprintln!();
                    return Ok('\n');
                }
                // Arrows and the like: keep waiting, as `read_char` did.
                _ => {}
            }
        }
    }

    /// Ask for a line, echoing it: the reader thread holds the terminal in raw
    /// mode, so nothing else will.
    pub(crate) fn line(&self, msg: &str) -> Result<String, Box<dyn std::error::Error>> {
        eprint!("{msg}");
        std::io::stderr().flush()?;
        let Some(keys) = &self.keys else {
            return read_stdin_line();
        };
        let mut out = String::new();
        loop {
            match keys.recv().map_err(|_| input_gone())? {
                Key::Enter => {
                    eprintln!();
                    return Ok(out.trim().to_string());
                }
                Key::Backspace if out.pop().is_some() => {
                    eprint!("\u{8} \u{8}");
                    std::io::stderr().flush()?;
                }
                Key::Char(c) => {
                    out.push(c);
                    eprint!("{c}");
                    std::io::stderr().flush()?;
                }
                _ => {}
            }
        }
    }
}

fn read_stdin_line() -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Parse rejections seen while a step ran, in first-seen order with a repeat
/// count: a stuck meter otherwise fills the report with the same line.
#[derive(Default)]
pub(crate) struct ErrorLog {
    entries: Vec<(String, usize)>,
}

impl ErrorLog {
    /// Echo the rejection the first time it appears — a step that waits for a
    /// state can see hundreds of them.
    pub(super) fn record(&mut self, e: &dmm_lib::error::Error) {
        let text = e.to_string();
        match self.entries.iter_mut().find(|(t, _)| *t == text) {
            Some((_, count)) => *count += 1,
            None => {
                eprintln!("  error: {text}");
                self.entries.push((text, 1));
            }
        }
    }

    pub(crate) fn into_diagnostics(self) -> Vec<String> {
        self.entries
            .into_iter()
            .map(|(text, count)| {
                if count > 1 {
                    format!("{text} (x{count})")
                } else {
                    text
                }
            })
            .collect()
    }
}
