//! Deterministic CSI channel feature extraction for Aeryon.
//!
//! # Honesty
//!
//! - Features are deterministic signal descriptors, not activity labels.
//! - They describe measured WiFi channel variation within a DSP window.
//! - They do not claim human presence, occupancy, pose, identity, or vital signs.
//! - Later ML models may consume the same versioned schema; this crate does not run ML.

#![deny(missing_docs)]

pub mod aggregate;
pub mod errors;
pub mod extractor;
pub mod profile;
pub mod report;
pub mod schema;
pub mod service;
pub mod spectral;
pub mod statistics;
pub mod stats;
pub mod vector;

pub use aggregate::{AggregationPolicy, aggregate_feature_values, uses_max_aggregation};
pub use errors::FeatureError;
pub use extractor::extract_features;
pub use profile::{
    BASELINE_FEATURES_V1_ID, BASELINE_FEATURES_V1_VERSION, FeatureProfile, FeaturesConfig,
    PERCENTILE_CONVENTION, baseline_features_v1,
};
pub use report::FeatureExtractionReport;
pub use schema::{
    CSI_CHANNEL_FEATURES_V1_ID, CSI_CHANNEL_FEATURES_V1_VERSION, FeatureAggregationScope,
    FeatureDefinition, FeatureId, FeatureSchema, csi_channel_features_v1,
};
pub use service::{DspResultRx, DspResultTx, FeatureService, FeatureVectorSink, FeatureVectorTx};
pub use spectral::{
    FrequencyBandPolicy, SpectralFeatures, extract_spectral_features, normalized_spectral_entropy,
    spectral_flatness,
};
pub use statistics::{
    max, mean, mean_absolute_delta, median, min, peak_to_mean_ratio, percentile, population_std,
    require_finite_non_empty, require_finite_output, rms,
};
pub use stats::{FeatureStats, FeatureWorkerState};
pub use vector::{FeatureVector, FeatureVectorStatus, LinkFeatureValues};

/// Subsystem identifier.
pub const ID: &str = "features";

/// Returns the subsystem name.
pub fn name() -> &'static str {
    ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_matches_id() {
        assert_eq!(name(), ID);
    }
}
