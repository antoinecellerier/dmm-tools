//! The one line a capture rewrites while it reads the meter.
//!
//! A slow mode can take many seconds to settle and sample with nothing new on
//! screen. The line ticks per reading rather than on a timer, so it stops if
//! the meter does, and it is cleared afterwards, leaving the scrollback as it
//! would be without it.

use super::SampleData;
use super::input::Input;
use console::{Term, style};
use dmm_lib::measurement::Measurement;
use dmm_shared::tr;

const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub(super) struct Progress {
    /// `None` on a piped run, which keeps its plain output, and in tests,
    /// where `Term` would write past the test harness's capture.
    term: Option<Term>,
    frames: usize,
    drawn: bool,
}

impl Progress {
    pub(super) fn new(input: &Input) -> Self {
        Progress {
            term: (input.is_tty() && !cfg!(test)).then(Term::stderr),
            frames: 0,
            drawn: false,
        }
    }

    /// A reading arrived while the run watches the meter.
    pub(super) fn waiting(&mut self, m: &Measurement) {
        self.tick(m, |reading, frame| {
            tr!(
                crate::l10n::get(),
                "capture-progress-watching",
                reading = reading,
                frame = frame,
            )
            .into_owned()
        });
    }

    /// Reading `done` of the step's `total` arrived.
    pub(super) fn reading(&mut self, done: usize, total: usize, m: &Measurement) {
        self.tick(m, |reading, _| {
            tr!(
                crate::l10n::get(),
                "capture-progress-reading",
                done = done,
                total = total,
                reading = reading,
            )
            .into_owned()
        });
    }

    fn tick(&mut self, m: &Measurement, text: impl FnOnce(String, usize) -> String) {
        let Some(term) = &self.term else { return };
        self.frames += 1;
        let spinner = SPINNER[self.frames % SPINNER.len()];
        let text = text(SampleData::from_measurement(m).summary(), self.frames);
        // One column short of the width: a line that fills the last column
        // wraps on some terminals, and `clear_line` then misses its first row.
        let width = usize::from(term.size().1).saturating_sub(1);
        let line = format!("  {spinner} {text}");
        let line = console::truncate_str(&line, width, "…");
        // The cursor goes back to the line's start, so a frame error or a
        // library warning printed meanwhile writes over the line instead of
        // running on after it. The line is decoration: a failed write must
        // not end the run.
        let _ = term.clear_line();
        let _ = term.write_str(&format!("{}\r", style(line).dim()));
        self.drawn = true;
    }

    /// Take the line off the screen, before printing something that stays.
    pub(super) fn clear(&mut self) {
        if let (Some(term), true) = (&self.term, self.drawn) {
            let _ = term.clear_line();
        }
        self.drawn = false;
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.clear();
    }
}
