//! Synthetic frame payload implementing the domain [`Frame`] contract.

use aeryon_domain::{Frame, FrameMetadata, Metadata, MetadataKey, MetadataValue};

/// Stable source marker stored in frame metadata.
pub const SOURCE_ID: &str = "synthetic";

/// A synthetic sensor frame with a numerical sample payload.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticFrame {
    /// Frame header shared with the platform domain model.
    pub metadata: FrameMetadata,
    /// Deterministic numerical samples.
    pub samples: Vec<f64>,
}

impl SyntheticFrame {
    /// Creates a synthetic frame and stamps the synthetic source identifier.
    pub fn new(mut metadata: FrameMetadata, samples: Vec<f64>) -> Self {
        metadata.metadata.insert(
            MetadataKey::Source,
            MetadataValue::Text(SOURCE_ID.to_owned()),
        );
        metadata.metadata.insert(
            MetadataKey::Custom(std::borrow::Cow::Borrowed("synthetic")),
            MetadataValue::Bool(true),
        );
        Self { metadata, samples }
    }

    /// Returns an empty metadata container for callers that need one.
    pub fn empty_metadata() -> Metadata {
        Metadata::new()
    }
}

impl Frame for SyntheticFrame {
    type Payload = Vec<f64>;

    fn metadata(&self) -> &FrameMetadata {
        &self.metadata
    }

    fn payload(&self) -> &Self::Payload {
        &self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_domain::{FrameId, SensorId, Timestamp};

    #[test]
    fn frame_marks_synthetic_source() {
        let frame = SyntheticFrame::new(
            FrameMetadata {
                frame_id: FrameId::new(1),
                sensor_id: SensorId::new(1),
                timestamp: Timestamp::from_nanos(1),
                sequence: 0,
                mission_id: None,
                metadata: Metadata::new(),
            },
            vec![0.0, 1.0],
        );
        assert_eq!(
            frame.metadata().metadata.get(&MetadataKey::Source),
            Some(&MetadataValue::Text(SOURCE_ID.to_owned()))
        );
        assert_eq!(frame.payload().len(), 2);
    }
}
