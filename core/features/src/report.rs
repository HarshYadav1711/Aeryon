//! Concise feature-extraction report (metadata only).

use aeryon_domain::Timestamp;

use crate::vector::FeatureVectorStatus;

/// Metadata report for one feature extraction attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureExtractionReport {
    /// Source DSP window identity.
    pub window_id: u64,
    /// Feature profile identity.
    pub profile_id: String,
    /// Feature profile version.
    pub profile_version: u32,
    /// Feature schema identity.
    pub schema_id: String,
    /// Feature schema version.
    pub schema_version: u32,
    /// Number of features requested by the profile/schema.
    pub features_requested: usize,
    /// Number of aggregate features produced.
    pub features_produced: usize,
    /// Number of antenna links featured.
    pub link_count: usize,
    /// Extraction start timestamp.
    pub started_at: Timestamp,
    /// Extraction completion timestamp.
    pub completed_at: Timestamp,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
    /// Outcome status.
    pub status: FeatureVectorStatus,
}
