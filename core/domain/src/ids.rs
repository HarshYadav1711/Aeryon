//! Strongly typed identifiers shared across the perception platform.
//!
//! Newtypes prevent accidental mixing of unrelated identifiers at compile time.

use core::fmt;

macro_rules! define_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// Creates an identifier from its raw numeric value.
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the raw numeric value.
            pub const fn value(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(SensorId, "Identifies a physical or logical sensor.");
define_id!(FrameId, "Identifies a single acquired sensor frame.");
define_id!(
    ObservationId,
    "Identifies a structured observation derived from sensor data."
);
define_id!(MissionId, "Identifies a mission or operational context.");
define_id!(EntityId, "Identifies an entity tracked in the world model.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_types() {
        let sensor = SensorId::new(1);
        let frame = FrameId::new(1);
        assert_eq!(sensor.value(), frame.value());
        // Distinct types prevent accidental cross-assignment at compile time.
    }

    #[test]
    fn id_display_contains_type_name() {
        let id = MissionId::new(42);
        assert!(id.to_string().contains("MissionId"));
        assert!(id.to_string().contains("42"));
    }

    #[test]
    fn ids_order_by_value() {
        assert!(EntityId::new(1) < EntityId::new(2));
    }
}
