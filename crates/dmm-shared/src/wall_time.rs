//! Wall times, the `SystemTime` a reading was taken at, as both binaries
//! show, write and read them: in the system's time zone, which `TZ`
//! overrides, and in RFC 3339 in files.

use jiff::fmt::temporal::DateTimePrinter;
use jiff::tz::{Offset, TimeZone};
use jiff::{Timestamp, ZonedDateTime};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

/// `t` in the system's time zone, to format with [`ZonedDateTime::strftime`].
pub fn local(t: SystemTime) -> ZonedDateTime {
    timestamp(t).to_zoned_datetime(zone())
}

/// `t` in RFC 3339 at the local offset, its fraction of a second in as many
/// groups of three digits as it needs: `2026-09-15T14:30:05.250+02:00`,
/// `2026-09-15T14:30:05+02:00`. What an export's timestamp holds.
pub fn rfc3339(t: SystemTime) -> String {
    let t = timestamp(t);
    at_offset(t, zone().to_offset(t), digits_needed(t))
}

/// `t` in RFC 3339 at the local offset, to the millisecond, the fraction
/// always written: `2026-09-15T14:30:05.000+02:00`.
pub fn rfc3339_millis(t: SystemTime) -> String {
    let t = timestamp(t);
    at_offset(t, zone().to_offset(t), 3)
}

/// An RFC 3339 time, as the writers above put it or any offset, `Z`
/// included, gives it. One this platform's `SystemTime` cannot hold (before
/// 1601 on Windows) is refused.
pub fn parse(text: &str) -> Result<SystemTime, jiff::Error> {
    text.parse::<Timestamp>().and_then(SystemTime::try_from)
}

/// The system's time zone. One that cannot be found, from a `TZ` jiff
/// cannot read or a system that names none, is UTC, warned about once and
/// not looked for again: jiff caches only a zone it found, and would look
/// again on every reading an export writes.
fn zone() -> TimeZone {
    static MISSING: AtomicBool = AtomicBool::new(false);
    if MISSING.load(Ordering::Relaxed) {
        return TimeZone::UTC;
    }
    TimeZone::try_system().unwrap_or_else(|e| {
        if !MISSING.swap(true, Ordering::Relaxed) {
            log::warn!("no local time zone ({e}): times are shown and written in UTC");
        }
        TimeZone::UTC
    })
}

/// The fraction's digits [`rfc3339`] writes for `t`: none for a whole
/// second, else 3, 6 or 9, whichever holds it.
fn digits_needed(t: Timestamp) -> u8 {
    match t.subsec_nanosecond() {
        0 => 0,
        n if n % 1_000_000 == 0 => 3,
        n if n % 1_000 == 0 => 6,
        _ => 9,
    }
}

/// `t` at `offset`, written `+00:00` at UTC rather than `Z`, its fraction
/// cut, not rounded, to `digits`.
fn at_offset(t: Timestamp, offset: Offset, digits: u8) -> String {
    DateTimePrinter::new()
        .precision(Some(digits))
        .timestamp_with_offset_to_string(&t, offset)
}

/// Held at jiff's range, the years -9999 to 9999, rather than refused: no
/// clock reads a time outside it.
fn timestamp(t: SystemTime) -> Timestamp {
    Timestamp::try_from(t).unwrap_or(if t < SystemTime::UNIX_EPOCH {
        Timestamp::MIN
    } else {
        Timestamp::MAX
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 2026-09-15T12:30:05Z.
    const SECOND: u64 = 1_789_475_405;

    /// `nanos` in whole 100 ns, the tick Windows' `SystemTime` counts in:
    /// a finer one is lost before the time is written.
    fn at(nanos: u32) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::new(SECOND, nanos)
    }

    /// `at(nanos)` as [`rfc3339`] writes it at `offset_secs` east of UTC,
    /// whatever zone the test runs in.
    fn written(nanos: u32, offset_secs: i32) -> String {
        let t = timestamp(at(nanos));
        at_offset(
            t,
            Offset::from_seconds(offset_secs).unwrap(),
            digits_needed(t),
        )
    }

    #[test]
    fn the_fraction_takes_the_digits_it_needs_in_threes() {
        assert_eq!(written(0, 7200), "2026-09-15T14:30:05+02:00");
        assert_eq!(written(250_000_000, 7200), "2026-09-15T14:30:05.250+02:00");
        assert_eq!(
            written(250_100_000, 7200),
            "2026-09-15T14:30:05.250100+02:00"
        );
        assert_eq!(
            written(250_000_100, 7200),
            "2026-09-15T14:30:05.250000100+02:00"
        );
    }

    #[test]
    fn utc_and_offsets_west_are_written_as_offsets() {
        assert_eq!(written(0, 0), "2026-09-15T12:30:05+00:00");
        assert_eq!(written(0, -12_600), "2026-09-15T09:00:05-03:30");
    }

    #[test]
    fn the_millisecond_form_always_writes_three_digits() {
        let millis = |nanos| at_offset(timestamp(at(nanos)), Offset::UTC, 3);
        assert_eq!(millis(0), "2026-09-15T12:30:05.000+00:00");
        assert_eq!(
            millis(250_999_999),
            "2026-09-15T12:30:05.250+00:00",
            "truncated, not rounded"
        );
    }

    #[test]
    fn what_is_written_reads_back() {
        for nanos in [0, 250_000_000, 250_100_000, 250_000_100] {
            assert_eq!(parse(&rfc3339(at(nanos))).unwrap(), at(nanos));
        }
        assert_eq!(parse("2026-09-15T12:30:05Z").unwrap(), at(0));
        assert_eq!(
            parse("2026-09-15T14:30:05.250+02:00").unwrap(),
            at(250_000_000)
        );
        assert!(parse("2026-09-15 14:30").is_err());
    }
}
