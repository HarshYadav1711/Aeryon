//! Shared application context.

use std::time::Instant;

use aeryon_plugin_runtime::PluginRuntime;

use crate::config::AppConfig;

/// Shared state owned by the running application.
pub struct AppContext {
    /// Loaded application configuration.
    pub config: AppConfig,
    /// Plugin runtime instance.
    pub plugin_runtime: PluginRuntime,
    /// Time the context was created.
    pub started_at: Instant,
    /// Application version string.
    pub version: &'static str,
}

impl AppContext {
    /// Creates a new application context.
    pub fn new(config: AppConfig, plugin_runtime: PluginRuntime, version: &'static str) -> Self {
        Self {
            config,
            plugin_runtime,
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
    use crate::config::AppConfig;

    #[test]
    fn context_tracks_version_and_config() {
        let context = AppContext::new(AppConfig::default(), PluginRuntime::new(), "0.1.0");
        assert_eq!(context.version, "0.1.0");
        assert_eq!(context.config.application.name, "aeryon");
    }
}
