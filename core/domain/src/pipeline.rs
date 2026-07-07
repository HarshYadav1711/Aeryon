//! Pipeline stage contracts.

/// Identifier for a stage within a perception pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PipelineStageId(u32);

impl PipelineStageId {
    /// Creates a stage identifier from its numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Canonical pipeline stage categories shared across deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStageKind {
    /// Raw sensor frame ingestion.
    Acquisition,
    /// Sensor correction and normalization.
    Calibration,
    /// Signal processing.
    SignalProcessing,
    /// Feature extraction.
    FeatureExtraction,
    /// Model or rule execution.
    Inference,
    /// Multi-source fusion into scene interpretations.
    Perception,
    /// World model update.
    WorldUpdate,
}

/// Describes a stage in the perception pipeline.
///
/// Pipeline orchestration uses this trait to enumerate stages without coupling
/// to concrete subsystem implementations.
pub trait PipelineStage {
    /// Returns the stable stage identifier.
    fn id(&self) -> PipelineStageId;

    /// Returns the canonical stage category.
    fn kind(&self) -> PipelineStageKind;

    /// Returns a human-readable stage name.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BenchStage {
        id: PipelineStageId,
        kind: PipelineStageKind,
        name: &'static str,
    }

    impl PipelineStage for BenchStage {
        fn id(&self) -> PipelineStageId {
            self.id
        }

        fn kind(&self) -> PipelineStageKind {
            self.kind
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn pipeline_stage_exposes_identity() {
        let stage = BenchStage {
            id: PipelineStageId::new(3),
            kind: PipelineStageKind::FeatureExtraction,
            name: "feature-extraction",
        };
        assert_eq!(stage.id(), PipelineStageId::new(3));
        assert_eq!(stage.kind(), PipelineStageKind::FeatureExtraction);
        assert_eq!(stage.name(), "feature-extraction");
    }

    #[test]
    fn stage_ids_are_ordered() {
        assert!(PipelineStageId::new(1) < PipelineStageId::new(2));
    }
}
