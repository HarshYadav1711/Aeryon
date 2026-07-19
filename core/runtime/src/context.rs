//! Shared application context.

use std::sync::Arc;
use std::time::{Instant, SystemTime};

use aeryon_events::EventBus;
use aeryon_plugin_runtime::PluginRuntime;

use crate::config::AppConfig;
use crate::metrics::RuntimeMetrics;
use crate::signal_store::SignalSnapshotStore;

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
    /// Bounded latest-frame / latest-DSP / recent-event store.
    pub signal_store: Arc<SignalSnapshotStore>,
    /// Monotonic time the context was created.
    pub started_at: Instant,
    /// Wall-clock time the context was created.
    pub started_at_wall: SystemTime,
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
        signal_store: Arc<SignalSnapshotStore>,
        version: &'static str,
    ) -> Self {
        Self {
            config,
            plugin_runtime,
            event_bus,
            metrics,
            signal_store,
            started_at: Instant::now(),
            started_at_wall: SystemTime::now(),
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
            SignalSnapshotStore::default().shared(),
            "0.1.0",
        );
        assert_eq!(context.version, "0.1.0");
        assert_eq!(context.config.application.name, "aeryon");
    }
}
