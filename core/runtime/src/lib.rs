//! Application runtime for the Aeryon perception platform.
//!
//! This crate boots the platform, loads configuration, initializes logging and
//! the plugin runtime, and coordinates graceful shutdown.

#![deny(missing_docs)]

pub mod config;
pub mod context;
pub mod error;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod runtime;

pub use config::{
    ApiConfig, AppConfig, ApplicationConfig, DEFAULT_CONFIG, LoggingConfig, PluginsConfig,
    RuntimeSettings, SensorsConfig,
};
pub use context::AppContext;
pub use error::{ConfigError, LoggingError, RuntimeError};
pub use health::RuntimeHealth;
pub use metrics::RuntimeMetrics;
pub use runtime::Runtime;

/// Returns the runtime crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns the standard Aeryon startup banner.
pub fn banner() -> &'static str {
    "Aeryon\nTransforming Signals into Understanding"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn banner_contains_project_name() {
        assert!(banner().contains("Aeryon"));
    }
}
