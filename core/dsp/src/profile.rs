//! Versioned baseline DSP profile and TOML-facing configuration.

use serde::Deserialize;

use crate::assembler::AssemblerConfig;
use crate::errors::DspError;

/// Built-in baseline DSP profile identity.
pub const BASELINE_DSP_V1_ID: &str = "baseline-dsp-v1";

/// Built-in baseline DSP profile version.
pub const BASELINE_DSP_V1_VERSION: u32 = 1;

/// Spectral normalization method recorded in profile provenance.
pub const SPECTRAL_NORMALIZATION: &str = "onesided_periodogram_hann_window_power";

/// FFT implementation recorded in profile provenance.
pub const FFT_IMPLEMENTATION: &str = "rustfft";

/// Versioned DSP profile provenance (not a plugin framework).
#[derive(Debug, Clone, PartialEq)]
pub struct DspProfile {
    /// Profile identity (for example `baseline-dsp-v1`).
    pub id: String,
    /// Profile version.
    pub version: u32,
    /// Temporal window size in frames.
    pub window_size_frames: usize,
    /// Hop size in frames.
    pub hop_size_frames: usize,
    /// Whether temporal mean removal is applied before the FFT.
    pub mean_removal_enabled: bool,
    /// Whether a Hann window is applied before the FFT.
    pub hann_window_enabled: bool,
    /// FFT implementation label.
    pub fft_implementation: String,
    /// Spectral normalization method label.
    pub spectral_normalization: String,
    /// Timestamp jitter tolerance used for spectral gating.
    pub timestamp_jitter_tolerance: f64,
}

impl DspProfile {
    /// Validates profile provenance fields.
    pub fn validate(&self) -> Result<(), DspError> {
        if self.id.trim().is_empty() {
            return Err(DspError::InvalidConfig {
                message: "DSP profile id must not be empty".to_owned(),
            });
        }
        if self.version == 0 {
            return Err(DspError::InvalidConfig {
                message: "DSP profile version must be >= 1".to_owned(),
            });
        }
        AssemblerConfig {
            window_size_frames: self.window_size_frames,
            hop_size_frames: self.hop_size_frames,
            queue_capacity: self.window_size_frames,
            maximum_sequence_gap: 1,
            timestamp_jitter_tolerance: self.timestamp_jitter_tolerance,
        }
        .validate()?;
        Ok(())
    }
}

/// Returns the built-in baseline DSP profile using assembler dimensions from config.
pub fn baseline_dsp_v1(
    window_size_frames: usize,
    hop_size_frames: usize,
    timestamp_jitter_tolerance: f64,
) -> DspProfile {
    DspProfile {
        id: BASELINE_DSP_V1_ID.to_owned(),
        version: BASELINE_DSP_V1_VERSION,
        window_size_frames,
        hop_size_frames,
        mean_removal_enabled: true,
        hann_window_enabled: true,
        fft_implementation: FFT_IMPLEMENTATION.to_owned(),
        spectral_normalization: SPECTRAL_NORMALIZATION.to_owned(),
        timestamp_jitter_tolerance,
    }
}

/// TOML-facing DSP configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DspConfig {
    /// Whether the DSP worker is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Selected profile identity.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Bounded calibrated-frame input queue capacity.
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
    /// Temporal window size in frames.
    #[serde(default = "default_window_size")]
    pub window_size_frames: usize,
    /// Hop size in frames.
    #[serde(default = "default_hop_size")]
    pub hop_size_frames: usize,
    /// Maximum allowed missing-frame gap between sequences.
    #[serde(default = "default_maximum_sequence_gap")]
    pub maximum_sequence_gap: u64,
    /// Timestamp jitter tolerance for spectral analysis.
    #[serde(default = "default_timestamp_jitter_tolerance")]
    pub timestamp_jitter_tolerance: f64,
}

fn default_profile() -> String {
    BASELINE_DSP_V1_ID.to_owned()
}

fn default_queue_capacity() -> usize {
    64
}

fn default_window_size() -> usize {
    16
}

fn default_hop_size() -> usize {
    4
}

fn default_maximum_sequence_gap() -> u64 {
    1
}

fn default_timestamp_jitter_tolerance() -> f64 {
    0.10
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: default_profile(),
            queue_capacity: default_queue_capacity(),
            window_size_frames: default_window_size(),
            hop_size_frames: default_hop_size(),
            maximum_sequence_gap: default_maximum_sequence_gap(),
            timestamp_jitter_tolerance: default_timestamp_jitter_tolerance(),
        }
    }
}

impl DspConfig {
    /// Validates configuration values.
    pub fn validate(&self) -> Result<(), DspError> {
        if !self.enabled {
            return Ok(());
        }
        if self.profile != BASELINE_DSP_V1_ID {
            return Err(DspError::InvalidConfig {
                message: format!(
                    "unsupported DSP profile `{}`; only `{BASELINE_DSP_V1_ID}` is available",
                    self.profile
                ),
            });
        }
        if self.queue_capacity == 0 {
            return Err(DspError::InvalidConfig {
                message: "dsp.queue_capacity must be greater than zero".to_owned(),
            });
        }
        let assembler = self.assembler_config();
        assembler.validate()?;
        let profile = self.resolve_profile()?;
        profile.validate()?;
        Ok(())
    }

    /// Builds assembler configuration from this section.
    pub fn assembler_config(&self) -> AssemblerConfig {
        AssemblerConfig {
            window_size_frames: self.window_size_frames,
            hop_size_frames: self.hop_size_frames,
            queue_capacity: self.queue_capacity.max(self.window_size_frames),
            maximum_sequence_gap: self.maximum_sequence_gap,
            timestamp_jitter_tolerance: self.timestamp_jitter_tolerance,
        }
    }

    /// Resolves the selected versioned profile.
    pub fn resolve_profile(&self) -> Result<DspProfile, DspError> {
        if self.profile != BASELINE_DSP_V1_ID {
            return Err(DspError::InvalidConfig {
                message: format!("unknown DSP profile `{}`", self.profile),
            });
        }
        let profile = baseline_dsp_v1(
            self.window_size_frames,
            self.hop_size_frames,
            self.timestamp_jitter_tolerance,
        );
        profile.validate()?;
        Ok(profile)
    }
}
