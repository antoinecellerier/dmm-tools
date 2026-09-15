//! Session clock: the time base a session's readings are stamped with.
//!
//! Production runs on [`Clock::real`], where session time *is* wall time. Two
//! other variants let session time be produced instead of waited for:
//! [`Clock::scaled`] runs it at a chosen multiple of real time, and
//! [`Clock::manual`] only moves when a test [`advance`](Clock::advance)s it.
//! A scaled clock can also carry a burst — [`with_preseed`](Clock::with_preseed)
//! — that hands out its first seconds instantly, so a session can start with
//! minutes of history already behind it.
//!
//! A `Clock` is cheap to clone and clones share one state, so the [`Dmm`](crate::Dmm),
//! the mock's waveform and the pacing loop all read the same session time.
//!
//! Hardware-facing timing stays on [`Instant`]: transport bring-up sleeps,
//! frame read deadlines and settle delays pace physical USB, and no flag can
//! make a meter answer sooner.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime};

/// Longest burst [`Clock::from_flags`] accepts, in seconds.
///
/// A day of session time is more than a screenshot or a perf run asks for,
/// and a bound is what keeps the burst spendable: a budget too large to
/// exhaust leaves a paced acquisition loop running unpaced for the rest of
/// the process, and one past what a [`Duration`] can hold falls back to no
/// burst at all, so the run silently paces at real time instead.
const MAX_PRESEED_SECS: f64 = 86_400.0;

/// The time base of one session. Clone it to share it.
#[derive(Clone, Debug)]
pub struct Clock {
    inner: Inner,
    /// What wall time this session's zero stands for, once something has
    /// pinned it (see [`Clock::with_wall_origin`]). Outside the shared
    /// `Inner` on purpose: it is decided once, before the clones are handed
    /// out, and unlike the burst there is nothing to spend.
    wall_origin: Option<(Instant, SystemTime)>,
}

#[derive(Clone, Debug)]
enum Inner {
    /// Wall time, straight from [`Instant::now`].
    Real,
    /// Session time running at `factor` times real time, plus whatever the
    /// burst has already handed out. Shared so clones spend one budget.
    Scaled(Arc<Scaled>),
    /// Session time that stands still until [`Clock::advance`] moves it.
    Manual(Arc<Mutex<Instant>>),
}

#[derive(Debug)]
struct Scaled {
    /// Real instant the clock was built; also the origin of its session time.
    origin: Instant,
    /// Session seconds per real second. Finite and greater than zero.
    factor: f64,
    /// The burst as configured, for [`Clock::preseed`]. `state.remaining` is
    /// what is left of it.
    preseed: Duration,
    state: Mutex<ScaledState>,
}

#[derive(Debug)]
struct ScaledState {
    /// Session time the burst has granted so far, added to every `now()`.
    offset: Duration,
    /// Burst not yet spent.
    remaining: Duration,
}

impl Clock {
    /// The wall clock: what every session runs on unless a flag says otherwise.
    pub fn real() -> Self {
        Self {
            inner: Inner::Real,
            wall_origin: None,
        }
    }

    /// Session time running `factor` times faster than real time.
    ///
    /// `factor` must be finite and greater than zero; [`Clock::from_flags`]
    /// checks that for values coming from a command line. A value that is not
    /// falls back to 1.0 rather than poisoning every later duration.
    pub fn scaled(factor: f64) -> Self {
        debug_assert!(
            factor.is_finite() && factor > 0.0,
            "clock scale must be finite and positive, got {factor}"
        );
        Self::scaled_with_burst(factor, Duration::ZERO)
    }

    /// Session time that only moves when [`Clock::advance`] says so. For tests.
    pub fn manual() -> Self {
        Self {
            inner: Inner::Manual(Arc::new(Mutex::new(Instant::now()))),
            wall_origin: None,
        }
    }

