//! Versioned calibration profiles and TOML-facing configuration.

use serde::Deserialize;

use crate::errors::CalibrationError;
use crate::stage::CalibrationStageId;
use crate::stages::{
    DEFAULT_RMS_EPSILON, LinearPhaseDetrendStage, PhaseUnwrapStage, RmsAmplitudeNormalizeStage,
};

/// Built-in baseline CSI calibration profile identity.
pub const BASELINE_CSI_V1_ID: &str = "baseline-csi-v1";

/// Built-in baseline CSI calibration profile version.
pub const BASELINE_CSI_V1_VERSION: u32 = 1;

/// Typed stage configuration deserialized from TOML / profile construction.
#[derive(Debug, Clone, PartialEq)]
pub enum StageConfig {
    /// Spatial phase unwrap.
    PhaseUnwrap {
        /// Whether this stage is enabled.
        enabled: bool,
    },
    /// Linear phase detrend.
    LinearPhaseDetrend {
        /// Whether this stage is enabled.
        enabled: bool,
    },
    /// RMS amplitude normalize.
    RmsAmplitudeNormalize {
        /// Whether this stage is enabled.
        enabled: bool,
        /// Positive finite epsilon for zero-energy rejection.
        epsilon: f32,
    },
}

impl StageConfig {
    /// Stage identity for this configuration entry.
    pub fn stage_id(&self) -> CalibrationStageId {
        match self {
            Self::PhaseUnwrap { .. } => CalibrationStageId::PhaseUnwrap,
            Self::LinearPhaseDetrend { .. } => CalibrationStageId::LinearPhaseDetrend,
            Self::RmsAmplitudeNormalize { .. } => CalibrationStageId::RmsAmplitudeNormalize,
        }
    }

    /// Whether the stage will execute.
    pub fn enabled(&self) -> bool {
        match self {
            Self::PhaseUnwrap { enabled }
            | Self::LinearPhaseDetrend { enabled }
            | Self::RmsAmplitudeNormalize { enabled, .. } => *enabled,
        }
    }
}

/// Versioned calibration profile with an explicit ordered stage list.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationProfile {
    /// Profile identity (for example `baseline-csi-v1`).
    pub id: String,
    /// Profile version.
    pub version: u32,
    /// Human-readable description.
    pub description: String,
    /// Ordered stage configurations. Order is never silently rewritten.
    pub stages: Vec<StageConfig>,
}

impl CalibrationProfile {
    /// Validates invariants and returns the profile unchanged on success.
    pub fn validate(&self) -> Result<(), CalibrationError> {
        if self.id.trim().is_empty() {
            return Err(CalibrationError::InvalidProfile {
                message: "profile id must not be empty".to_owned(),
            });
        }
        if self.version == 0 {
            return Err(CalibrationError::InvalidProfile {
                message: "profile version must be >= 1".to_owned(),
            });
        }

        let enabled: Vec<_> = self.stages.iter().filter(|stage| stage.enabled()).collect();
        if enabled.is_empty() {
            return Err(CalibrationError::InvalidProfile {
                message: "enabled profile must contain at least one enabled stage".to_owned(),
            });
        }

        let mut seen = Vec::new();
        for stage in &self.stages {
            let id = stage.stage_id();
            if seen.contains(&id) {
                return Err(CalibrationError::InvalidProfile {
                    message: format!("duplicate stage `{id}` is not allowed"),
                });
            }
            seen.push(id);

            if let StageConfig::RmsAmplitudeNormalize { epsilon, .. } = stage {
                RmsAmplitudeNormalizeStage::try_new(*epsilon)?;
            }
        }

        Ok(())
    }

    /// Ordered display names of enabled stages in execution order.
    pub fn enabled_stage_names(&self) -> Vec<&'static str> {
        self.stages
            .iter()
            .filter(|stage| stage.enabled())
            .map(|stage| stage.stage_id().as_str())
            .collect()
    }

    /// Builds executable stage instances for enabled entries, preserving order.
    pub(crate) fn build_stages(&self) -> Result<Vec<ConfiguredStage>, CalibrationError> {
        self.validate()?;
        let mut stages = Vec::new();
        for config in &self.stages {
            if !config.enabled() {
                continue;
            }
            stages.push(ConfiguredStage::from_config(config)?);
        }
        Ok(stages)
    }
}

