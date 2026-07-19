//! Built-in calibration stages.

pub mod linear_phase_detrend;
pub mod phase_unwrap;
pub mod rms_amplitude_normalize;

pub use linear_phase_detrend::LinearPhaseDetrendStage;
pub use phase_unwrap::PhaseUnwrapStage;
pub use rms_amplitude_normalize::{DEFAULT_RMS_EPSILON, RmsAmplitudeNormalizeStage};
