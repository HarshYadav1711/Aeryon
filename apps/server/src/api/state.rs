//! Shared Axum application state backed by the live [`Runtime`].

use std::sync::Arc;

use aeryon_csi_replay::PLUGIN_ID as CSI_REPLAY_PLUGIN_ID;
use aeryon_plugin_runtime::{Capability, HealthStatus, LifecycleState, PluginId};
use aeryon_runtime::{Runtime, RuntimeHealth};
use aeryon_synthetic_sensor::PLUGIN_ID as SYNTHETIC_PLUGIN_ID;
use tokio::sync::RwLock;

use super::dto::{
    CalibrationSnapshot, ConfiguredFrequencies, CsiReplayHealthSummary, CsiReplaySnapshot,
    HealthResponse, PluginSummary, PluginsResponse, RuntimeSnapshot, SyntheticHealthSummary,
    SyntheticSensorSnapshot,
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

        let response = HealthResponse {
            status: health.to_string(),
            healthy: is_http_healthy(health),
            uptime_secs: duration_secs(context.uptime()),
            timestamp: now_rfc3339(),
            event_consumer_running: metrics.consumer_running(),
            synthetic_sensor: synthetic_health_summary(&runtime),
            csi_replay: csi_replay_health_summary(&runtime),
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
            csi_replay_lifecycle: metrics.csi_lifecycle().map(|state| state.to_string()),
            csi_replay_enabled: context.config.sensors.csi_replay.enabled,
            active_source: active_source_label(
                context.config.synthetic_sensor.enabled,
                context.config.sensors.csi_replay.enabled,
            )
            .to_owned(),
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
        let plugin_id = PluginId::new(SYNTHETIC_PLUGIN_ID);

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
            frames_received: if config.enabled {
                metrics.frames_received()
            } else {
                0
            },
            last_sequence: if config.enabled {
                metrics.last_sequence()
            } else {
                None
            },
            last_frame_timestamp: if config.enabled {
                metrics.last_frame_nanos().map(nanos_to_rfc3339)
            } else {
                None
            },
            health,
        }
    }

    /// Builds the CSI replay snapshot from config + live metrics.
    pub async fn csi_replay_snapshot(&self) -> CsiReplaySnapshot {
        let runtime = self.runtime.read().await;
        let context = runtime.context();
        let metrics = runtime.metrics();
        let config = &context.config.sensors.csi_replay;
        let stats = metrics.csi_replay();
        let plugin_id = PluginId::new(CSI_REPLAY_PLUGIN_ID);

        let registered = context.plugin_runtime.lifecycle_state(&plugin_id).is_some();
        let lifecycle_state = context
            .plugin_runtime
            .lifecycle_state(&plugin_id)
            .or_else(|| metrics.csi_lifecycle())
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

        CsiReplaySnapshot {
            enabled: config.enabled,
            lifecycle_state,
            health,
            source_type: "csi_replay",
            data_classification: "deterministic_development_fixture",
            fixture_path: config.display_path(),
            loop_playback: config.loop_playback,
            frame_interval_ms: config.frame_interval_ms,
            maximum_frames: config.maximum_frames,
            frames_read: stats.frames_read(),
            frames_accepted: stats.frames_accepted(),
            frames_rejected: stats.frames_rejected(),
            latest_sequence: stats.latest_sequence(),
            latest_frame_timestamp: stats.latest_frame_nanos().map(nanos_to_rfc3339),
            receive_antennas: stats.receive_antennas(),
            transmit_antennas: stats.transmit_antennas(),
            subcarrier_count: stats.subcarrier_count(),
            center_frequency_hz: stats.center_frequency_hz(),
            bandwidth_hz: stats.bandwidth_hz(),
            completion: stats.completion().as_str().to_owned(),
            last_error: stats.last_error(),
        }
    }

    /// Builds the calibration snapshot from config + live metrics.
    pub async fn calibration_snapshot(&self) -> CalibrationSnapshot {
        let runtime = self.runtime.read().await;
        let context = runtime.context();
        let stats = runtime.metrics().calibration();
        let config = &context.config.calibration;

        let (profile_id, profile_version, stages) = if config.enabled {
            match config.resolve_profile() {
                Ok(profile) => (
                    Some(profile.id.clone()),
                    Some(profile.version),
                    profile
                        .enabled_stage_names()
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
                Err(_) => (stats.profile_id(), stats.profile_version(), Vec::new()),
            }
        } else {
            (None, None, Vec::new())
        };

        let health = if !config.enabled {
            "disabled".to_owned()
        } else if let Some(impact) = stats.evaluate_health() {
            impact.to_string()
        } else {
            match stats.worker_state() {
                aeryon_runtime::CalibrationWorkerState::Failed => "failed".to_owned(),
                aeryon_runtime::CalibrationWorkerState::Running => "healthy".to_owned(),
                aeryon_runtime::CalibrationWorkerState::Stopped => "stopped".to_owned(),
                aeryon_runtime::CalibrationWorkerState::Idle => "idle".to_owned(),
                aeryon_runtime::CalibrationWorkerState::Disabled => "disabled".to_owned(),
            }
        };

        CalibrationSnapshot {
            enabled: config.enabled,
            worker_state: stats.worker_state().as_str().to_owned(),
            profile_id,
            profile_version,
            stages,
            raw_frames_submitted: stats.raw_frames_submitted(),
            frames_calibrated: stats.frames_calibrated(),
            frames_failed: stats.calibration_failures(),
            latest_sequence: stats.latest_sequence(),
            latest_calibrated_timestamp: stats.latest_timestamp_nanos().map(nanos_to_rfc3339),
            last_duration_ns: stats.last_duration_ns(),
            average_duration_ns: stats.average_duration_ns(),
            last_warning: stats.last_warning(),
            last_error: stats.last_error(),
            queue_depth: stats.queue_depth(),
            health,
            data_classification: "csi_replay_development_source",
        }
    }
}

fn active_source_label(synthetic: bool, csi_replay: bool) -> &'static str {
    match (synthetic, csi_replay) {
        (true, false) => "synthetic",
        (false, true) => "csi_replay",
        (false, false) => "none",
        (true, true) => "invalid",
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
    let plugin_id = PluginId::new(SYNTHETIC_PLUGIN_ID);
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

fn csi_replay_health_summary(runtime: &Runtime) -> CsiReplayHealthSummary {
    let enabled = runtime.context().config.sensors.csi_replay.enabled;
    let plugin_id = PluginId::new(CSI_REPLAY_PLUGIN_ID);
    let lifecycle_state = runtime
        .context()
        .plugin_runtime
        .lifecycle_state(&plugin_id)
        .or_else(|| runtime.metrics().csi_lifecycle())
        .map(|state| state.to_string());
    let health = runtime
        .context()
        .plugin_runtime
        .health(&plugin_id)
        .ok()
        .map(health_status_label)
        .map(str::to_owned);
    let completion = if enabled {
        Some(
            runtime
                .metrics()
                .csi_replay()
                .completion()
                .as_str()
                .to_owned(),
        )
    } else {
        None
    };

    CsiReplayHealthSummary {
        enabled,
        lifecycle_state,
        health,
        completion,
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
