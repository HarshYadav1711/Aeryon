//! Versioned channel-change observation profile and TOML configuration.

use serde::Deserialize;

use aeryon_features::{
    CSI_CHANNEL_FEATURES_V1_ID, CSI_CHANNEL_FEATURES_V1_VERSION, FeatureId, csi_channel_features_v1,
};

use crate::errors::PerceptionError;

/// Built-in channel-change observation profile identity.
pub const CHANNEL_CHANGE_V1_ID: &str = "channel-change-v1";

/// Built-in channel-change observation profile version.
pub const CHANNEL_CHANGE_V1_VERSION: u32 = 1;

/// Versioned observation threshold profile.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelChangeProfile {
    /// Profile identity.
    pub id: String,
    /// Profile version.
    pub version: u32,
    /// Required feature schema identity.
    pub feature_schema_id: String,
    /// Required feature schema version.
    pub feature_schema_version: u32,
    /// Feature identifiers used by the score.
    pub feature_ids: Vec<FeatureId>,
    /// Normalization scale for motion-energy RMS (positive).
    pub motion_energy_rms_scale: f64,
    /// Normalization scale for motion-energy p95 (positive).
    pub motion_energy_p95_scale: f64,
    /// Score below this value → Stable.
    pub stable_threshold: f64,
    /// Score at/above this value → HighlyChanging.
    pub high_change_threshold: f64,
    /// Minimum acceptable distance from a threshold boundary.
    pub minimum_margin: f64,
    /// Maximum accepted timestamp jitter before Indeterminate.
    pub maximum_timestamp_jitter: f64,
    /// Profile description.
    pub description: String,
}

impl ChannelChangeProfile {
    /// Validates profile invariants.
    pub fn validate(&self) -> Result<(), PerceptionError> {
        if self.id.trim().is_empty() {
            return Err(PerceptionError::InvalidProfile {
                message: "observation profile id must not be empty".to_owned(),
            });
        }
        if self.version == 0 {
            return Err(PerceptionError::InvalidProfile {
                message: "observation profile version must be >= 1".to_owned(),
            });
        }
        if self.feature_schema_id != CSI_CHANNEL_FEATURES_V1_ID
            || self.feature_schema_version != CSI_CHANNEL_FEATURES_V1_VERSION
        {
            return Err(PerceptionError::IncompatibleFeatureSchema {
                message: format!(
                    "unsupported feature schema `{}/{}`",
                    self.feature_schema_id, self.feature_schema_version
                ),
            });
        }
        let schema = csi_channel_features_v1();
        for feature in &self.feature_ids {
            if schema.definition(*feature).is_none() {
                return Err(PerceptionError::MissingFeatures {
                    message: format!("feature `{}` is not in the schema", feature.as_str()),
                });
            }
        }
        for (name, value) in [
            ("motion_energy_rms_scale", self.motion_energy_rms_scale),
            ("motion_energy_p95_scale", self.motion_energy_p95_scale),
            ("stable_threshold", self.stable_threshold),
            ("high_change_threshold", self.high_change_threshold),
            ("minimum_margin", self.minimum_margin),
            ("maximum_timestamp_jitter", self.maximum_timestamp_jitter),
        ] {
            if !value.is_finite() {
                return Err(PerceptionError::InvalidProfile {
                    message: format!("{name} must be finite"),
                });
            }
        }
        if self.motion_energy_rms_scale <= 0.0 || self.motion_energy_p95_scale <= 0.0 {
            return Err(PerceptionError::InvalidProfile {
                message: "normalization scales must be positive".to_owned(),
            });
        }
        if self.stable_threshold < 0.0 || self.high_change_threshold < 0.0 {
            return Err(PerceptionError::InvalidProfile {
                message: "thresholds must be non-negative".to_owned(),
            });
        }
        if self.stable_threshold >= self.high_change_threshold {
            return Err(PerceptionError::InvalidProfile {
                message: "stable_threshold must be strictly less than high_change_threshold"
                    .to_owned(),
            });
        }
        if self.minimum_margin < 0.0 {
            return Err(PerceptionError::InvalidProfile {
                message: "minimum_margin must be non-negative".to_owned(),
            });
        }
        if self.maximum_timestamp_jitter < 0.0 {
            return Err(PerceptionError::InvalidProfile {
                message: "maximum_timestamp_jitter must be non-negative".to_owned(),
            });
        }
        Ok(())
    }
}

/// Nested TOML overrides for [`CHANNEL_CHANGE_V1_ID`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChannelChangeV1Config {
    /// Normalization scale for motion-energy RMS.
    #[serde(default = "default_rms_scale")]
    pub motion_energy_rms_scale: f64,
    /// Normalization scale for motion-energy p95.
    #[serde(default = "default_p95_scale")]
    pub motion_energy_p95_scale: f64,
    /// Stable threshold.
    #[serde(default = "default_stable_threshold")]
    pub stable_threshold: f64,
    /// Highly-changing threshold.
    #[serde(default = "default_high_change_threshold")]
    pub high_change_threshold: f64,
    /// Minimum threshold margin.
    #[serde(default = "default_minimum_margin")]
    pub minimum_margin: f64,
    /// Maximum accepted timestamp jitter.
    #[serde(default = "default_maximum_timestamp_jitter")]
    pub maximum_timestamp_jitter: f64,
}

