//! Application configuration.

use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;

use aeryon_calibration::CalibrationConfig;
use aeryon_csi_replay::CsiReplayConfig;
use aeryon_dsp::DspConfig;
use aeryon_features::FeaturesConfig;
use aeryon_perception::PerceptionConfig;
use aeryon_synthetic_sensor::SyntheticSensorConfig;
use serde::Deserialize;

use crate::error::ConfigError;

/// Default TOML configuration shipped with the platform.
pub const DEFAULT_CONFIG: &str = r#"[application]
name = "aeryon"
environment = "development"

[logging]
level = "info"

[plugins]
enabled = true
autoload = false

[runtime]
shutdown_timeout_secs = 10
first_frame_timeout_ms = 2000

[api]
enabled = true
host = "127.0.0.1"
port = 8080
cors_origins = ["http://127.0.0.1:5173", "http://localhost:5173"]

[synthetic_sensor]
enabled = true
interval_ms = 100
samples_per_frame = 64
sample_rate_hz = 1000.0
primary_frequency_hz = 10.0
secondary_frequency_hz = 37.0
secondary_amplitude = 0.25
log_every_n_frames = 10

[sensors.csi_replay]
enabled = false
path = "datasets/fixtures/csi/synthetic_dev_v1.ndjson"
loop_playback = false
frame_interval_ms = 100
maximum_frames = 0

[calibration]
enabled = true
profile = "baseline-csi-v1"
queue_capacity = 64

[calibration.baseline_csi_v1.phase_unwrap]
enabled = true

[calibration.baseline_csi_v1.linear_phase_detrend]
enabled = true

[calibration.baseline_csi_v1.rms_amplitude_normalize]
enabled = true
epsilon = 1.0e-8

[dsp]
enabled = false
profile = "baseline-dsp-v1"
queue_capacity = 64
window_size_frames = 16
hop_size_frames = 4
maximum_sequence_gap = 1
timestamp_jitter_tolerance = 0.10

[features]
enabled = false
profile = "baseline-features-v1"
queue_capacity = 64

[perception]
enabled = false
profile = "channel-change-v1"
queue_capacity = 64

[perception.channel_change_v1]
stable_threshold = 0.22
high_change_threshold = 0.55
motion_energy_rms_scale = 0.35
motion_energy_p95_scale = 0.55
minimum_margin = 0.0
maximum_timestamp_jitter = 0.10
"#;

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AppConfig {
    /// Application metadata.
    pub application: ApplicationConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
    /// Plugin subsystem configuration.
    pub plugins: PluginsConfig,
    /// Runtime behavior configuration.
    pub runtime: RuntimeSettings,
    /// Local HTTP API configuration.
    #[serde(default)]
    pub api: ApiConfig,
    /// Synthetic sensor configuration.
    #[serde(default)]
    pub synthetic_sensor: SyntheticSensorConfig,
    /// Sensor plugin configuration group.
    #[serde(default)]
    pub sensors: SensorsConfig,
    /// CSI calibration configuration.
    #[serde(default)]
    pub calibration: CalibrationConfig,
    /// Temporal CSI DSP configuration.
    #[serde(default)]
    pub dsp: DspConfig,
    /// CSI feature extraction configuration.
    #[serde(default)]
    pub features: FeaturesConfig,
    /// Channel-change perception configuration.
    #[serde(default)]
    pub perception: PerceptionConfig,
}

/// Nested sensor plugin configuration sections.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct SensorsConfig {
    /// CSI fixture replay configuration.
    #[serde(default)]
    pub csi_replay: CsiReplayConfig,
}

/// Application metadata and environment settings.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApplicationConfig {
    /// Application name.
    pub name: String,
    /// Deployment environment label.
    pub environment: String,
}

/// Logging configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoggingConfig {
    /// Log level filter (for example `info` or `debug`).
    pub level: String,
}

/// Plugin subsystem configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PluginsConfig {
    /// Whether plugin support is enabled.
    pub enabled: bool,
    /// Whether configured plugins should be loaded automatically at startup.
    pub autoload: bool,
}

