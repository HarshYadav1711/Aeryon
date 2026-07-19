//! Timestamp helpers for stable API wire formats.

use std::time::{Duration, SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};

/// Formats a [`SystemTime`] as an RFC 3339 UTC timestamp.
pub fn system_time_to_rfc3339(time: SystemTime) -> String {
    let datetime = DateTime::<Utc>::from(time);
    datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Formats Unix-epoch nanoseconds as an RFC 3339 UTC timestamp.
pub fn nanos_to_rfc3339(nanos: u64) -> String {
    let secs = i64::try_from(nanos / 1_000_000_000).unwrap_or(i64::MAX);
    let nsecs = u32::try_from(nanos % 1_000_000_000).unwrap_or(0);
    DateTime::<Utc>::from_timestamp(secs, nsecs)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(SecondsFormat::Nanos, true)
}

/// Returns the current wall-clock time as RFC 3339 UTC.
pub fn now_rfc3339() -> String {
    system_time_to_rfc3339(SystemTime::now())
}

/// Converts uptime into whole seconds with fractional millis.
pub fn duration_secs(duration: Duration) -> f64 {
    duration.as_secs_f64()
}
