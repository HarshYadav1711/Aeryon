//! Configuration for the deterministic synthetic sensor.
//!
//! This sensor is integration-test infrastructure, not a real perception source.

use core::fmt;

use serde::Deserialize;

/// Configuration for the synthetic sensor plugin.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SyntheticSensorConfig {
    /// Whether the synthetic sensor should be registered and started.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Interval between emitted frames in milliseconds.
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
    /// Number of samples in each frame.
    #[serde(default = "default_samples_per_frame")]
    pub samples_per_frame: usize,
    /// Sample rate in hertz used by the deterministic signal model.
    #[serde(default = "default_sample_rate_hz")]
    pub sample_rate_hz: f64,
    /// Primary sine frequency in hertz.
    #[serde(default = "default_primary_frequency_hz")]
    pub primary_frequency_hz: f64,
    /// Secondary sine frequency in hertz.
    #[serde(default = "default_secondary_frequency_hz")]
    pub secondary_frequency_hz: f64,
    /// Amplitude of the secondary sine component.
    #[serde(default = "default_secondary_amplitude")]
    pub secondary_amplitude: f64,
    /// Optional maximum number of frames to emit before stopping the producer.
    #[serde(default)]
    pub maximum_frames: Option<u64>,
    /// Emit an info summary every N frames (minimum 1).
    #[serde(default = "default_log_every_n_frames")]
    pub log_every_n_frames: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_interval_ms() -> u64 {
    100
}

fn default_samples_per_frame() -> usize {
    64
}

fn default_sample_rate_hz() -> f64 {
    1_000.0
}

fn default_primary_frequency_hz() -> f64 {
    10.0
}

fn default_secondary_frequency_hz() -> f64 {
    37.0
}

fn default_secondary_amplitude() -> f64 {
    0.25
}

fn default_log_every_n_frames() -> u64 {
    10
}

impl Default for SyntheticSensorConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            interval_ms: default_interval_ms(),
            samples_per_frame: default_samples_per_frame(),
            sample_rate_hz: default_sample_rate_hz(),
            primary_frequency_hz: default_primary_frequency_hz(),
            secondary_frequency_hz: default_secondary_frequency_hz(),
            secondary_amplitude: default_secondary_amplitude(),
            maximum_frames: None,
            log_every_n_frames: default_log_every_n_frames(),
        }
    }
}

/// Typed configuration validation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntheticConfigError {
    /// Frame interval must be greater than zero.
    ZeroInterval,
    /// Samples per frame must be greater than zero.
    ZeroSamplesPerFrame,
    /// Sample rate must be greater than zero and finite.
    InvalidSampleRate,
    /// A frequency must be finite and non-negative.
    InvalidFrequency(&'static str),
    /// Amplitude must be finite.
    InvalidAmplitude,
    /// A frequency exceeds the Nyquist limit for the configured sample rate.
    FrequencyAboveNyquist(&'static str),
    /// Log interval must be at least one.
    ZeroLogEveryNFrames,
}

impl fmt::Display for SyntheticConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInterval => f.write_str("synthetic_sensor.interval_ms must be > 0"),
            Self::ZeroSamplesPerFrame => {
                f.write_str("synthetic_sensor.samples_per_frame must be > 0")
            }
            Self::InvalidSampleRate => {
                f.write_str("synthetic_sensor.sample_rate_hz must be finite and > 0")
            }
            Self::InvalidFrequency(name) => {
                write!(f, "synthetic_sensor.{name} must be finite and >= 0")
            }
            Self::InvalidAmplitude => {
                f.write_str("synthetic_sensor.secondary_amplitude must be finite")
            }
            Self::FrequencyAboveNyquist(name) => {
                write!(
                    f,
                    "synthetic_sensor.{name} exceeds Nyquist limit for sample_rate_hz"
                )
            }
            Self::ZeroLogEveryNFrames => {
                f.write_str("synthetic_sensor.log_every_n_frames must be > 0")
            }
        }
    }
}

impl std::error::Error for SyntheticConfigError {}

impl SyntheticSensorConfig {
    /// Validates configuration values.
    pub fn validate(&self) -> Result<(), SyntheticConfigError> {
        if self.interval_ms == 0 {
            return Err(SyntheticConfigError::ZeroInterval);
        }
        if self.samples_per_frame == 0 {
            return Err(SyntheticConfigError::ZeroSamplesPerFrame);
        }
        if !self.sample_rate_hz.is_finite() || self.sample_rate_hz <= 0.0 {
            return Err(SyntheticConfigError::InvalidSampleRate);
        }
        validate_frequency("primary_frequency_hz", self.primary_frequency_hz)?;
        validate_frequency("secondary_frequency_hz", self.secondary_frequency_hz)?;
        if !self.secondary_amplitude.is_finite() {
            return Err(SyntheticConfigError::InvalidAmplitude);
        }
        let nyquist = self.sample_rate_hz / 2.0;
        if self.primary_frequency_hz > nyquist {
            return Err(SyntheticConfigError::FrequencyAboveNyquist(
                "primary_frequency_hz",
            ));
        }
        if self.secondary_frequency_hz > nyquist {
            return Err(SyntheticConfigError::FrequencyAboveNyquist(
                "secondary_frequency_hz",
            ));
        }
        if self.log_every_n_frames == 0 {
            return Err(SyntheticConfigError::ZeroLogEveryNFrames);
        }
        Ok(())
    }
}

fn validate_frequency(name: &'static str, value: f64) -> Result<(), SyntheticConfigError> {
    if !value.is_finite() || value < 0.0 {
        Err(SyntheticConfigError::InvalidFrequency(name))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        SyntheticSensorConfig::default()
            .validate()
            .expect("defaults valid");
    }

    #[test]
    fn zero_interval_is_rejected() {
        let config = SyntheticSensorConfig {
            interval_ms: 0,
            ..SyntheticSensorConfig::default()
        };
        assert_eq!(
            config.validate().expect_err("invalid"),
            SyntheticConfigError::ZeroInterval
        );
    }

    #[test]
    fn zero_samples_are_rejected() {
        let config = SyntheticSensorConfig {
            samples_per_frame: 0,
            ..SyntheticSensorConfig::default()
        };
        assert_eq!(
            config.validate().expect_err("invalid"),
            SyntheticConfigError::ZeroSamplesPerFrame
        );
    }

    #[test]
    fn non_finite_amplitude_is_rejected() {
        let config = SyntheticSensorConfig {
            secondary_amplitude: f64::NAN,
            ..SyntheticSensorConfig::default()
        };
        assert_eq!(
            config.validate().expect_err("invalid"),
            SyntheticConfigError::InvalidAmplitude
        );
    }

    #[test]
    fn frequency_above_nyquist_is_rejected() {
        let config = SyntheticSensorConfig {
            sample_rate_hz: 100.0,
            primary_frequency_hz: 80.0,
            ..SyntheticSensorConfig::default()
        };
        assert_eq!(
            config.validate().expect_err("invalid"),
            SyntheticConfigError::FrequencyAboveNyquist("primary_frequency_hz")
        );
    }
}