/// Runtime behavior configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RuntimeSettings {
    /// Graceful shutdown timeout in seconds.
    pub shutdown_timeout_secs: u64,
    /// Maximum wait for the first frame after sensor start.
    #[serde(default = "default_first_frame_timeout_ms")]
    pub first_frame_timeout_ms: u64,
}

/// Local development HTTP API configuration.
///
/// This surface is local-development infrastructure for the live dashboard
/// slice. It is not a production-hardened public API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiConfig {
    /// Whether the HTTP API should bind and serve requests.
    #[serde(default = "default_api_enabled")]
    pub enabled: bool,
    /// Bind host (IPv4/IPv6 address).
    #[serde(default = "default_api_host")]
    pub host: String,
    /// Bind port.
    #[serde(default = "default_api_port")]
    pub port: u16,
    /// Allowed CORS origins for the local frontend.
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
}

fn default_first_frame_timeout_ms() -> u64 {
    2_000
}

fn default_api_enabled() -> bool {
    true
}

fn default_api_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_api_port() -> u16 {
    8080
}

fn default_cors_origins() -> Vec<String> {
    vec![
        "http://127.0.0.1:5173".to_owned(),
        "http://localhost:5173".to_owned(),
    ]
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: default_api_enabled(),
            host: default_api_host(),
            port: default_api_port(),
            cors_origins: default_cors_origins(),
        }
    }
}

impl ApiConfig {
    /// Validates host, port, and CORS origin configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::InvalidApiPort(self.port));
        }

        if self.host.trim().is_empty() {
            return Err(ConfigError::InvalidApiHost(self.host.clone()));
        }

        // Reject host values that already embed a port; bind address is host+port.
        if self.host.contains(':') && self.host.parse::<SocketAddr>().is_ok() {
            return Err(ConfigError::InvalidApiBind(format!(
                "host `{host}` includes a port; set host and port separately",
                host = self.host
            )));
        }

        if IpAddr::from_str(self.host.trim()).is_err() {
            return Err(ConfigError::InvalidApiHost(self.host.clone()));
        }

        for origin in &self.cors_origins {
            validate_cors_origin(origin)?;
        }

        Ok(())
    }

    /// Returns the socket address used to bind the HTTP server.
    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        self.validate()?;
        let ip = IpAddr::from_str(self.host.trim())
            .map_err(|_| ConfigError::InvalidApiHost(self.host.clone()))?;
        Ok(SocketAddr::new(ip, self.port))
    }
}

fn validate_cors_origin(origin: &str) -> Result<(), ConfigError> {
    let trimmed = origin.trim();
    if trimmed.is_empty() || trimmed == "*" {
        return Err(ConfigError::InvalidCorsOrigin(origin.to_owned()));
    }

    let (scheme, remainder) = trimmed
        .split_once("://")
        .ok_or_else(|| ConfigError::InvalidCorsOrigin(origin.to_owned()))?;

    if scheme != "http" && scheme != "https" {
        return Err(ConfigError::InvalidCorsOrigin(origin.to_owned()));
    }

    let host_port = remainder
        .split('/')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();

    if host_port.is_empty() || host_port == "*" {
        return Err(ConfigError::InvalidCorsOrigin(origin.to_owned()));
    }

    Ok(())
}

impl AppConfig {
    /// Returns the default configuration.
    pub fn default_config() -> Self {
        Self::from_toml(DEFAULT_CONFIG).expect("default configuration must be valid")
    }

