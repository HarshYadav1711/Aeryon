//! Frame contracts and shared metadata containers.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use crate::ids::{FrameId, MissionId, SensorId};
use crate::time::Timestamp;

/// Typed metadata field keys.
///
/// Known keys are enumerated explicitly; custom keys remain available without
/// resorting to untyped string dispatch for core platform fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetadataKey {
    /// Operational mission associated with the data.
    Mission,
    /// Monotonic sequence number within a sensor stream.
    Sequence,
    /// Originating subsystem or component name.
    Source,
    /// Application-defined extension key.
    Custom(Cow<'static, str>),
}

/// Typed metadata values stored alongside domain objects.
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// UTF-8 text.
    Text(String),
    /// IEEE-754 floating-point number.
    Float(f64),
    /// Signed integer.
    Integer(i64),
    /// Boolean flag.
    Bool(bool),
    /// Absolute timestamp.
    Timestamp(Timestamp),
}

/// Structured key-value metadata attached to frames, observations, and world objects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    fields: BTreeMap<MetadataKey, MetadataValue>,
}

impl Metadata {
    /// Creates empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a metadata entry.
    pub fn insert(&mut self, key: MetadataKey, value: MetadataValue) {
        self.fields.insert(key, value);
    }

    /// Returns the value associated with `key`, if present.
    pub fn get(&self, key: &MetadataKey) -> Option<&MetadataValue> {
        self.fields.get(key)
    }

    /// Returns an iterator over metadata entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&MetadataKey, &MetadataValue)> {
        self.fields.iter()
    }

    /// Returns `true` when no metadata entries are stored.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Descriptive header for a single acquired frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameMetadata {
    /// Unique frame identifier.
    pub frame_id: FrameId,
    /// Sensor that produced the frame.
    pub sensor_id: SensorId,
    /// Acquisition timestamp.
    pub timestamp: Timestamp,
    /// Monotonic sequence number within the sensor stream.
    pub sequence: u64,
    /// Optional mission context.
    pub mission_id: Option<MissionId>,
    /// Additional frame-level metadata.
    pub metadata: Metadata,
}

/// A single acquisition unit from a sensor.
///
/// The associated `Payload` type lets each modality define its own raw data
/// representation while sharing common frame metadata.
pub trait Frame {
    /// Modality-specific frame payload type.
    type Payload;

    /// Returns descriptive metadata for the frame.
    fn metadata(&self) -> &FrameMetadata;

    /// Returns the modality-specific payload.
    fn payload(&self) -> &Self::Payload;
}

impl fmt::Display for MetadataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mission => f.write_str("mission"),
            Self::Sequence => f.write_str("sequence"),
            Self::Source => f.write_str("source"),
            Self::Custom(key) => f.write_str(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FrameId, SensorId};

    struct SampleFrame {
        header: FrameMetadata,
        payload: Vec<u8>,
    }

    impl Frame for SampleFrame {
        type Payload = Vec<u8>;

        fn metadata(&self) -> &FrameMetadata {
            &self.header
        }

        fn payload(&self) -> &Self::Payload {
            &self.payload
        }
    }

    fn sample_metadata() -> FrameMetadata {
        FrameMetadata {
            frame_id: FrameId::new(1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(100),
            sequence: 0,
            mission_id: None,
            metadata: Metadata::new(),
        }
    }

    #[test]
    fn frame_exposes_metadata_and_payload() {
        let frame = SampleFrame {
            header: sample_metadata(),
            payload: vec![0x01, 0x02],
        };
        assert_eq!(frame.metadata().frame_id, FrameId::new(1));
        assert_eq!(frame.payload(), &[0x01, 0x02]);
    }

    #[test]
    fn metadata_stores_typed_values() {
        let mut metadata = Metadata::new();
        metadata.insert(
            MetadataKey::Sequence,
            MetadataValue::Integer(9),
        );
        assert_eq!(
            metadata.get(&MetadataKey::Sequence),
            Some(&MetadataValue::Integer(9))
        );
    }

    #[test]
    fn metadata_iterates_in_key_order() {
        let mut metadata = Metadata::new();
        metadata.insert(
            MetadataKey::Custom(Cow::Borrowed("z")),
            MetadataValue::Bool(true),
        );
        metadata.insert(MetadataKey::Mission, MetadataValue::Integer(1));
        let keys: Vec<_> = metadata.iter().map(|(key, _)| key.clone()).collect();
        assert_eq!(keys.first(), Some(&MetadataKey::Mission));
    }
}
