//! One-shot work on a thread of its own — a file dialog, a file read or
//! written, the update check — so the window never waits on it.
//!
//! The acquisition thread streams for a whole session and has its own
//! channels (`connection.rs`); this is for work that answers once.

use std::sync::mpsc;

use eframe::egui;

/// Work running on a named thread, its answer picked up by [`Task::poll`].
#[derive(Debug)]
pub(super) struct Task<T> {
    rx: mpsc::Receiver<Result<T, String>>,
}

impl<T: Send + 'static> Task<T> {
    /// Run `f` on a thread named `name`. A panic in it comes back as its
    /// text, and the window repaints when the answer is in: an idle window
    /// draws nothing until asked, so nothing would poll it.
    pub(super) fn spawn(
        name: &str,
        ctx: &egui::Context,
        f: impl FnOnce() -> T + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
                    .map_err(|panic| super::connection::panic_text(&*panic));
                let _ = thread_tx.send(answer);
                ctx.request_repaint();
            });
        if let Err(e) = spawned {
            let _ = tx.send(Err(spawn_refused(&e)));
        }
        Self { rx }
    }
}

/// Why a thread the system refused to start gives no answer.
pub(super) fn spawn_refused(e: &std::io::Error) -> String {
    format!("could not start a thread: {e}")
}

impl<T> Task<T> {
    /// The answer once it is in: what `f` returned, or why there is none.
    /// Comes once; drop the task then.
    pub(super) fn poll(&self) -> Option<Result<T, String>> {
        match self.rx.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("the thread stopped without an answer".to_string()))
            }
        }
    }

    /// A task the test answers by hand, through the sender.
    #[cfg(test)]
    pub(super) fn by_hand() -> (mpsc::Sender<Result<T, String>>, Self) {
        let (tx, rx) = mpsc::channel();
        (tx, Self { rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run frames until `task` answers.
    fn wait<T>(task: &Task<T>) -> Result<T, String> {
        for _ in 0..10_000 {
            if let Some(answer) = task.poll() {
                return answer;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("the task never answered");
    }

    /// The answer comes back on a thread of the given name, and a panic
    /// comes back as its text.
    #[test]
    fn a_task_answers_or_says_why_not() {
        let ctx = crate::app::styled_context();
        let named = Task::spawn("task-test", &ctx, || {
            std::thread::current().name().map(str::to_owned)
        });
        assert_eq!(wait(&named), Ok(Some("task-test".to_string())));
        let panicked = Task::<()>::spawn("task-test", &ctx, || panic!("boom"));
        assert_eq!(wait(&panicked), Err("boom".to_string()));
    }

    /// A task whose sender went away says so rather than waiting forever.
    #[test]
    fn a_task_gone_without_an_answer_says_so() {
        let (tx, task) = Task::<()>::by_hand();
        assert_eq!(task.poll(), None);
        drop(tx);
        assert!(task.poll().is_some_and(|answer| answer.is_err()));
    }
}