    /// Parses configuration from a TOML string and validates it.
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads configuration from a TOML file.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(ConfigError::Io)?;
        Self::from_toml(&source)
    }

    /// Loads configuration from `path` when present, otherwise returns defaults.
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            Self::load_from_path(path)
        } else {
            Ok(Self::default_config())
        }
    }

    /// Validates nested configuration sections.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.api.validate()?;
        self.synthetic_sensor
            .validate()
            .map_err(ConfigError::Synthetic)?;
        self.sensors
            .csi_replay
            .validate()
            .map_err(ConfigError::CsiReplay)?;
        self.calibration
            .validate()
            .map_err(ConfigError::Calibration)?;
        self.dsp.validate().map_err(ConfigError::Dsp)?;
        self.features.validate().map_err(ConfigError::Features)?;
        self.perception
            .validate()
            .map_err(ConfigError::Perception)?;
        if self.dsp.enabled && !self.calibration.enabled {
            return Err(ConfigError::DspRequiresCalibration);
        }
        if self.features.enabled && !self.dsp.enabled {
            return Err(ConfigError::FeaturesRequireDsp);
        }
        if self.perception.enabled && !self.features.enabled {
            return Err(ConfigError::PerceptionRequiresFeatures);
        }
        if self.synthetic_sensor.enabled && self.sensors.csi_replay.enabled {
            return Err(ConfigError::ConflictingSensorSources);
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses() {
        let config = AppConfig::default_config();
        assert_eq!(config.application.name, "aeryon");
        assert_eq!(config.logging.level, "info");
        assert!(config.plugins.enabled);
        assert!(config.synthetic_sensor.enabled);
        assert!(!config.sensors.csi_replay.enabled);
    }

    #[test]
    fn invalid_toml_is_rejected() {
        let error = AppConfig::from_toml("application =").expect_err("invalid toml");
        assert!(matches!(error, ConfigError::Parse(_)));
    }

    #[test]
    fn invalid_synthetic_config_is_rejected() {
        let error = AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10

            [synthetic_sensor]
            enabled = true
            interval_ms = 0
            "#,
        )
        .expect_err("invalid synthetic config");
        assert!(matches!(error, ConfigError::Synthetic(_)));
    }

    #[test]
    fn conflicting_sensor_sources_are_rejected() {
        let error = AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10

            [synthetic_sensor]
            enabled = true

            [sensors.csi_replay]
            enabled = true
            path = "datasets/fixtures/csi/synthetic_dev_v1.ndjson"
            frame_interval_ms = 100
            "#,
        )
        .expect_err("rejected");
        assert!(
            matches!(
                error,
                ConfigError::ConflictingSensorSources | ConfigError::CsiReplay(_)
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn custom_toml_overrides_defaults() {
        let config = AppConfig::from_toml(
            r#"
            [application]
            name = "testbed"
            environment = "staging"

            [logging]
            level = "debug"

            [plugins]
            enabled = false
            autoload = true

            [runtime]
            shutdown_timeout_secs = 5
            first_frame_timeout_ms = 500

            [api]
            enabled = false
            host = "0.0.0.0"
            port = 9090
            cors_origins = ["http://127.0.0.1:3000"]

            [synthetic_sensor]
            enabled = false
            interval_ms = 50
            samples_per_frame = 32
            "#,
        )
        .expect("valid config");

        assert_eq!(config.application.environment, "staging");
        assert_eq!(config.logging.level, "debug");
        assert!(!config.plugins.enabled);
        assert_eq!(config.runtime.shutdown_timeout_secs, 5);
        assert!(!config.api.enabled);
        assert_eq!(config.api.port, 9090);
        assert!(!config.synthetic_sensor.enabled);
        assert_eq!(config.synthetic_sensor.samples_per_frame, 32);
    }

    #[test]
    fn valid_api_configuration_loads() {
        let config = AppConfig::default_config();
        assert!(config.api.enabled);
        assert_eq!(
            config.api.socket_addr().expect("bind addr"),
            "127.0.0.1:8080".parse().expect("parse")
        );
    }

    #[test]
    fn invalid_api_port_is_rejected() {
        let error = AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10

            [api]
            enabled = true
            host = "127.0.0.1"
            port = 0
            "#,
        )
        .expect_err("port 0 rejected");
        assert!(matches!(error, ConfigError::InvalidApiPort(0)));
    }

    #[test]
    fn invalid_api_host_socket_combo_is_rejected() {
        let error = AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10

            [api]
            enabled = true
            host = "127.0.0.1:8080"
            port = 8080
            "#,
        )
        .expect_err("combined host:port rejected");
        assert!(matches!(error, ConfigError::InvalidApiBind(_)));
    }

    #[test]
    fn wildcard_cors_origin_is_rejected() {
        let error = AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10

            [api]
            enabled = true
            host = "127.0.0.1"
            port = 8080
            cors_origins = ["*"]
            "#,
        )
        .expect_err("wildcard cors rejected");
        assert!(matches!(error, ConfigError::InvalidCorsOrigin(_)));
    }
}
