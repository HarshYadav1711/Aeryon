//! Shared Axum application state backed by the live [`Runtime`].

use std::sync::Arc;

use aeryon_plugin_runtime::{Capability, HealthStatus, LifecycleState, PluginId};
use aeryon_runtime::{Runtime, RuntimeHealth};
use aeryon_synthetic_sensor::PLUGIN_ID;
use tokio::sync::RwLock;

use super::dto::{
    ConfiguredFrequencies, HealthResponse, PluginSummary, PluginsResponse, RuntimeSnapshot,
    SyntheticHealthSummary, SyntheticSensorSnapshot,
};
use super::time::{duration_secs, nanos_to_rfc3339, now_rfc3339, system_time_to_rfc3339};

/// Shared server state. Handlers read the running application through this handle.
#[derive(Clone)]
pub struct AppState {
    runtime: Arc<RwLock<Runtime>>,
}

impl AppState {
    /// Wraps a running runtime for API access.
    pub fn new(runtime: Arc<RwLock<Runtime>>) -> Self {
        Self { runtime }
    }

    /// Returns the shared runtime lock.
    pub fn runtime(&self) -> &Arc<RwLock<Runtime>> {
        &self.runtime
    }

    /// Builds a health DTO and HTTP-healthy flag from live runtime state.
    pub async fn health_snapshot(&self) -> (HealthResponse, bool) {
        let mut runtime = self.runtime.write().await;
        runtime.refresh_health();
        let health = runtime.health();
        let context = runtime.context();
        let metrics = runtime.metrics();

        let synthetic = synthetic_health_summary(&runtime);
        let response = HealthResponse {
            status: health.to_string(),
            healthy: is_http_healthy(health),
            uptime_secs: duration_secs(context.uptime()),
            timestamp: now_rfc3339(),
            event_consumer_running: metrics.consumer_running(),
            synthetic_sensor: synthetic,
        };
        let healthy = response.healthy;
        (response, healthy)
    }

    /// Builds a runtime snapshot DTO.
    pub async fn runtime_snapshot(&self) -> RuntimeSnapshot {
        let mut runtime = self.runtime.write().await;
        runtime.refresh_health();
        let health = runtime.health();
        let context = runtime.context();
        let metrics = runtime.metrics();

        let plugins = context.plugin_runtime.registry().list();
        let registered_plugin_count = plugins.len();
        let active_plugin_count = context
            .plugin_runtime
            .lifecycle_snapshot()
            .into_iter()
            .filter(|(_, state)| *state == LifecycleState::Running)
            .count();

        RuntimeSnapshot {
            application_name: context.config.application.name.clone(),
            application_version: context.version.to_owned(),
            lifecycle_state: health.to_string(),
            uptime_secs: duration_secs(context.uptime()),
            startup_timestamp: system_time_to_rfc3339(context.started_at_wall),
            registered_plugin_count,
            active_plugin_count,
            frames_received: metrics.frames_received(),
            last_frame_sequence: metrics.last_sequence(),
            last_frame_timestamp: metrics.last_frame_nanos().map(nanos_to_rfc3339),
            synthetic_sensor_lifecycle: metrics.sensor_lifecycle().map(|state| state.to_string()),
            synthetic_source_enabled: context.config.synthetic_sensor.enabled,
        }
    }

    /// Builds plugin summary DTOs.
    pub async fn plugins_snapshot(&self) -> PluginsResponse {
        let runtime = self.runtime.read().await;
        let context = runtime.context();
        let mut plugins = Vec::new();

        for metadata in context.plugin_runtime.registry().list() {
            let lifecycle_state = context
                .plugin_runtime
                .lifecycle_state(&metadata.id)
                .unwrap_or(LifecycleState::Registered);
            let health = context
                .plugin_runtime
                .health(&metadata.id)
                .unwrap_or(HealthStatus::Unhealthy);

            plugins.push(PluginSummary {
                id: metadata.id.to_string(),
                name: metadata.name,
                version: metadata.version.to_string(),
                capabilities: metadata
                    .capabilities
                    .iter()
                    .copied()
                    .map(capability_label)
                    .map(str::to_owned)
                    .collect(),
                lifecycle_state: lifecycle_state.to_string(),
                health: health_status_label(health).to_owned(),
            });
        }

        PluginsResponse { plugins }
    }

    /// Builds the synthetic sensor snapshot from config + live metrics.
    pub async fn synthetic_sensor_snapshot(&self) -> SyntheticSensorSnapshot {
        let runtime = self.runtime.read().await;
        let context = runtime.context();
        let metrics = runtime.metrics();
        let config = &context.config.synthetic_sensor;
        let plugin_id = PluginId::new(PLUGIN_ID);

        let registered = context.plugin_runtime.lifecycle_state(&plugin_id).is_some();
        let lifecycle_state = context
            .plugin_runtime
            .lifecycle_state(&plugin_id)
            .or_else(|| metrics.sensor_lifecycle())
            .map(|state| state.to_string());
        let health = if registered {
            context
                .plugin_runtime
                .health(&plugin_id)
                .ok()
                .map(health_status_label)
                .map(str::to_owned)
        } else {
            None
        };

        SyntheticSensorSnapshot {
            enabled: config.enabled,
            lifecycle_state,
            configured_interval_ms: config.interval_ms,
            samples_per_frame: config.samples_per_frame,
            sample_rate_hz: config.sample_rate_hz,
            configured_frequencies_hz: ConfiguredFrequencies {
                primary_hz: config.primary_frequency_hz,
                secondary_hz: config.secondary_frequency_hz,
            },
            frames_received: metrics.frames_received(),
            last_sequence: metrics.last_sequence(),
            last_frame_timestamp: metrics.last_frame_nanos().map(nanos_to_rfc3339),
            health,
        }
    }
}

fn is_http_healthy(health: RuntimeHealth) -> bool {
    matches!(
        health,
        RuntimeHealth::Running | RuntimeHealth::Degraded | RuntimeHealth::Starting
    )
}

fn synthetic_health_summary(runtime: &Runtime) -> SyntheticHealthSummary {
    let enabled = runtime.context().config.synthetic_sensor.enabled;
    let plugin_id = PluginId::new(PLUGIN_ID);
    let lifecycle_state = runtime
        .context()
        .plugin_runtime
        .lifecycle_state(&plugin_id)
        .or_else(|| runtime.metrics().sensor_lifecycle())
        .map(|state| state.to_string());
    let health = runtime
        .context()
        .plugin_runtime
        .health(&plugin_id)
        .ok()
        .map(health_status_label)
        .map(str::to_owned);

    SyntheticHealthSummary {
        enabled,
        lifecycle_state,
        health,
    }
}

fn capability_label(capability: Capability) -> &'static str {
    match capability {
        Capability::Sensor => "sensor",
        Capability::Calibration => "calibration",
        Capability::Dsp => "dsp",
        Capability::FeatureExtraction => "feature_extraction",
        Capability::Inference => "inference",
        Capability::Visualization => "visualization",
        Capability::Storage => "storage",
        Capability::Exporter => "exporter",
        Capability::Importer => "importer",
        Capability::Configuration => "configuration",
        Capability::Logging => "logging",
    }
}

fn health_status_label(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
    }
}