fn default_rms_scale() -> f64 {
    // Development baseline derived from synthetic_dev_v1 motion-energy magnitude
    // after RMS amplitude normalization (order ~0.1–1.0).
    0.35
}

fn default_p95_scale() -> f64 {
    0.55
}

fn default_stable_threshold() -> f64 {
    0.22
}

fn default_high_change_threshold() -> f64 {
    0.55
}

fn default_minimum_margin() -> f64 {
    // Zero allows exact threshold boundaries to classify cleanly.
    // Raise this for hysteresis when noisy scores sit on a threshold.
    0.0
}

fn default_maximum_timestamp_jitter() -> f64 {
    0.10
}

impl Default for ChannelChangeV1Config {
    fn default() -> Self {
        Self {
            motion_energy_rms_scale: default_rms_scale(),
            motion_energy_p95_scale: default_p95_scale(),
            stable_threshold: default_stable_threshold(),
            high_change_threshold: default_high_change_threshold(),
            minimum_margin: default_minimum_margin(),
            maximum_timestamp_jitter: default_maximum_timestamp_jitter(),
        }
    }
}

impl ChannelChangeV1Config {
    /// Builds a versioned profile from config overrides.
    pub fn to_profile(&self) -> ChannelChangeProfile {
        ChannelChangeProfile {
            id: CHANNEL_CHANGE_V1_ID.to_owned(),
            version: CHANNEL_CHANGE_V1_VERSION,
            feature_schema_id: CSI_CHANNEL_FEATURES_V1_ID.to_owned(),
            feature_schema_version: CSI_CHANNEL_FEATURES_V1_VERSION,
            feature_ids: vec![FeatureId::MotionEnergyRms, FeatureId::MotionEnergyP95],
            motion_energy_rms_scale: self.motion_energy_rms_scale,
            motion_energy_p95_scale: self.motion_energy_p95_scale,
            stable_threshold: self.stable_threshold,
            high_change_threshold: self.high_change_threshold,
            minimum_margin: self.minimum_margin,
            maximum_timestamp_jitter: self.maximum_timestamp_jitter,
            description: "Heuristic WiFi channel-change intensity from motion-energy RMS \
                 and p95. Development baseline thresholds — not a learned model, not a \
                 probability, and not human-presence detection."
                .to_owned(),
        }
    }
}

/// TOML-facing perception configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PerceptionConfig {
    /// Whether the perception worker is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Selected profile identity.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Bounded feature-vector input queue capacity.
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
    /// Nested overrides for `channel-change-v1`.
    #[serde(default)]
    pub channel_change_v1: ChannelChangeV1Config,
}

fn default_profile() -> String {
    CHANNEL_CHANGE_V1_ID.to_owned()
}

fn default_queue_capacity() -> usize {
    64
}

impl Default for PerceptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: default_profile(),
            queue_capacity: default_queue_capacity(),
            channel_change_v1: ChannelChangeV1Config::default(),
        }
    }
}

impl PerceptionConfig {
    /// Validates configuration values.
    pub fn validate(&self) -> Result<(), PerceptionError> {
        if !self.enabled {
            return Ok(());
        }
        if self.profile != CHANNEL_CHANGE_V1_ID {
            return Err(PerceptionError::InvalidProfile {
                message: format!(
                    "unsupported perception profile `{}`; only `{CHANNEL_CHANGE_V1_ID}` is available",
                    self.profile
                ),
            });
        }
        if self.queue_capacity == 0 {
            return Err(PerceptionError::InvalidProfile {
                message: "perception.queue_capacity must be greater than zero".to_owned(),
            });
        }
        let profile = self.resolve_profile()?;
        profile.validate()?;
        Ok(())
    }

    /// Resolves the selected versioned profile.
    pub fn resolve_profile(&self) -> Result<ChannelChangeProfile, PerceptionError> {
        if self.profile != CHANNEL_CHANGE_V1_ID {
            return Err(PerceptionError::InvalidProfile {
                message: format!("unknown perception profile `{}`", self.profile),
            });
        }
        let profile = self.channel_change_v1.to_profile();
        profile.validate()?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_validates() {
        ChannelChangeV1Config::default()
            .to_profile()
            .validate()
            .expect("valid");
    }

    #[test]
    fn unordered_thresholds_rejected() {
        let config = ChannelChangeV1Config {
            stable_threshold: 0.8,
            high_change_threshold: 0.2,
            ..ChannelChangeV1Config::default()
        };
        assert!(config.to_profile().validate().is_err());
    }
}
