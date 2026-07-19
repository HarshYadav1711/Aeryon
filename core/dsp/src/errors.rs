//! Typed DSP validation and processing errors.

use aeryon_domain::{FrameId, SensorId};
use thiserror::Error;

/// Errors produced while assembling or processing CSI windows.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DspError {
    /// DSP profile or runtime configuration failed validation.
    #[error("invalid DSP configuration: {message}")]
    InvalidConfig {
        /// Operator-safe detail.
        message: String,
    },
    /// Window failed temporal or geometry validation.
    #[error("invalid CSI window: {message}")]
    InvalidWindow {
        /// Operator-safe detail.
        message: String,
    },
    /// Frame rejected by the window assembler.
    #[error("window assembler rejected frame: {message}")]
    AssemblerRejected {
        /// Optional frame identity.
        frame_id: Option<FrameId>,
        /// Optional sensor identity.
        sensor_id: Option<SensorId>,
        /// Optional sequence.
        sequence: Option<u64>,
        /// Operator-safe detail.
        message: String,
        /// Stable machine-readable code.
        code: DspFailureCode,
    },
    /// Motion-energy computation failed.
    #[error("motion-energy calculation failed: {message}")]
    MotionEnergy {
        /// Operator-safe detail.
        message: String,
    },
    /// Spectral analysis failed or was rejected.
    #[error("spectral analysis failed: {message}")]
    Spectral {
        /// Operator-safe detail.
        message: String,
        /// Stable machine-readable code.
        code: DspFailureCode,
    },
    /// Output validation failed (for example non-finite values).
    #[error("DSP output validation failed: {message}")]
    OutputValidation {
        /// Operator-safe detail.
        message: String,
    },
}

impl DspError {
    /// Stable wire / statistics code.
    pub fn code(&self) -> DspFailureCode {
        match self {
            Self::InvalidConfig { .. } => DspFailureCode::InvalidConfig,
            Self::InvalidWindow { .. } => DspFailureCode::InvalidWindow,
            Self::AssemblerRejected { code, .. } => *code,
            Self::MotionEnergy { .. } => DspFailureCode::MotionEnergy,
            Self::Spectral { code, .. } => *code,
            Self::OutputValidation { .. } => DspFailureCode::OutputValidation,
        }
    }

    /// Optional sequence for failure events.
    pub fn sequence(&self) -> Option<u64> {
        match self {
            Self::AssemblerRejected { sequence, .. } => *sequence,
            _ => None,
        }
    }

    /// Optional sensor for failure events.
    pub fn sensor_id(&self) -> Option<SensorId> {
        match self {
            Self::AssemblerRejected { sensor_id, .. } => *sensor_id,
            _ => None,
        }
    }
}

/// Machine-readable DSP failure codes for API surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DspFailureCode {
    /// Profile / configuration validation failed.
    InvalidConfig,
    /// Window geometry or temporal invariants failed.
    InvalidWindow,
    /// Sensor mismatch across frames.
    SensorMismatch,
    /// Antenna or subcarrier geometry mismatch.
    GeometryMismatch,
    /// Calibration profile identity or version mismatch.
    CalibrationProfileMismatch,
    /// Sequence numbers are not strictly increasing.
    NonMonotonicSequence,
    /// Sequence gap exceeds configured tolerance.
    SequenceGap,
    /// Capture timestamps are not monotonic.
    NonMonotonicTimestamp,
    /// Timestamp jitter exceeds spectral tolerance.
    ExcessiveJitter,
    /// Motion-energy proxy computation failed.
    MotionEnergy,
    /// Spectral analysis rejected the input.
    Spectral,
    /// Insufficient samples for spectral analysis.
    InsufficientLength,
    /// Effective sample rate is invalid.
    InvalidSampleRate,
    /// Non-finite intermediate or output values.
    NonFinite,
    /// Output validation failed.
    OutputValidation,
    /// DSP worker exited unexpectedly.
    WorkerExited,
}

impl DspFailureCode {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "invalid_config",
            Self::InvalidWindow => "invalid_window",
            Self::SensorMismatch => "sensor_mismatch",
            Self::GeometryMismatch => "geometry_mismatch",
            Self::CalibrationProfileMismatch => "calibration_profile_mismatch",
            Self::NonMonotonicSequence => "non_monotonic_sequence",
            Self::SequenceGap => "sequence_gap",
            Self::NonMonotonicTimestamp => "non_monotonic_timestamp",
            Self::ExcessiveJitter => "excessive_jitter",
            Self::MotionEnergy => "motion_energy",
            Self::Spectral => "spectral",
            Self::InsufficientLength => "insufficient_length",
            Self::InvalidSampleRate => "invalid_sample_rate",
            Self::NonFinite => "non_finite",
            Self::OutputValidation => "output_validation",
            Self::WorkerExited => "worker_exited",
        }
    }
}
