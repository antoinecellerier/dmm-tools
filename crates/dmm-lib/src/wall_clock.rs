use crate::clock::Clock;
use std::time::{Instant, SystemTime};

/// Captures an `(Instant, SystemTime)` origin pair so monotonic `Instant`
/// timestamps on `Measurement` can be translated to wall-clock times for
/// display and export without losing the ordering guarantees of `Instant`.
///
/// Both the GUI and CLI construct one `WallClock` per session and then use it
/// to derive stable wall-clock timestamps from `m.timestamp`. This keeps
/// exported CSV/JSON and on-screen timestamps aligned with when the device
/// produced the reading, not when the UI or formatter processed it.
#[derive(Debug, Clone, Copy)]
pub struct WallClock {
    instant_origin: Instant,
    system_origin: SystemTime,
}

impl WallClock {
    /// Capture `Instant::now()` and `SystemTime::now()` in quick succession.
    /// The two calls are separated by a handful of nanoseconds, so the pair
    /// defines a stable correspondence for the rest of the session.
    pub fn new() -> Self {
        Self::from_clock(&Clock::real())
    }

    /// Capture the origin pair against a session [`Clock`].
    ///
    /// The system origin is backdated by the clock's preseed burst: those
    /// first readings carry session time that is already spent when the
    /// process starts, so without the backdate a 90 s burst would export 900
    /// readings all stamped with the launch time. On a real clock the burst is
    /// zero and this is [`WallClock::new`].
    pub fn from_clock(clock: &Clock) -> Self {
        let now = SystemTime::now();
        Self {
            instant_origin: clock.now(),
            system_origin: now.checked_sub(clock.preseed()).unwrap_or(now),
        }
    }

    /// Translate a monotonic `Instant` into the corresponding wall-clock
    /// `SystemTime`, using the elapsed time since the origin.
    ///
    /// An `Instant` earlier than the origin (which should not happen in
    /// practice — `WallClock` is captured before any measurement timestamps)
    /// falls back to the origin to avoid underflow.
    pub fn wall_time_for(&self, instant: Instant) -> SystemTime {
        let delta = instant.saturating_duration_since(self.instant_origin);
        self.system_origin + delta
    }
}

impl Default for WallClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn origin_roundtrips_through_instant() {
        let wc = WallClock::new();
        let at_origin = wc.wall_time_for(wc.instant_origin);
        assert_eq!(at_origin, wc.system_origin);
    }

    #[test]
    fn later_instant_maps_to_later_system_time() {
        let wc = WallClock::new();
        let later = wc.instant_origin + Duration::from_millis(250);
        let mapped = wc.wall_time_for(later);
        let delta = mapped.duration_since(wc.system_origin).unwrap();
        assert_eq!(delta, Duration::from_millis(250));
    }

    /// A preseeded session hands out its first readings instantly. They must
    /// still export the wall time they stand for, a burst before the live
    /// readings that follow.
    #[test]
    fn preseed_backdates_the_system_origin() {
        let clock = Clock::real().with_preseed(90.0);
        let wc = WallClock::from_clock(&clock);
        let first = clock.now();
        // The pacing loop spends the burst; 90 s of session time, no waiting.
        clock.sleep(Duration::from_secs(90));
        let live = clock.now();

        let span = wc
            .wall_time_for(live)
            .duration_since(wc.wall_time_for(first))
            .expect("the live reading is later than the burst");
        assert!(span >= Duration::from_secs(90), "burst spanned {span:?}");

        // And the reading taken once the burst is spent carries true wall time.
        let skew = SystemTime::now()
            .duration_since(wc.wall_time_for(live))
            .expect("the live reading is not in the future");
        assert!(
            skew < Duration::from_secs(1),
            "live reading is {skew:?} old"
        );
    }

    #[test]
    fn instant_before_origin_clamps_to_origin() {
        let wc = WallClock::new();
        // An instant we know is older than origin: the wall_clock's own
        // construction happened after this function started, so a captured
        // `Instant` from "now" (reading `Instant::now()` again) is after
        // origin, not before. To exercise the underflow path we use
        // `saturating_duration_since` via a synthesised older instant.
        let earlier = wc
            .instant_origin
            .checked_sub(Duration::from_secs(1))
            .unwrap_or(wc.instant_origin);
        let mapped = wc.wall_time_for(earlier);
        assert_eq!(mapped, wc.system_origin);
    }
}
