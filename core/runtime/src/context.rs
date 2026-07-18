//! Shared application context.

use std::sync::Arc;
use std::time::Instant;

use aeryon_events::EventBus;
use aeryon_plugin_runtime::PluginRuntime;

use crate::config::AppConfig;
use crate::metrics::RuntimeMetrics;

/// Shared state owned by the running application.
pub struct AppContext {
    /// Loaded application configuration.
    pub config: AppConfig,
    /// Plugin runtime instance.
    pub plugin_runtime: PluginRuntime,
    /// Typed in-process event bus.
    pub event_bus: EventBus,
    /// Shared runtime statistics.
    pub metrics: Arc<RuntimeMetrics>,
    /// Time the context was created.
    pub started_at: Instant,
    /// Application version string.
    pub version: &'static str,
}

impl AppContext {
    /// Creates a new application context.
    pub fn new(
        config: AppConfig,
        plugin_runtime: PluginRuntime,
        event_bus: EventBus,
        metrics: Arc<RuntimeMetrics>,
        version: &'static str,
    ) -> Self {
        Self {
            config,
            plugin_runtime,
            event_bus,
            metrics,
            started_at: Instant::now(),
            version,
        }
    }

    /// Returns elapsed time since the context was created.
    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_tracks_version_and_config() {
        let context = AppContext::new(
            AppConfig::default(),
            PluginRuntime::new(),
            EventBus::new(),
            RuntimeMetrics::new().shared(),
            "0.1.0",
        );
        assert_eq!(context.version, "0.1.0");
        assert_eq!(context.config.application.name, "aeryon");
    }
}
