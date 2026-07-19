//! Configuration for the CSI fixture replay plugin.

use core::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Configuration for CSI fixture replay.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CsiReplayConfig {
    /// Whether the CSI replay plugin should be registered and started.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Repository-relative or absolute path to an Aeryon CSI Fixture Format file.
    #[serde(default = "default_path")]
    pub path: PathBuf,
    /// When true, restart from the beginning after a finite fixture completes.
    #[serde(default)]
    pub loop_playback: bool,
    /// Interval between emitted frames in milliseconds.
    #[serde(default = "default_frame_interval_ms")]
    pub frame_interval_ms: u64,
    /// Maximum frames to emit (`0` means no additional limit beyond the fixture).
    #[serde(default)]
    pub maximum_frames: u64,
}

fn default_enabled() -> bool {
    false
}

fn default_path() -> PathBuf {
    PathBuf::from("datasets/fixtures/csi/synthetic_dev_v1.ndjson")
}

fn default_frame_interval_ms() -> u64 {
    100
}

impl Default for CsiReplayConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            path: default_path(),
            loop_playback: false,
            frame_interval_ms: default_frame_interval_ms(),
            maximum_frames: 0,
        }
    }
}

/// Typed CSI replay configuration errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiReplayConfigError {
    /// Frame interval must be greater than zero.
    ZeroInterval,
    /// Fixture path must be non-empty when enabled.
    EmptyPath,
    /// Enabled replay requires an existing readable fixture file.
    FixtureMissing(PathBuf),
    /// Fixture path exists but is not a regular file.
    FixtureNotFile(PathBuf),
}

impl fmt::Display for CsiReplayConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInterval => f.write_str("sensors.csi_replay.frame_interval_ms must be > 0"),
            Self::EmptyPath => {
                f.write_str("sensors.csi_replay.path must not be empty when enabled")
            }
            Self::FixtureMissing(path) => {
                write!(
                    f,
                    "sensors.csi_replay.path does not exist: {}",
                    path.display()
                )
            }
            Self::FixtureNotFile(path) => {
                write!(
                    f,
                    "sensors.csi_replay.path is not a file: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for CsiReplayConfigError {}

impl CsiReplayConfig {
    /// Validates configuration values.
    ///
    /// When `enabled`, the fixture path must exist and be a readable file.
    pub fn validate(&self) -> Result<(), CsiReplayConfigError> {
        if self.frame_interval_ms == 0 {
            return Err(CsiReplayConfigError::ZeroInterval);
        }
        if !self.enabled {
            return Ok(());
        }
        if self.path.as_os_str().is_empty() {
            return Err(CsiReplayConfigError::EmptyPath);
        }
        let path = Path::new(&self.path);
        if !path.exists() {
            return Err(CsiReplayConfigError::FixtureMissing(self.path.clone()));
        }
        if !path.is_file() {
            return Err(CsiReplayConfigError::FixtureNotFile(self.path.clone()));
        }
        Ok(())
    }

    /// Returns a repository-relative display path suitable for API responses.
    pub fn display_path(&self) -> String {
        self.path.to_string_lossy().replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn defaults_are_disabled_and_valid() {
        let config = CsiReplayConfig::default();
        assert!(!config.enabled);
        config.validate().expect("disabled defaults valid");
    }

    #[test]
    fn zero_interval_is_rejected() {
        let config = CsiReplayConfig {
            frame_interval_ms: 0,
            ..CsiReplayConfig::default()
        };
        assert_eq!(
            config.validate().expect_err("invalid"),
            CsiReplayConfigError::ZeroInterval
        );
    }

    #[test]
    fn enabled_missing_fixture_is_rejected() {
        let config = CsiReplayConfig {
            enabled: true,
            path: PathBuf::from("does-not-exist.ndjson"),
            ..CsiReplayConfig::default()
        };
        assert!(matches!(
            config.validate().expect_err("missing"),
            CsiReplayConfigError::FixtureMissing(_)
        ));
    }

    #[test]
    fn enabled_existing_fixture_is_accepted() {
        let mut file = NamedTempFile::new().expect("temp");
        writeln!(file, "placeholder").expect("write");
        let config = CsiReplayConfig {
            enabled: true,
            path: file.path().to_path_buf(),
            ..CsiReplayConfig::default()
        };
        config.validate().expect("valid");
    }
}
