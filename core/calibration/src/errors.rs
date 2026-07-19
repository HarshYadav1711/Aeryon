//! Typed calibration errors.

use core::fmt;

use aeryon_domain::{FrameId, SensorId};

use crate::stage::CalibrationStageId;

/// Errors produced by calibration profile construction, stages, or output validation.
#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationError {
    /// Profile failed validation or could not be constructed.
    InvalidProfile {
        /// Human-readable summary.
        message: String,
    },
    /// An unsupported stage identity or version was requested.
    UnsupportedStage {
        /// Unsupported stage label.
        stage: String,
    },
    /// Raw frame dimensions or sample layout are malformed for calibration.
    MalformedFrame {
        /// Frame identifier when available.
        frame_id: Option<FrameId>,
        /// Sequence when available.
        sequence: Option<u64>,
        /// Detail message.
        message: String,
    },
    /// A non-finite complex sample was encountered.
    NonFiniteSample {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Optional failing stage.
        stage: Option<CalibrationStageId>,
        /// Flat sample index when known.
        sample_index: Option<usize>,
        /// Antenna link when known.
        link: Option<AntennaLink>,
    },
    /// Not enough subcarrier information for a stage.
    InsufficientSubcarriers {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Failing stage.
        stage: CalibrationStageId,
        /// Antenna link when known.
        link: Option<AntennaLink>,
        /// Observed subcarrier count.
        count: usize,
    },
    /// Linear regression is degenerate (for example zero variance in `x`).
    DegenerateRegression {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Failing stage.
        stage: CalibrationStageId,
        /// Antenna link.
        link: AntennaLink,
        /// Detail message.
        message: String,
    },
    /// An antenna link has near-zero energy under the configured epsilon policy.
    ZeroEnergyLink {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Failing stage.
        stage: CalibrationStageId,
        /// Antenna link.
        link: AntennaLink,
        /// Measured RMS magnitude.
        rms: f32,
        /// Configured epsilon.
        epsilon: f32,
    },
    /// A stage reported failure with context.
    StageFailure {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Failing stage.
        stage: CalibrationStageId,
        /// Detail message.
        message: String,
    },
    /// Final calibrated output failed validation.
    OutputValidation {
        /// Frame identifier.
        frame_id: FrameId,
        /// Sequence number.
        sequence: u64,
        /// Detail message.
        message: String,
    },
    /// Calibration pipeline is unavailable or disabled.
    PipelineUnavailable {
        /// Detail message.
        message: String,
    },
}

/// Receive/transmit antenna link coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AntennaLink {
    /// Receive antenna index.
    pub rx: u16,
    /// Transmit antenna index.
    pub tx: u16,
}

impl AntennaLink {
    /// Creates an antenna link coordinate.
    pub const fn new(rx: u16, tx: u16) -> Self {
        Self { rx, tx }
    }
}

impl fmt::Display for AntennaLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rx={} tx={}", self.rx, self.tx)
    }
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile { message } => write!(f, "invalid calibration profile: {message}"),
            Self::UnsupportedStage { stage } => {
                write!(f, "unsupported calibration stage `{stage}`")
            }
            Self::MalformedFrame {
                frame_id,
                sequence,
                message,
            } => write!(
                f,
                "malformed CSI frame for calibration (frame_id={frame_id:?}, sequence={sequence:?}): {message}"
            ),
            Self::NonFiniteSample {
                frame_id,
                sequence,
                stage,
                sample_index,
                link,
            } => write!(
                f,
                "non-finite CSI sample during calibration (frame={}, seq={}, stage={stage:?}, index={sample_index:?}, link={link:?})",
                frame_id.value(),
                sequence
            ),
            Self::InsufficientSubcarriers {
                frame_id,
                sequence,
                stage,
                link,
                count,
            } => write!(
                f,
                "insufficient subcarriers for {stage} (frame={}, seq={}, link={link:?}, count={count})",
                frame_id.value(),
                sequence
            ),
            Self::DegenerateRegression {
                frame_id,
                sequence,
                stage,
                link,
                message,
            } => write!(
                f,
                "degenerate regression in {stage} (frame={}, seq={}, {link}): {message}",
                frame_id.value(),
                sequence
            ),
            Self::ZeroEnergyLink {
                frame_id,
                sequence,
                stage,
                link,
                rms,
                epsilon,
            } => write!(
                f,
                "zero-energy link in {stage} (frame={}, seq={}, {link}): rms={rms} epsilon={epsilon}",
                frame_id.value(),
                sequence
            ),
            Self::StageFailure {
                frame_id,
                sequence,
                stage,
                message,
            } => write!(
                f,
                "calibration stage {stage} failed (frame={}, seq={}): {message}",
                frame_id.value(),
                sequence
            ),
            Self::OutputValidation {
                frame_id,
                sequence,
                message,
            } => write!(
                f,
                "calibrated output validation failed (frame={}, seq={}): {message}",
                frame_id.value(),
                sequence
            ),
            Self::PipelineUnavailable { message } => {
                write!(f, "calibration pipeline unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for CalibrationError {}

impl CalibrationError {
    /// Stable machine-readable failure code for API / WebSocket surfaces.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidProfile { .. } => "invalid_profile",
            Self::UnsupportedStage { .. } => "unsupported_stage",
            Self::MalformedFrame { .. } => "malformed_frame",
            Self::NonFiniteSample { .. } => "non_finite_sample",
            Self::InsufficientSubcarriers { .. } => "insufficient_subcarriers",
            Self::DegenerateRegression { .. } => "degenerate_regression",
            Self::ZeroEnergyLink { .. } => "zero_energy_link",
            Self::StageFailure { .. } => "stage_failure",
            Self::OutputValidation { .. } => "output_validation",
            Self::PipelineUnavailable { .. } => "pipeline_unavailable",
        }
    }

    /// Optional failing stage identity.
    pub fn stage(&self) -> Option<CalibrationStageId> {
        match self {
            Self::NonFiniteSample { stage, .. } => *stage,
            Self::InsufficientSubcarriers { stage, .. }
            | Self::DegenerateRegression { stage, .. }
            | Self::ZeroEnergyLink { stage, .. }
            | Self::StageFailure { stage, .. } => Some(*stage),
            _ => None,
        }
    }

    /// Optional frame identifier.
    pub fn frame_id(&self) -> Option<FrameId> {
        match self {
            Self::MalformedFrame { frame_id, .. } => *frame_id,
            Self::NonFiniteSample { frame_id, .. }
            | Self::InsufficientSubcarriers { frame_id, .. }
            | Self::DegenerateRegression { frame_id, .. }
            | Self::ZeroEnergyLink { frame_id, .. }
            | Self::StageFailure { frame_id, .. }
            | Self::OutputValidation { frame_id, .. } => Some(*frame_id),
            _ => None,
        }
    }

    /// Optional sequence number.
    pub fn sequence(&self) -> Option<u64> {
        match self {
            Self::MalformedFrame { sequence, .. } => *sequence,
            Self::NonFiniteSample { sequence, .. }
            | Self::InsufficientSubcarriers { sequence, .. }
            | Self::DegenerateRegression { sequence, .. }
            | Self::ZeroEnergyLink { sequence, .. }
            | Self::StageFailure { sequence, .. }
            | Self::OutputValidation { sequence, .. } => Some(*sequence),
            _ => None,
        }
    }

    /// Optional sensor identifier (not always available on construction errors).
    pub fn sensor_id(&self) -> Option<SensorId> {
        None
    }
}