    /// Start the session `secs` of session time in, handed out instantly.
    ///
    /// The burst is spent by [`sleep`](Clock::sleep): each call takes what it
    /// asked for out of the budget without waiting, so a paced acquisition
    /// loop produces its first `secs` of readings as fast as the device
    /// answers and then drops into scaled time with no seam. Only the scaled
    /// variant carries a burst — on a real clock this builds `scaled(1.0)`,
    /// which is the same time base plus the budget.
    ///
    /// A negative or non-finite `secs` means no burst.
    pub fn with_preseed(self, secs: f64) -> Self {
        debug_assert!(
            !matches!(self.inner, Inner::Manual(_)),
            "a manual clock has no burst to hand out"
        );
        let burst = Duration::try_from_secs_f64(secs).unwrap_or(Duration::ZERO);
        let preseeded = match &self.inner {
            Inner::Real => Self::scaled_with_burst(1.0, burst),
            Inner::Scaled(s) => Self::scaled_with_burst(s.factor, burst),
            Inner::Manual(_) => return self,
        };
        // A pinned origin says what session zero means, not how fast time
        // runs, so it survives being given a burst.
        Self {
            wall_origin: self.wall_origin,
            ..preseeded
        }
    }

    /// Pin this session's *current* time to the wall time `at`.
    ///
    /// A replay's session zero is the moment its recording was made, so a
    /// [`WallClock`](crate::WallClock) built from this clock maps readings to
    /// the times the meter produced them rather than to the run playing them
    /// back. Nothing else pins an origin: a live or mock session's zero is
    /// simply when it started.
    pub fn with_wall_origin(mut self, at: SystemTime) -> Self {
        self.wall_origin = Some((self.now(), at));
        self
    }

    /// The pinned origin: the session instant and the wall time it stands
    /// for. `None` unless [`Clock::with_wall_origin`] set one.
    pub fn wall_origin(&self) -> Option<(Instant, SystemTime)> {
        self.wall_origin
    }

    /// Build the session clock from the `--mock-clock-*` flag values.
    ///
    /// `None` for both is a real clock, so a binary can call this
    /// unconditionally. The messages are what the caller prints verbatim, so
    /// they name the offending value rather than the flag: each binary spells
    /// its own flag name in the error it wraps this in.
    pub fn from_flags(scale: Option<f64>, preseed: Option<f64>) -> Result<Self, String> {
        if scale.is_none() && preseed.is_none() {
            return Ok(Self::real());
        }
        let clock = match scale {
            Some(f) if f.is_finite() && f > 0.0 => Self::scaled(f),
            Some(f) => return Err(format!("clock scale must be a positive number, got '{f}'")),
            // A burst on its own: start further in, then run at real speed.
            None => Self::scaled(1.0),
        };
        match preseed {
            Some(s) if s.is_finite() && (0.0..=MAX_PRESEED_SECS).contains(&s) => {
                Ok(clock.with_preseed(s))
            }
            Some(s) => Err(format!(
                "clock preseed must be between 0 and {MAX_PRESEED_SECS} seconds, got '{s}'"
            )),
            None => Ok(clock),
        }
    }

    /// Whether this is the wall clock. Binaries refuse the clock flags on a
    /// hardware device, where virtual time would stamp readings the meter
    /// never produced at those instants.
    pub fn is_real(&self) -> bool {
        matches!(self.inner, Inner::Real)
    }

    /// The burst this clock was configured with; zero unless preseeded.
    ///
    /// [`WallClock`](crate::WallClock) backdates its system origin by this, so
    /// readings from the burst export the wall time they stand for.
    pub fn preseed(&self) -> Duration {
        match &self.inner {
            Inner::Scaled(s) => s.preseed,
            Inner::Real | Inner::Manual(_) => Duration::ZERO,
        }
    }

    /// The current session time.
    pub fn now(&self) -> Instant {
        match &self.inner {
            Inner::Real => Instant::now(),
            Inner::Scaled(s) => {
                let offset = lock(&s.state).offset;
                let real_elapsed = Instant::now()
                    .checked_duration_since(s.origin)
                    .unwrap_or(Duration::ZERO);
                // Saturate rather than panic: a session long enough to
                // overflow an `Instant` is a bug elsewhere, not a reason to
                // take the acquisition thread down.
                let base = s.origin.checked_add(offset).unwrap_or(s.origin);
                base.checked_add(scale_duration(real_elapsed, s.factor))
                    .unwrap_or(base)
            }
            Inner::Manual(now) => *lock(now),
        }
    }

