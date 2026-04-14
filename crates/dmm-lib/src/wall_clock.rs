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
        Self {
            instant_origin: Instant::now(),
            system_origin: SystemTime::now(),
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
