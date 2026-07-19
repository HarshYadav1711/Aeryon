//! Structured calibration reports and typed warnings.

use aeryon_domain::{FrameId, Timestamp};

use crate::stage::CalibrationStageId;

/// Aggregate calibration pipeline status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalibrationStatus {
    /// All enabled stages completed and output validation succeeded.
    Success,
    /// Calibration failed before producing a publishable frame.
    Failed,
}

impl CalibrationStatus {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

/// Structured warning produced during calibration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationWarning {
    /// A stage was configured but disabled and therefore skipped.
    StageSkipped {
        /// Skipped stage.
        stage: CalibrationStageId,
        /// Configured order index among profile stages.
        order: u16,
    },
    /// Single-subcarrier link left unchanged by linear phase detrend.
    SingleSubcarrierLinkUnchanged {
        /// Stage that emitted the warning.
        stage: CalibrationStageId,
        /// Receive antenna index.
        rx: u16,
        /// Transmit antenna index.
        tx: u16,
    },
}

impl CalibrationWarning {
    /// Concise operator-facing summary.
    pub fn summary(&self) -> String {
        match self {
            Self::StageSkipped { stage, order } => {
                format!("stage {stage} at order {order} skipped (disabled)")
            }
            Self::SingleSubcarrierLinkUnchanged { stage, rx, tx } => {
                format!("{stage}: single-subcarrier link rx={rx} tx={tx} left unchanged")
            }
        }
    }
}

/// Per-stage execution report.
#[derive(Debug, Clone, PartialEq)]
pub struct StageReport {
    /// Stage identity.
    pub stage_id: CalibrationStageId,
    /// Stage display name.
    pub stage_name: String,
    /// Zero-based execution order among executed stages.
    pub order: u16,
    /// Stage duration in nanoseconds.
    pub duration_ns: u64,
    /// Whether the stage completed successfully.
    pub success: bool,
    /// Number of structured warnings emitted by the stage.
    pub warning_count: u32,
    /// Structured warnings emitted by the stage.
    pub warnings: Vec<CalibrationWarning>,
    /// Concise diagnostics (no sample arrays).
    pub diagnostics: StageDiagnostics,
}

/// Concise per-stage diagnostics.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StageDiagnostics {
    /// No additional diagnostics.
    #[default]
    None,
    /// Phase-unwrap aggregate facts.
    PhaseUnwrap {
        /// Number of antenna links processed.
        links_processed: u32,
        /// Total adjacent subcarrier wraps applied.
        wraps_applied: u64,
    },
    /// Linear phase detrend aggregate / summary facts.
    LinearPhaseDetrend {
        /// Number of antenna links processed.
        links_processed: u32,
        /// Links left unchanged (single subcarrier policy).
        links_unchanged: u32,
        /// Mean absolute fitted slope across processed links.
        mean_abs_slope: f64,
        /// Mean absolute fitted intercept across processed links.
        mean_abs_intercept: f64,
    },
    /// RMS amplitude normalization diagnostics.
    RmsAmplitudeNormalize {
        /// Number of antenna links processed.
        links_processed: u32,
        /// Original per-link RMS magnitudes (before normalization).
        original_rms: Vec<f32>,
    },
}

/// Frame-level calibration report preserving executed stage order.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationReport {
    /// Raw frame identifier.
    pub raw_frame_id: FrameId,
    /// Sequence number.
    pub sequence: u64,
    /// Profile identity.
    pub profile_id: String,
    /// Profile version.
    pub profile_version: u32,
    /// Pipeline start timestamp.
    pub started_at: Timestamp,
    /// Pipeline completion timestamp.
    pub completed_at: Timestamp,
    /// Total duration in nanoseconds.
    pub duration_ns: u64,
    /// Ordered stage reports matching execution order.
    pub stages: Vec<StageReport>,
    /// Input sample count.
    pub input_sample_count: usize,
    /// Output sample count.
    pub output_sample_count: usize,
    /// Structured warnings.
    pub warnings: Vec<CalibrationWarning>,
    /// Aggregate status.
    pub status: CalibrationStatus,
}

impl CalibrationReport {
    /// Concise warning summary for API surfaces.
    pub fn warning_summary(&self) -> Option<String> {
        if self.warnings.is_empty() {
            return None;
        }
        Some(
            self.warnings
                .iter()
                .map(CalibrationWarning::summary)
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}