    /// Let `d` of session time pass.
    ///
    /// Real time waits it out; a scaled clock spends the burst first and then
    /// waits `d / factor`; a manual clock just moves on.
    pub fn sleep(&self, d: Duration) {
        match &self.inner {
            Inner::Real => std::thread::sleep(d),
            Inner::Scaled(s) => {
                let waited = {
                    let mut state = lock(&s.state);
                    let from_burst = d.min(state.remaining);
                    state.remaining -= from_burst;
                    state.offset = state.offset.checked_add(from_burst).unwrap_or(state.offset);
                    d - from_burst
                };
                if !waited.is_zero() {
                    std::thread::sleep(scale_duration(waited, 1.0 / s.factor));
                }
            }
            Inner::Manual(_) => self.advance(d),
        }
    }

    /// Move a manual clock forward by `d`.
    ///
    /// Real and scaled clocks move on their own, so asking them to advance is
    /// a programming error: it trips a debug assertion and does nothing.
    pub fn advance(&self, d: Duration) {
        debug_assert!(
            matches!(self.inner, Inner::Manual(_)),
            "only a manual clock can be advanced"
        );
        if let Inner::Manual(now) = &self.inner {
            let mut now = lock(now);
            *now = now.checked_add(d).unwrap_or(*now);
        }
    }

    fn scaled_with_burst(factor: f64, burst: Duration) -> Self {
        let factor = if factor.is_finite() && factor > 0.0 {
            factor
        } else {
            1.0
        };
        Self {
            wall_origin: None,
            inner: Inner::Scaled(Arc::new(Scaled {
                origin: Instant::now(),
                factor,
                preseed: burst,
                state: Mutex::new(ScaledState {
                    offset: Duration::ZERO,
                    remaining: burst,
                }),
            })),
        }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::real()
    }
}

