//! Sensor identity contracts.

use crate::ids::SensorId;

/// Describes a sensing source that produces frames.
///
/// Subsystems use this trait to refer to sensors without depending on
/// acquisition or hardware details. Implementations live in plugin crates.
pub trait Sensor {
    /// Returns the stable identifier for this sensor.
    fn id(&self) -> SensorId;

    /// Returns a human-readable sensor name for logging and diagnostics.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SensorId;

    struct BenchSensor {
        id: SensorId,
        name: &'static str,
    }

    impl Sensor for BenchSensor {
        fn id(&self) -> SensorId {
            self.id
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn sensor_exposes_id_and_name() {
        let sensor = BenchSensor {
            id: SensorId::new(7),
            name: "bench-sensor",
        };
        assert_eq!(sensor.id(), SensorId::new(7));
        assert_eq!(sensor.name(), "bench-sensor");
    }
}
