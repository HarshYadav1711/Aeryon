//! Structured evidence for a channel-change observation.

use aeryon_features::FeatureId;

/// One named feature contribution retained as evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureEvidence {
    /// Feature identity.
    pub feature_id: FeatureId,
    /// Raw feature value.
    pub value: f64,
    /// Contribution after normalization into the score, when applicable.
    pub normalized_contribution: Option<f64>,
}

/// Concise structured evidence for a channel-change decision.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationEvidence {
    /// Features consulted.
    pub features: Vec<FeatureEvidence>,
    /// Resulting heuristic activity score.
    pub activity_score: f64,
    /// Stable-state threshold.
    pub stable_threshold: f64,
    /// Highly-changing threshold.
    pub high_change_threshold: f64,
    /// Distance from the nearest threshold.
    pub threshold_margin: f64,
    /// Data-quality warnings considered.
    pub data_quality_warnings: Vec<String>,
}