/// Take a clock lock, ignoring poisoning.
///
/// A panic elsewhere must not stop the session from telling the time: the
/// state behind these locks is a pair of durations that is never left
/// half-updated.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `d × factor`, saturating at [`Duration::MAX`] instead of panicking.
///
/// `factor` is finite and positive by construction, so the product can only
/// leave the representable range by overflowing.
fn scale_duration(d: Duration, factor: f64) -> Duration {
    Duration::try_from_secs_f64(d.as_secs_f64() * factor).unwrap_or(Duration::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The acquisition thread holds a clone while the UI thread holds the
    /// original, so a `Clock` that stopped being shareable would break the GUI
    /// rather than this crate.
    #[test]
    fn clock_is_shareable_across_threads() {
        fn assert_shareable<T: Clone + Send + Sync + std::fmt::Debug>() {}
        assert_shareable::<Clock>();
    }

    /// Loose bound on purpose: the assertion is that scaled time runs much
    /// faster than the sleep, not that the scheduler is precise.
    #[test]
    fn scaled_time_runs_faster_than_real_time() {
        let clock = Clock::scaled(10.0);
        let start = clock.now();
        std::thread::sleep(Duration::from_millis(20));
        let elapsed = clock.now().saturating_duration_since(start);
        assert!(
            elapsed >= Duration::from_millis(150),
            "20 ms at 10x should be well past 150 ms of session time, got {elapsed:?}"
        );
    }

    #[test]
    fn manual_sleep_advances_exactly_and_costs_no_wall_time() {
        let clock = Clock::manual();
        let start = clock.now();
        let real_start = Instant::now();
        clock.sleep(Duration::from_secs(30));
        assert_eq!(
            clock.now().saturating_duration_since(start),
            Duration::from_secs(30)
        );
        assert!(real_start.elapsed() < Duration::from_secs(1));
    }

    /// The acquisition thread and the test that drives it hold two clones.
    #[test]
    fn manual_advance_is_visible_through_a_clone() {
        let clock = Clock::manual();
        let thread_side = clock.clone();
        let start = thread_side.now();
        clock.advance(Duration::from_millis(250));
        assert_eq!(
            thread_side.now().saturating_duration_since(start),
            Duration::from_millis(250)
        );
    }

    /// The burst is what fills a session's history instantly: 90 s of pacing
    /// sleeps at 100 ms are 900 ticks that cost no wall time, and the tick
    /// after that is paced for real.
    #[test]
    fn a_burst_pays_for_whole_ticks_then_stops() {
        let clock = Clock::scaled(10.0).with_preseed(90.0);
        let start = clock.now();
        let tick = Duration::from_millis(100);

        let real_start = Instant::now();
        for _ in 0..900 {
            clock.sleep(tick);
        }
        let burst_time = real_start.elapsed();
        assert!(
            burst_time < Duration::from_millis(500),
            "the burst must not wait, took {burst_time:?}"
        );
        assert!(
            clock.now().saturating_duration_since(start) >= Duration::from_secs(90),
            "900 ticks of 100 ms are 90 s of session time"
        );

        // Budget spent: the 901st tick is a real 10 ms sleep at 10x.
        let real_start = Instant::now();
        clock.sleep(tick);
        assert!(
            real_start.elapsed() >= Duration::from_millis(5),
            "past the burst the clock must pace for real"
        );
    }

    #[test]
    fn a_sleep_longer_than_the_burst_waits_out_the_remainder() {
        let clock = Clock::real().with_preseed(0.02);
        let start = clock.now();
        let real_start = Instant::now();
        clock.sleep(Duration::from_millis(50));
        let real_elapsed = real_start.elapsed();

        assert!(
            real_elapsed >= Duration::from_millis(25),
            "only the 20 ms burst is free, got {real_elapsed:?}"
        );
        assert!(
            clock.now().saturating_duration_since(start) >= Duration::from_millis(50),
            "the whole sleep is session time, burst or not"
        );
    }

    #[test]
    fn preseed_reports_the_configured_burst() {
        assert_eq!(Clock::real().preseed(), Duration::ZERO);
        assert_eq!(Clock::manual().preseed(), Duration::ZERO);
        assert_eq!(Clock::scaled(2.0).preseed(), Duration::ZERO);
        assert_eq!(
            Clock::scaled(2.0).with_preseed(90.0).preseed(),
            Duration::from_secs(90)
        );
        // A burst without a scale still runs at real speed once spent.
        assert_eq!(
            Clock::real().with_preseed(1.5).preseed(),
            Duration::from_millis(1500)
        );
    }

    /// Both binaries print these messages verbatim, and both build their clock
    /// from the same two `Option`s.
    #[test]
    fn from_flags_validates_both_values() {
        assert!(Clock::from_flags(None, None).expect("no flags").is_real());
        assert!(
            !Clock::from_flags(Some(20.0), None)
                .expect("scale")
                .is_real()
        );
        assert_eq!(
            Clock::from_flags(None, Some(90.0))
                .expect("preseed")
                .preseed(),
            Duration::from_secs(90)
        );
        assert_eq!(
            Clock::from_flags(Some(0.0), None).unwrap_err(),
            "clock scale must be a positive number, got '0'"
        );
        assert_eq!(
            Clock::from_flags(Some(f64::INFINITY), None).unwrap_err(),
            "clock scale must be a positive number, got 'inf'"
        );
        assert_eq!(
            Clock::from_flags(None, Some(-1.0)).unwrap_err(),
            "clock preseed must be between 0 and 86400 seconds, got '-1'"
        );
    }

    /// A burst has to be both representable and spendable: `1e30` overflows
    /// `Duration` and used to fall back to no burst at all, and `1e18` is a
    /// budget an uncounted `read` never exhausts, so it spins out readings
    /// unpaced for as long as the process lives.
    #[test]
    fn from_flags_bounds_the_preseed() {
        for secs in [1e30, 1e18, MAX_PRESEED_SECS + 1.0, f64::NAN] {
            assert!(
                Clock::from_flags(None, Some(secs)).is_err(),
                "preseed {secs} should be refused"
            );
        }
        assert_eq!(
            Clock::from_flags(None, Some(MAX_PRESEED_SECS))
                .expect("the cap itself is allowed")
                .preseed(),
            Duration::from_secs(86_400)
        );
    }
}
