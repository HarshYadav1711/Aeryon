//! Configurable CSI calibration pipeline for Aeryon.
//!
//! Applies ordered, deterministic sanitization stages to canonical
//! [`aeryon_csi::CsiFrame`] values and produces immutable
//! [`CalibratedCsiFrame`] outputs with structured provenance.
//!
//! # Honesty
//!
//! - Calibration is deterministic for a given frame and profile.
//! - Phase unwrapping is **spatial** across ordered subcarriers, not temporal.
//! - Linear phase detrending is a baseline affine sanitization step.
//! - RMS amplitude normalization operates independently per antenna link.
//! - This is **not** full hardware-specific calibration and does not make
//!   development fixtures equivalent to calibrated RF captures.
//! - No perception inference is performed.

#![deny(missing_docs)]

pub mod errors;
pub mod frame;
pub mod pipeline;
pub mod profile;
pub mod report;
pub mod stage;
pub mod stages;

pub use errors::{AntennaLink, CalibrationError};
pub use frame::CalibratedCsiFrame;
pub use pipeline::CalibrationPipeline;
pub use profile::{
    BASELINE_CSI_V1_ID, BASELINE_CSI_V1_VERSION, BaselineCsiV1Config, CalibrationConfig,
    CalibrationProfile, RmsNormalizeConfig, StageConfig, StageEnabledConfig, baseline_csi_v1,
    baseline_csi_v1_with,
};
pub use report::{
    CalibrationReport, CalibrationStatus, CalibrationWarning, StageDiagnostics, StageReport,
};
pub use stage::CalibrationStageId;
pub use stages::{
    DEFAULT_RMS_EPSILON, LinearPhaseDetrendStage, PhaseUnwrapStage, RmsAmplitudeNormalizeStage,
};

/// Subsystem identifier.
pub const ID: &str = "calibration";

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
