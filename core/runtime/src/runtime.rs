//! Application runtime lifecycle management.

use aeryon_plugin_runtime::{LifecycleState, PluginRuntime};

use crate::config::AppConfig;
use crate::context::AppContext;
use crate::error::{LoggingError, RuntimeError};
use crate::health::RuntimeHealth;
use crate::logging::init_logging;

/// Coordinates application startup, shutdown, and health reporting.
pub struct Runtime {
    context: AppContext,
    health: RuntimeHealth,
}

impl Runtime {
    /// Boots the runtime using `config`.
    ///
    /// Initializes logging and the plugin runtime. The runtime remains in the
    /// `Starting` state until [`start`](Self::start) is called.
    pub fn boot(config: AppConfig) -> Result<Self, RuntimeError> {
        if let Err(error) = init_logging(&config.logging) {
            if error != LoggingError::AlreadyInitialized {
                return Err(RuntimeError::Logging(error));
            }
        }

        tracing::info!("startup");
        tracing::info!(environment = %config.application.environment, "configuration loaded");

        let plugin_runtime = PluginRuntime::new();
        tracing::info!(
            enabled = config.plugins.enabled,
            autoload = config.plugins.autoload,
            "plugin runtime initialized"
        );

        let context = AppContext::new(config, plugin_runtime, env!("CARGO_PKG_VERSION"));

        Ok(Self {
            context,
            health: RuntimeHealth::Starting,
        })
    }

    /// Transitions the runtime to the `Running` state.
    pub fn start(&mut self) -> Result<(), RuntimeError> {
        self.require_health(RuntimeHealth::Starting)?;
        tracing::info!("runtime entering running state");
        self.health = RuntimeHealth::Running;
        Ok(())
    }

    /// Shuts down the runtime and stops active plugins.
    pub fn shutdown(&mut self) -> Result<(), RuntimeError> {
        if self.health == RuntimeHealth::Stopped {
            return Ok(());
        }

        if self.health == RuntimeHealth::Failed {
            return Err(RuntimeError::InvalidState {
                expected: RuntimeHealth::Running,
                actual: self.health,
            });
        }

        self.health = RuntimeHealth::Stopping;
        tracing::info!("shutdown");

        if self.context.config.plugins.enabled {
            let running_plugins: Vec<_> = self
                .context
                .plugin_runtime
                .lifecycle_snapshot()
                .into_iter()
                .filter(|(_, state)| *state == LifecycleState::Running)
                .map(|(id, _)| id)
                .collect();

            for plugin_id in running_plugins {
                self.context.plugin_runtime.stop(&plugin_id)?;
            }
        }

        self.health = RuntimeHealth::Stopped;
        tracing::info!("runtime stopped");
        Ok(())
    }

    /// Returns the current runtime health state.
    pub fn health(&self) -> RuntimeHealth {
        self.health
    }

    /// Returns the application context.
    pub fn context(&self) -> &AppContext {
        &self.context
    }

    /// Returns a concise startup summary for operator output.
    pub fn startup_summary(&self) -> String {
        format!(
            "Aeryon {} | environment={} | plugins={} | status={}",
            self.context.version,
            self.context.config.application.environment,
            if self.context.config.plugins.enabled {
                "enabled"
            } else {
                "disabled"
            },
            self.health
        )
    }

    fn require_health(&self, expected: RuntimeHealth) -> Result<(), RuntimeError> {
        if self.health == expected {
            Ok(())
        } else {
            Err(RuntimeError::InvalidState {
                expected,
                actual: self.health,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn boot_initializes_in_starting_state() {
        let runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        assert_eq!(runtime.health(), RuntimeHealth::Starting);
        assert!(runtime.context().config.plugins.enabled);
    }

    #[test]
    fn start_transitions_to_running() {
        let mut runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        assert_eq!(runtime.health(), RuntimeHealth::Running);
    }

    #[test]
    fn shutdown_transitions_to_stopped() {
        let mut runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        runtime.shutdown().expect("shutdown succeeds");
        assert_eq!(runtime.health(), RuntimeHealth::Stopped);
    }

    #[test]
    fn start_rejects_invalid_transition() {
        let mut runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        runtime.start().expect("first start succeeds");
        let error = runtime.start().expect_err("second start fails");
        assert!(matches!(
            error,
            RuntimeError::InvalidState {
                expected: RuntimeHealth::Starting,
                actual: RuntimeHealth::Running,
            }
        ));
    }

    #[test]
    fn startup_summary_contains_version_and_status() {
        let mut runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        let summary = runtime.startup_summary();
        assert!(summary.contains("Aeryon"));
        assert!(summary.contains("running"));
    }
}
