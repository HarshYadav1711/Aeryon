//! Temporal primitives for ordering and correlating platform data.

use core::fmt;

/// An absolute point in time represented as nanoseconds since the Unix epoch.
///
/// All subsystems use this type so timestamps remain comparable across modules
/// without implicit unit conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    nanos: u64,
}

impl Timestamp {
    /// Creates a timestamp from nanoseconds since the Unix epoch.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Returns nanoseconds since the Unix epoch.
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_ordered() {
        let earlier = Timestamp::from_nanos(100);
        let later = Timestamp::from_nanos(200);
        assert!(earlier < later);
    }

    #[test]
    fn timestamp_round_trips_nanos() {
        let ts = Timestamp::from_nanos(1_705_000_000_000_000_000);
        assert_eq!(ts.as_nanos(), 1_705_000_000_000_000_000);
    }
}
