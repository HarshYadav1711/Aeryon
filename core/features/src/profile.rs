//! Versioned feature-extraction profile and TOML-facing configuration.

use serde::Deserialize;

use crate::aggregate::AggregationPolicy;
use crate::errors::FeatureError;
use crate::schema::{
    CSI_CHANNEL_FEATURES_V1_ID, CSI_CHANNEL_FEATURES_V1_VERSION, FeatureId, FeatureSchema,
    csi_channel_features_v1,
};
use crate::spectral::FrequencyBandPolicy;
use aeryon_dsp::{BASELINE_DSP_V1_ID, BASELINE_DSP_V1_VERSION};

/// Built-in baseline feature profile identity.
pub const BASELINE_FEATURES_V1_ID: &str = "baseline-features-v1";

/// Built-in baseline feature profile version.
pub const BASELINE_FEATURES_V1_VERSION: u32 = 1;

/// Percentile interpolation convention recorded in the profile.
pub const PERCENTILE_CONVENTION: &str = "linear_sorted_copy";

/// Versioned feature-extraction profile.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureProfile {
    /// Profile identity.
    pub id: String,
    /// Profile version.
    pub version: u32,
    /// Compatible DSP profile identity.
    pub dsp_profile_id: String,
    /// Compatible DSP profile version.
    pub dsp_profile_version: u32,
    /// Feature schema identity.
    pub feature_schema_id: String,
    /// Feature schema version.
    pub feature_schema_version: u32,
    /// Relative frequency-band policy.
    pub frequency_band_policy: FrequencyBandPolicy,
    /// Percentile interpolation convention label.
    pub percentile_convention: String,
    /// Positive epsilon for spectral flatness flooring.
    pub flatness_epsilon: f64,
    /// Cross-link aggregation policy.
    pub aggregation_policy: AggregationPolicy,
    /// Enabled features (unique, non-empty).
    pub enabled_features: Vec<FeatureId>,
}

impl FeatureProfile {
    /// Validates profile invariants.
    pub fn validate(&self) -> Result<(), FeatureError> {
        if self.id.trim().is_empty() {
            return Err(FeatureError::InvalidProfile {
                message: "feature profile id must not be empty".to_owned(),
            });
        }
        if self.version == 0 {
            return Err(FeatureError::InvalidProfile {
                message: "feature profile version must be >= 1".to_owned(),
            });
        }
        if self.dsp_profile_id != BASELINE_DSP_V1_ID
            || self.dsp_profile_version != BASELINE_DSP_V1_VERSION
        {
            return Err(FeatureError::IncompatibleDspProfile {
                message: format!(
                    "profile requires DSP `{}/{}`; got `{}/{}`",
                    BASELINE_DSP_V1_ID,
                    BASELINE_DSP_V1_VERSION,
                    self.dsp_profile_id,
                    self.dsp_profile_version
                ),
            });
        }
        if self.feature_schema_id != CSI_CHANNEL_FEATURES_V1_ID
            || self.feature_schema_version != CSI_CHANNEL_FEATURES_V1_VERSION
        {
            return Err(FeatureError::SchemaMismatch {
                message: format!(
                    "unsupported feature schema `{}/{}`",
                    self.feature_schema_id, self.feature_schema_version
                ),
            });
        }
        if self.enabled_features.is_empty() {
            return Err(FeatureError::InvalidProfile {
                message: "enabled feature list must not be empty".to_owned(),
            });
        }
        let mut seen = Vec::with_capacity(self.enabled_features.len());
        for feature in &self.enabled_features {
            if seen.contains(feature) {
                return Err(FeatureError::InvalidProfile {
                    message: format!("duplicate enabled feature `{}`", feature.as_str()),
                });
            }
            seen.push(*feature);
        }
        if !self.flatness_epsilon.is_finite() || self.flatness_epsilon <= 0.0 {
            return Err(FeatureError::InvalidProfile {
                message: "flatness_epsilon must be finite and positive".to_owned(),
            });
        }
        Ok(())
    }

    /// Resolves the feature schema referenced by this profile.
    pub fn schema(&self) -> Result<FeatureSchema, FeatureError> {
        let schema = csi_channel_features_v1();
        schema.assert_compatible(&self.feature_schema_id, self.feature_schema_version)?;
        schema.validate()?;
        Ok(schema)
    }

    /// Checks DSP result provenance against this profile.
    pub fn assert_dsp_compatible(
        &self,
        dsp_profile_id: &str,
        dsp_profile_version: u32,
    ) -> Result<(), FeatureError> {
        if dsp_profile_id != self.dsp_profile_id || dsp_profile_version != self.dsp_profile_version
        {
            return Err(FeatureError::IncompatibleDspProfile {
                message: format!(
                    "DSP result `{dsp_profile_id}/{dsp_profile_version}` incompatible with feature profile `{}/{}`",
                    self.id, self.version
                ),
            });
        }
        Ok(())
    }
}

/// Returns the built-in baseline feature profile.
pub fn baseline_features_v1() -> FeatureProfile {
    FeatureProfile {
        id: BASELINE_FEATURES_V1_ID.to_owned(),
        version: BASELINE_FEATURES_V1_VERSION,
        dsp_profile_id: BASELINE_DSP_V1_ID.to_owned(),
        dsp_profile_version: BASELINE_DSP_V1_VERSION,
        feature_schema_id: CSI_CHANNEL_FEATURES_V1_ID.to_owned(),
        feature_schema_version: CSI_CHANNEL_FEATURES_V1_VERSION,
        frequency_band_policy: FrequencyBandPolicy::RelativeNonDcThirds,
        percentile_convention: PERCENTILE_CONVENTION.to_owned(),
        flatness_epsilon: 1.0e-12,
        aggregation_policy: AggregationPolicy::PreferDspAggregateThenMean,
        enabled_features: FeatureId::ALL.to_vec(),
    }
}

/// TOML-facing feature configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FeaturesConfig {
    /// Whether the feature worker is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Selected profile identity.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Bounded DSP-result input queue capacity.
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
}

fn default_profile() -> String {
    BASELINE_FEATURES_V1_ID.to_owned()
}

fn default_queue_capacity() -> usize {
    64
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: default_profile(),
            queue_capacity: default_queue_capacity(),
        }
    }
}

impl FeaturesConfig {
    /// Validates configuration values.
    pub fn validate(&self) -> Result<(), FeatureError> {
        if !self.enabled {
            return Ok(());
        }
        if self.profile != BASELINE_FEATURES_V1_ID {
            return Err(FeatureError::InvalidProfile {
                message: format!(
                    "unsupported features profile `{}`; only `{BASELINE_FEATURES_V1_ID}` is available",
                    self.profile
                ),
            });
        }
        if self.queue_capacity == 0 {
            return Err(FeatureError::InvalidProfile {
                message: "features.queue_capacity must be greater than zero".to_owned(),
            });
        }
        let profile = self.resolve_profile()?;
        profile.validate()?;
        Ok(())
    }

    /// Resolves the selected versioned profile.
    pub fn resolve_profile(&self) -> Result<FeatureProfile, FeatureError> {
        if self.profile != BASELINE_FEATURES_V1_ID {
            return Err(FeatureError::InvalidProfile {
                message: format!("unknown features profile `{}`", self.profile),
            });
        }
        let profile = baseline_features_v1();
        profile.validate()?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_profile_validates() {
        let profile = baseline_features_v1();
        profile.validate().expect("valid");
        assert_eq!(profile.enabled_features.len(), FeatureId::ALL.len());
    }

    #[test]
    fn disabled_config_ok() {
        FeaturesConfig::default().validate().expect("disabled ok");
    }
}
