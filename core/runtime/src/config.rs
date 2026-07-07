//! Application configuration.

use std::fs;
use std::path::Path;

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
}

impl AppConfig {
    /// Returns the default configuration.
    pub fn default_config() -> Self {
        Self::from_toml(DEFAULT_CONFIG).expect("default configuration must be valid")
    }

    /// Parses configuration from a TOML string.
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        toml::from_str(source).map_err(ConfigError::Parse)
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
    }

    #[test]
    fn invalid_toml_is_rejected() {
        let error = AppConfig::from_toml("application =").expect_err("invalid toml");
        assert!(matches!(error, ConfigError::Parse(_)));
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
            "#,
        )
        .expect("valid config");

        assert_eq!(config.application.environment, "staging");
        assert_eq!(config.logging.level, "debug");
        assert!(!config.plugins.enabled);
        assert_eq!(config.runtime.shutdown_timeout_secs, 5);
    }
}