/// Executable stage selected from typed configuration (no string dispatch).
#[derive(Debug, Clone)]
pub(crate) enum ConfiguredStage {
    PhaseUnwrap(PhaseUnwrapStage),
    LinearPhaseDetrend(LinearPhaseDetrendStage),
    RmsAmplitudeNormalize(RmsAmplitudeNormalizeStage),
}

impl ConfiguredStage {
    fn from_config(config: &StageConfig) -> Result<Self, CalibrationError> {
        Ok(match config {
            StageConfig::PhaseUnwrap { .. } => Self::PhaseUnwrap(PhaseUnwrapStage),
            StageConfig::LinearPhaseDetrend { .. } => {
                Self::LinearPhaseDetrend(LinearPhaseDetrendStage)
            }
            StageConfig::RmsAmplitudeNormalize { epsilon, .. } => {
                Self::RmsAmplitudeNormalize(RmsAmplitudeNormalizeStage::try_new(*epsilon)?)
            }
        })
    }

    pub(crate) fn as_stage(&self) -> &dyn crate::stage::CalibrationStage {
        match self {
            Self::PhaseUnwrap(stage) => stage,
            Self::LinearPhaseDetrend(stage) => stage,
            Self::RmsAmplitudeNormalize(stage) => stage,
        }
    }
}

/// Built-in baseline CSI v1 profile.
///
/// Default enabled order:
/// 1. `phase_unwrap`
/// 2. `linear_phase_detrend`
/// 3. `rms_amplitude_normalize`
pub fn baseline_csi_v1() -> CalibrationProfile {
    baseline_csi_v1_with(BaselineCsiV1Config::default())
}

/// Builds baseline-csi-v1 from optional stage overrides.
pub fn baseline_csi_v1_with(config: BaselineCsiV1Config) -> CalibrationProfile {
    CalibrationProfile {
        id: BASELINE_CSI_V1_ID.to_owned(),
        version: BASELINE_CSI_V1_VERSION,
        description: "Deterministic baseline CSI sanitization for development fixtures \
            (spatial unwrap, affine phase detrend, RMS normalize). Not hardware calibration."
            .to_owned(),
        stages: vec![
            StageConfig::PhaseUnwrap {
                enabled: config.phase_unwrap.enabled,
            },
            StageConfig::LinearPhaseDetrend {
                enabled: config.linear_phase_detrend.enabled,
            },
            StageConfig::RmsAmplitudeNormalize {
                enabled: config.rms_amplitude_normalize.enabled,
                epsilon: config.rms_amplitude_normalize.epsilon,
            },
        ],
    }
}

/// TOML configuration for the calibration subsystem.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CalibrationConfig {
    /// Whether calibration is enabled.
    #[serde(default = "default_calibration_enabled")]
    pub enabled: bool,
    /// Profile identity to activate.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Bounded frame queue capacity for the calibration worker.
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
    /// Stage overrides for `baseline-csi-v1`.
    #[serde(default)]
    pub baseline_csi_v1: BaselineCsiV1Config,
}

fn default_calibration_enabled() -> bool {
    true
}

fn default_profile() -> String {
    BASELINE_CSI_V1_ID.to_owned()
}

fn default_queue_capacity() -> usize {
    64
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            enabled: default_calibration_enabled(),
            profile: default_profile(),
            queue_capacity: default_queue_capacity(),
            baseline_csi_v1: BaselineCsiV1Config::default(),
        }
    }
}

/// Stage overrides for the built-in baseline profile.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct BaselineCsiV1Config {
    /// Phase unwrap stage configuration.
    #[serde(default)]
    pub phase_unwrap: StageEnabledConfig,
    /// Linear phase detrend stage configuration.
    #[serde(default)]
    pub linear_phase_detrend: StageEnabledConfig,
    /// RMS amplitude normalize stage configuration.
    #[serde(default)]
    pub rms_amplitude_normalize: RmsNormalizeConfig,
}

