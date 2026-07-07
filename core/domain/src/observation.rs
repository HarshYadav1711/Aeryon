//! Observation data model.

use crate::frame::Metadata;
use crate::ids::{EntityId, FrameId, ObservationId, SensorId};
use crate::time::Timestamp;

/// Normalized confidence score in the closed interval `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f64);

impl Confidence {
    /// Creates a confidence value if `value` lies in `[0.0, 1.0]`.
    pub fn new(value: f64) -> Option<Self> {
        if (0.0..=1.0).contains(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the raw confidence score.
    pub fn value(self) -> f64 {
        self.0
    }
}

/// Modality-agnostic observation payload.
#[derive(Debug, Clone, PartialEq)]
pub enum ObservationValue {
    /// Boolean state.
    Bool(bool),
    /// Numeric measurement.
    Quantity {
        /// Measured value.
        value: f64,
        /// Unit identifier (for example `"m"` or `"celsius"`).
        unit: String,
    },
    /// Discrete categorical label.
    Category(String),
    /// Opaque binary payload for modality-specific encodings.
    Blob(Vec<u8>),
}

/// A structured interpretation derived from sensor data.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// Unique observation identifier.
    pub id: ObservationId,
    /// Time the observation was recorded.
    pub timestamp: Timestamp,
    /// Source frame identifier.
    pub frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Optional entity associations.
    pub entity_ids: Vec<EntityId>,
    /// Confidence in the observation.
    pub confidence: Confidence,
    /// Observation payload.
    pub value: ObservationValue,
    /// Additional observation metadata.
    pub metadata: Metadata,
}

impl Observation {
    /// Returns `true` when the observation references at least one entity.
    pub fn has_entities(&self) -> bool {
        !self.entity_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{MetadataKey, MetadataValue};
    use crate::ids::{FrameId, ObservationId, SensorId};

    #[test]
    fn confidence_rejects_out_of_range_values() {
        assert!(Confidence::new(-0.1).is_none());
        assert!(Confidence::new(1.1).is_none());
        assert_eq!(Confidence::new(0.75).map(Confidence::value), Some(0.75));
    }

    #[test]
    fn observation_tracks_entity_associations() {
        let observation = Observation {
            id: ObservationId::new(1),
            timestamp: Timestamp::from_nanos(1),
            frame_id: FrameId::new(2),
            sensor_id: SensorId::new(3),
            entity_ids: vec![EntityId::new(4)],
            confidence: Confidence::new(0.9).expect("valid confidence"),
            value: ObservationValue::Bool(true),
            metadata: Metadata::new(),
        };
        assert!(observation.has_entities());
    }

    #[test]
    fn metadata_can_attach_to_observations() {
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::Source, MetadataValue::Text("features".into()));
        let observation = Observation {
            id: ObservationId::new(1),
            timestamp: Timestamp::from_nanos(1),
            frame_id: FrameId::new(2),
            sensor_id: SensorId::new(3),
            entity_ids: Vec::new(),
            confidence: Confidence::new(1.0).expect("valid confidence"),
            value: ObservationValue::Category("present".into()),
            metadata,
        };
        assert_eq!(
            observation
                .metadata
                .get(&MetadataKey::Source)
                .and_then(|value| match value {
                    MetadataValue::Text(text) => Some(text.as_str()),
                    _ => None,
                }),
            Some("features")
        );
    }
}
