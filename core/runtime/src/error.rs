//! Runtime error types.

use core::fmt;

use aeryon_plugin_runtime::PluginError;

use crate::health::RuntimeHealth;

/// Errors encountered while loading configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// Configuration file could not be read.
    Io(std::io::Error),
    /// Configuration file could not be parsed.
    Parse(toml::de::Error),
}

/// Errors encountered while initializing logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggingError {
    /// The global tracing subscriber is already installed.
    AlreadyInitialized,
    /// The configured log level is invalid.
    InvalidLevel,
}

/// Errors produced by the application runtime.
#[derive(Debug)]
pub enum RuntimeError {
    /// Configuration failed to load.
    Config(ConfigError),
    /// Logging failed to initialize.
    Logging(LoggingError),
    /// A plugin runtime operation failed.
    Plugin(PluginError),
    /// The runtime is not in the required state.
    InvalidState {
        /// Expected runtime health state.
        expected: RuntimeHealth,
        /// Actual runtime health state.
        actual: RuntimeHealth,
    },
}

impl ConfigError {
    /// Creates an I/O error variant.
    pub fn io(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl RuntimeError {
    /// Creates a configuration error.
    pub fn config(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<ConfigError> for RuntimeError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<PluginError> for RuntimeError {
    fn from(error: PluginError) -> Self {
        Self::Plugin(error)
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "configuration I/O error: {error}"),
            Self::Parse(error) => write!(f, "configuration parse error: {error}"),
        }
    }
}

impl fmt::Display for LoggingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => f.write_str("logging subscriber already initialized"),
            Self::InvalidLevel => f.write_str("invalid logging level"),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "{error}"),
            Self::Logging(error) => write!(f, "{error}"),
            Self::Plugin(error) => write!(f, "plugin runtime error: {error}"),
            Self::InvalidState { expected, actual } => {
                write!(
                    f,
                    "invalid runtime state: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}
impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_error_formats_invalid_state() {
        let error = RuntimeError::InvalidState {
            expected: RuntimeHealth::Running,
            actual: RuntimeHealth::Stopped,
        };
        assert!(error.to_string().contains("running"));
        assert!(error.to_string().contains("stopped"));
    }
}