/// Simple enabled flag for stages without numeric parameters.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StageEnabledConfig {
    /// Whether the stage is enabled.
    #[serde(default = "default_stage_enabled")]
    pub enabled: bool,
}

fn default_stage_enabled() -> bool {
    true
}

impl Default for StageEnabledConfig {
    fn default() -> Self {
        Self {
            enabled: default_stage_enabled(),
        }
    }
}

/// RMS normalization stage parameters.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RmsNormalizeConfig {
    /// Whether the stage is enabled.
    #[serde(default = "default_stage_enabled")]
    pub enabled: bool,
    /// Positive finite epsilon for zero-energy rejection.
    #[serde(default = "default_rms_epsilon")]
    pub epsilon: f32,
}

fn default_rms_epsilon() -> f32 {
    DEFAULT_RMS_EPSILON
}

impl Default for RmsNormalizeConfig {
    fn default() -> Self {
        Self {
            enabled: default_stage_enabled(),
            epsilon: default_rms_epsilon(),
        }
    }
}

impl CalibrationConfig {
    /// Validates configuration and resolves the active profile when enabled.
    pub fn validate(&self) -> Result<(), CalibrationError> {
        if self.queue_capacity == 0 {
            return Err(CalibrationError::InvalidProfile {
                message: "calibration.queue_capacity must be greater than zero".to_owned(),
            });
        }
        if !self.enabled {
            return Ok(());
        }
        let profile = self.resolve_profile()?;
        profile.validate()
    }

    /// Resolves the configured profile identity into a typed profile.
    pub fn resolve_profile(&self) -> Result<CalibrationProfile, CalibrationError> {
        match self.profile.as_str() {
            BASELINE_CSI_V1_ID => {
                let profile = baseline_csi_v1_with(self.baseline_csi_v1.clone());
                profile.validate()?;
                Ok(profile)
            }
            other => Err(CalibrationError::InvalidProfile {
                message: format!(
                    "unsupported calibration profile `{other}` (supported: {BASELINE_CSI_V1_ID})"
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_profile_default_order() {
        let profile = baseline_csi_v1();
        profile.validate().expect("valid");
        assert_eq!(
            profile.enabled_stage_names(),
            vec![
                "phase_unwrap",
                "linear_phase_detrend",
                "rms_amplitude_normalize"
            ]
        );
    }

    #[test]
    fn empty_enabled_profile_rejected() {
        let mut profile = baseline_csi_v1();
        for stage in &mut profile.stages {
            match stage {
                StageConfig::PhaseUnwrap { enabled }
                | StageConfig::LinearPhaseDetrend { enabled }
                | StageConfig::RmsAmplitudeNormalize { enabled, .. } => *enabled = false,
            }
        }
        assert!(matches!(
            profile.validate(),
            Err(CalibrationError::InvalidProfile { .. })
        ));
    }

    #[test]
    fn duplicate_stage_rejected() {
        let profile = CalibrationProfile {
            id: "custom".into(),
            version: 1,
            description: "dup".into(),
            stages: vec![
                StageConfig::PhaseUnwrap { enabled: true },
                StageConfig::PhaseUnwrap { enabled: true },
            ],
        };
        assert!(matches!(
            profile.validate(),
            Err(CalibrationError::InvalidProfile { .. })
        ));
    }

    #[test]
    fn invalid_epsilon_rejected() {
        let mut config = CalibrationConfig::default();
        config.baseline_csi_v1.rms_amplitude_normalize.epsilon = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn unsupported_profile_rejected() {
        let config = CalibrationConfig {
            profile: "unknown".into(),
            ..CalibrationConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn zero_queue_capacity_rejected() {
        let config = CalibrationConfig {
            queue_capacity: 0,
            ..CalibrationConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn disabled_config_skips_profile_validation() {
        let config = CalibrationConfig {
            enabled: false,
            profile: "unknown".into(),
            ..CalibrationConfig::default()
        };
        config.validate().expect("disabled ok");
    }
}
