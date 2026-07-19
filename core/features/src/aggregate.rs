//! Cross-link aggregation policies for per-link feature collections.

use crate::errors::FeatureError;
use crate::schema::FeatureId;
use crate::statistics::{mean, require_finite_output};

/// Explicit policy for building the aggregate feature vector from per-link values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationPolicy {
    /// Prefer DSP aggregate motion-energy/spectrum when present; otherwise mean/max policy.
    PreferDspAggregateThenMean,
}

impl AggregationPolicy {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreferDspAggregateThenMean => "prefer_dsp_aggregate_then_mean",
        }
    }
}

/// Features whose cross-link fallback aggregation uses a maximum rather than a mean.
pub fn uses_max_aggregation(id: FeatureId) -> bool {
    matches!(
        id,
        FeatureId::MotionEnergyMaximum
            | FeatureId::MotionEnergyP95
            | FeatureId::MotionEnergyP90
            | FeatureId::MotionEnergyRange
            | FeatureId::MotionEnergyPeakToMeanRatio
            | FeatureId::DominantNonDcPower
            | FeatureId::TotalNonDcPower
    )
}

/// Aggregates one feature across valid link values using the documented policy.
///
/// Dominant frequency is **not** averaged: callers should take the frequency from
/// the highest-power link (or the DSP aggregate spectrum) before calling this.
pub fn aggregate_feature_values(
    id: FeatureId,
    values: &[f64],
    policy: AggregationPolicy,
) -> Result<f64, FeatureError> {
    let AggregationPolicy::PreferDspAggregateThenMean = policy;
    if values.is_empty() {
        return Err(FeatureError::EmptySignal {
            context: format!(" (aggregate {})", id.as_str()),
        });
    }
    for value in values {
        if !value.is_finite() {
            return Err(FeatureError::NonFiniteInput {
                context: format!(" (aggregate {})", id.as_str()),
                message: "link feature value is non-finite".to_owned(),
            });
        }
    }
    let aggregated = if uses_max_aggregation(id) {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    } else {
        mean(values)?
    };
    require_finite_output(aggregated, id.as_str())
}
