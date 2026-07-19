//! Temporal CSI windowing and baseline spectral DSP for Aeryon.
//!
//! # Honesty
//!
//! - Motion energy is a channel-change proxy, not human-motion classification.
//! - Spectral peaks are not interpreted as activities (walking, breathing, etc.).
//! - Frequencies use capture timestamps from the CSI fixture timeline, not replay
//!   wall-clock speed or browser arrival time.
//! - Pure Rust (`rustfft`) is the current baseline; optimization follows profiling.

#![deny(missing_docs)]

pub mod assembler;
pub mod errors;
pub mod motion;
pub mod profile;
pub mod report;
pub mod result;
pub mod service;
pub mod spectral;
pub mod stats;
pub mod window;

pub use assembler::{AssemblerConfig, AssemblerCounters, WindowAssembler};
pub use errors::{DspError, DspFailureCode as DspErrorCode};
pub use motion::{LinkMotionEnergy, MotionEnergySignal, compute_motion_energy};
pub use profile::{
    BASELINE_DSP_V1_ID, BASELINE_DSP_V1_VERSION, DspConfig, DspProfile, FFT_IMPLEMENTATION,
    SPECTRAL_NORMALIZATION, baseline_dsp_v1,
};
pub use report::process_window;
pub use result::{DspResultStatus, DspWindowResult, MotionEnergySeries};
pub use service::{CalibratedFrameRx, CalibratedFrameTx, DspResultSink, DspService};
pub use spectral::{
    LinkPowerSpectrum, SamplingAnalysis, SpectralAnalysis, analyze_sampling, analyze_spectrum,
    hann_window,
};
pub use stats::{DspStats, DspWorkerState};
pub use window::{CsiWindow, SAMPLE_LAYOUT};

/// Subsystem identifier.
pub const ID: &str = "dsp";

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
