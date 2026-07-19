//! Shared Axum application state backed by the live [`Runtime`].

use std::sync::Arc;

use aeryon_csi_replay::PLUGIN_ID as CSI_REPLAY_PLUGIN_ID;
use aeryon_plugin_runtime::{Capability, HealthStatus, LifecycleState, PluginId};
use aeryon_runtime::{Runtime, RuntimeHealth};
use aeryon_synthetic_sensor::PLUGIN_ID as SYNTHETIC_PLUGIN_ID;
use tokio::sync::RwLock;

use super::dto::{
    CalibratedMagnitudeGridLink, CalibrationSnapshot, ConfiguredFrequencies,
    CsiReplayHealthSummary, CsiReplaySnapshot, DspLatestResponse, DspSnapshot, HealthResponse,
    PluginSummary, PluginsResponse, RecentEventsResponse, RuntimeSnapshot, SignalLatestResponse,
    SyntheticHealthSummary, SyntheticSensorSnapshot,
};
use super::error::ApiError;
use super::event_map::domain_event_to_envelope;
use super::time::{duration_secs, nanos_to_rfc3339, now_rfc3339, system_time_to_rfc3339};

const PIPELINE_DATA_CLASSIFICATION: &str = "csi_replay_development_source";
const AMPLITUDE_UNITS: &str = "dimensionless_magnitude";
const PHASE_UNITS: &str = "radians";
const AMPLITUDE_SEMANTICS: &str =
    "Complex sample magnitude |H|; calibrated values include RMS normalization";
const PHASE_SEMANTICS: &str = "Radians; calibrated phase has subcarrier unwrapping and affine phase detrending (not full hardware calibration)";
const MOTION_ENERGY_SEMANTICS: &str =
    "Channel-change proxy from consecutive calibrated CSI matrices; not human motion or occupancy";
const SPECTRUM_SEMANTICS: &str = "One-sided Hann-windowed power spectrum of the motion-energy proxy; peaks are not activity labels";
const TIMELINE_SEMANTICS: &str =
    "Frequencies and time axes use CSI capture timestamps, not replay wall-clock speed";
