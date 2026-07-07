//! Structured logging initialization.

use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;
use crate::error::LoggingError;

static LOGGING: OnceLock<Result<(), LoggingError>> = OnceLock::new();

/// Initializes the global tracing subscriber from `config`.
///
/// Subsequent calls are ignored after the first successful initialization.
pub fn init_logging(config: &LoggingConfig) -> Result<(), LoggingError> {
    validate_level(&config.level)?;

    let level = config.level.clone();

    *LOGGING.get_or_init(|| {
        let filter = EnvFilter::try_new(level.as_str()).map_err(|_| LoggingError::InvalidLevel)?;

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .try_init()
            .map_err(|_| LoggingError::AlreadyInitialized)
    })
}

fn validate_level(level: &str) -> Result<(), LoggingError> {
    match level {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(LoggingError::InvalidLevel),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;

    #[test]
    fn invalid_log_level_is_rejected() {
        let config = LoggingConfig {
            level: "verbose".into(),
        };
        let error = init_logging(&config).expect_err("invalid level");
        assert_eq!(error, LoggingError::InvalidLevel);
    }
}
