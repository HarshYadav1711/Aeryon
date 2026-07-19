//! Channel-change observation types.

use aeryon_domain::{SensorId, Timestamp};

use crate::evidence::ObservationEvidence;

/// Provenance label for the heuristic reliability field.
pub const RELIABILITY_PROVENANCE: &str = "heuristic-threshold-reliability-v1";

/// Typed channel-change intensity state.
///
/// These labels describe measured WiFi channel variation only. They are not
/// occupancy, presence, walking, animal, or object-activity classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelChangeState {
    /// Channel-change score below the stable threshold.
    Stable,
    /// Channel-change score between stable and highly-changing thresholds.
    Changing,
    /// Channel-change score at or above the highly-changing threshold.
    HighlyChanging,
    /// State could not be determined from available evidence / data quality.
    Indeterminate,
}

impl ChannelChangeState {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Changing => "changing",
            Self::HighlyChanging => "highly_changing",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// Uncertainty / data-quality quantities (not a probability).
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationUncertainty {
    /// Distance from the nearest classification threshold.
    pub threshold_margin: f64,
    /// Threshold margin normalized by the Stable↔HighlyChanging span.
    pub normalized_threshold_margin: f64,
    /// Capture-time timestamp jitter from the feature vector.
    pub timestamp_jitter: f64,
    /// Number of warnings retained on the observation.
    pub warning_count: u32,
    /// Supporting frame count from the source window.
    pub supporting_frame_count: u32,
    /// Number of valid antenna links.
    pub valid_antenna_links: u32,
    /// Conservative heuristic reliability in `[0, 1]` (not a probability).
    pub reliability_score: f64,
    /// Explicit provenance for the reliability heuristic.
    pub reliability_provenance: String,
}

/// Immutable channel-change observation.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelChangeObservation {
    /// Observation identity.
    pub observation_id: u64,
    /// Sensor identity.
    pub sensor_id: SensorId,
    /// Source feature-vector identity.
    pub feature_vector_id: u64,
    /// Source DSP window identity.
    pub window_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// First capture timestamp.
    pub first_capture_timestamp: Timestamp,
    /// Last capture timestamp.
    pub last_capture_timestamp: Timestamp,
    /// Channel-change state.
    pub state: ChannelChangeState,
    /// Heuristic activity score in `[0, 1]` (not a probability).
    pub activity_score: f64,
    /// Explicit score semantics label.
    pub score_semantics: String,
    /// Threshold profile identity.
    pub threshold_profile_id: String,
    /// Threshold profile version.
    pub threshold_profile_version: u32,
    /// Structured evidence.
    pub evidence: ObservationEvidence,
    /// Uncertainty / reliability metadata.
    pub uncertainty: ObservationUncertainty,
    /// Feature schema identity.
    pub feature_schema_id: String,
    /// Feature schema version.
    pub feature_schema_version: u32,
    /// Feature profile identity.
    pub feature_profile_id: String,
    /// Feature profile version.
    pub feature_profile_version: u32,
    /// DSP profile identity.
    pub dsp_profile_id: String,
    /// DSP profile version.
    pub dsp_profile_version: u32,
    /// DSP backend identity.
    pub dsp_backend_id: String,
    /// DSP backend implementation version.
    pub dsp_backend_version: String,
    /// Creation timestamp.
    pub created_at: Timestamp,
    /// Warnings.
    pub warnings: Vec<String>,
}