const RECENT_EVENTS_DEFAULT_LIMIT: usize = 50;
const RECENT_EVENTS_MAX_LIMIT: usize = 100;

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

    /// Builds the DSP status snapshot from config + live metrics.
    pub async fn dsp_snapshot(&self) -> DspSnapshot {
        let runtime = self.runtime.read().await;
        let context = runtime.context();
        let stats = runtime.metrics().dsp();
        let config = &context.config.dsp;

        let (profile_id, profile_version) = if config.enabled {
            match config.resolve_profile() {
                Ok(profile) => (Some(profile.id.clone()), Some(profile.version)),
                Err(_) => (stats.profile_id(), stats.profile_version()),
            }
        } else {
            (None, None)
        };

        let (latest_first_sequence, latest_last_sequence) = match stats.latest_sequence_range() {
            Some((first, last)) => (Some(first), Some(last)),
            None => (None, None),
        };

        DspSnapshot {
            enabled: config.enabled,
            profile_id,
            profile_version,
            worker_state: stats.worker_state().as_str().to_owned(),
            health: dsp_health_label(stats.as_ref()),
            window_size_frames: if config.enabled {
                config.window_size_frames
            } else {
                stats.window_size_frames()
            },
            hop_size_frames: if config.enabled {
                config.hop_size_frames
            } else {
                stats.hop_size_frames()
            },
            calibrated_frames_received: stats.calibrated_frames_received(),
            windows_emitted: stats.windows_emitted(),
            windows_rejected: stats.windows_rejected(),
            latest_first_sequence,
            latest_last_sequence,
            latest_window_timestamp: stats.latest_window_timestamp_nanos().map(nanos_to_rfc3339),
            effective_sample_rate_hz: stats.effective_sample_rate_hz(),
            timestamp_jitter: stats.latest_timestamp_jitter(),
            latest_dominant_non_dc_hz: stats.latest_dominant_non_dc_hz(),
            last_duration_ns: stats.last_duration_ns(),
            average_duration_ns: stats.average_duration_ns(),
            last_warning: stats.last_warning(),
            last_error: stats.last_error(),
            configured_backend: Some(config.backend.as_str().to_owned()),
            active_backend: stats.active_backend(),
            backend_display_name: Some(config.backend.display_name().to_owned()),
            backend_version: stats.backend_version(),
            backend_abi_version: stats.backend_abi_version(),
            backend_available: if config.enabled {
                config.backend.is_compiled() && stats.backend_available()
            } else {
                config.backend.is_compiled()
            },
            backend_init_status: stats.backend_init_status(),
            last_backend_error: stats.last_backend_error(),
            data_classification: PIPELINE_DATA_CLASSIFICATION,
        }
    }

    /// Builds the latest raw/calibrated signal snapshot for one RX–TX link.
    pub async fn signal_latest_snapshot(
        &self,
        rx: u16,
        tx: u16,
    ) -> Result<SignalLatestResponse, ApiError> {
        let runtime = self.runtime.read().await;
        let store = runtime.signal_store();
        let Some(calibrated) = store.latest_calibrated() else {
            return Ok(SignalLatestResponse::unavailable());
        };
        let raw = store
            .latest_raw()
            .unwrap_or_else(|| Arc::clone(calibrated.raw()));

        if rx >= raw.receive_antennas() || tx >= raw.transmit_antennas() {
            return Err(ApiError::bad_request(
                "invalid_link",
                format!(
                    "rx={rx}, tx={tx} is outside frame antennas {}×{}",
                    raw.receive_antennas(),
                    raw.transmit_antennas()
                ),
            ));
        }

        let raw_amplitudes = raw
            .amplitude_iter(rx, tx)
            .expect("validated link")
            .collect();
        let raw_wrapped_phases = raw.phase_iter(rx, tx).expect("validated link").collect();
        let calibrated_amplitudes = (0..calibrated.subcarrier_count())
            .map(|sc| calibrated.amplitude(rx, tx, sc).expect("validated link"))
            .collect();
        let calibrated_phases = (0..calibrated.subcarrier_count())
            .map(|sc| calibrated.phase(rx, tx, sc).expect("validated link"))
            .collect();

        let mut calibrated_magnitude_grid = Vec::with_capacity(calibrated.link_count());
        for grid_rx in 0..calibrated.receive_antennas() {
            for grid_tx in 0..calibrated.transmit_antennas() {
                let magnitudes = (0..calibrated.subcarrier_count())
                    .map(|sc| {
                        calibrated
                            .amplitude(grid_rx, grid_tx, sc)
                            .expect("validated link")
                    })
                    .collect();
                calibrated_magnitude_grid.push(CalibratedMagnitudeGridLink {
                    rx: grid_rx,
                    tx: grid_tx,
                    magnitudes,
                });
            }
        }

        Ok(SignalLatestResponse {
            available: true,
            source_classification: Some(raw.source().as_str()),
            sensor_id: Some(raw.sensor_id().value()),
            sequence: Some(raw.sequence()),
            capture_timestamp: Some(nanos_to_rfc3339(raw.capture_timestamp().as_nanos())),
            rx: Some(rx),
            tx: Some(tx),
            subcarrier_indices: Some(raw.subcarrier_indices().to_vec()),
            raw_amplitudes: Some(raw_amplitudes),
            calibrated_amplitudes: Some(calibrated_amplitudes),
            raw_wrapped_phases: Some(raw_wrapped_phases),
            calibrated_phases: Some(calibrated_phases),
            raw_frame_id: Some(calibrated.raw_frame_id().value()),
            calibration_profile_id: Some(calibrated.profile_id().to_owned()),
            calibration_profile_version: Some(calibrated.profile_version()),
            amplitude_units: Some(AMPLITUDE_UNITS),
            phase_units: Some(PHASE_UNITS),
            amplitude_semantics: Some(AMPLITUDE_SEMANTICS),
            phase_semantics: Some(PHASE_SEMANTICS),
            data_classification: Some(PIPELINE_DATA_CLASSIFICATION),
            calibrated_magnitude_grid: Some(calibrated_magnitude_grid),
        })
    }

    /// Builds the latest DSP window result for one RX–TX link.
    pub async fn dsp_latest_snapshot(
        &self,
        rx: u16,
        tx: u16,
    ) -> Result<DspLatestResponse, ApiError> {
        let runtime = self.runtime.read().await;
        let Some(result) = runtime.signal_store().latest_dsp() else {
            return Ok(DspLatestResponse::unavailable());
        };

        let link_valid = result
            .antenna_links
            .iter()
            .any(|link| link.rx == rx && link.tx == tx);
        if !link_valid {
            return Err(ApiError::bad_request(
                "invalid_link",
                format!("rx={rx}, tx={tx} is not present in the latest DSP result"),
            ));
        }

        let motion_values = result
            .motion_for_link(rx, tx)
            .map(|values| values.to_vec())
            .unwrap_or_default();
        let spectrum = result.spectrum_for_link(rx, tx);
        let (frequencies, power, dominant) = match spectrum {
            Some(spectrum) => (
                Some(spectrum.frequencies_hz.clone()),
                Some(spectrum.power.clone()),
                spectrum
                    .dominant_non_dc_hz
                    .or_else(|| result.dominant_non_dc_hz()),
            ),
            None => (None, None, result.dominant_non_dc_hz()),
        };

        Ok(DspLatestResponse {
            available: true,
            rx: Some(rx),
            tx: Some(tx),
            sensor_id: Some(result.sensor_id.value()),
            window_id: Some(result.window_id),
            first_sequence: Some(result.first_sequence),
            last_sequence: Some(result.last_sequence),
            first_capture_timestamp: Some(nanos_to_rfc3339(
                result.first_capture_timestamp.as_nanos(),
            )),
            last_capture_timestamp: Some(nanos_to_rfc3339(
                result.last_capture_timestamp.as_nanos(),
            )),
            processed_at: Some(nanos_to_rfc3339(result.processed_at.as_nanos())),
            effective_sample_rate_hz: Some(result.sampling.effective_sample_rate_hz),
            timestamp_jitter: Some(result.sampling.timestamp_jitter),
            motion_energy_time_secs: Some(result.motion_energy.time_axis_secs.clone()),
            motion_energy_values: Some(motion_values),
            spectrum_frequencies_hz: frequencies,
            spectrum_power: power,
            dominant_non_dc_hz: dominant,
            processing_duration_ns: Some(result.processing_duration_ns),
            warnings: Some(result.warnings.clone()),
            dsp_profile_id: Some(result.dsp_profile_id.clone()),
            dsp_profile_version: Some(result.dsp_profile_version),
            motion_energy_semantics: Some(MOTION_ENERGY_SEMANTICS),
            spectrum_semantics: Some(SPECTRUM_SEMANTICS),
            timeline_semantics: Some(TIMELINE_SEMANTICS),
            data_classification: Some(PIPELINE_DATA_CLASSIFICATION),
        })
    }

    /// Builds a chronological recent-events snapshot from the signal store.
    pub async fn recent_events_snapshot(&self, limit: Option<usize>) -> RecentEventsResponse {
        let runtime = self.runtime.read().await;
        let samples_per_frame = runtime.context().config.synthetic_sensor.samples_per_frame;
        let limit = limit
            .unwrap_or(RECENT_EVENTS_DEFAULT_LIMIT)
            .min(RECENT_EVENTS_MAX_LIMIT);
        let events = runtime
            .signal_store()
            .recent_events(limit)
            .into_iter()
            .filter_map(|event| domain_event_to_envelope(event, samples_per_frame))
            .collect();
        RecentEventsResponse { events }
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

fn dsp_health_label(stats: &aeryon_runtime::DspStats) -> String {
    use aeryon_runtime::DspWorkerState;

    if !stats.enabled() {
        return "disabled".to_owned();
    }

    match stats.worker_state() {
        DspWorkerState::Disabled => "disabled".to_owned(),
        DspWorkerState::Idle => "idle".to_owned(),
        DspWorkerState::Completed => "completed".to_owned(),
        DspWorkerState::Stopped => "stopped".to_owned(),
        DspWorkerState::Failed => "failed".to_owned(),
        DspWorkerState::Running => {
            if stats.unexpected_exit() || stats.consecutive_failures() > 0 {
                "degraded".to_owned()
            } else {
                "running".to_owned()
            }
        }
    }
}
