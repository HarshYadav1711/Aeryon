//! Application runtime lifecycle management.

use std::sync::Arc;
use std::time::Duration;

use aeryon_calibration::CalibrationPipeline;
use aeryon_csi_replay::{CsiReplayPlugin, PLUGIN_ID as CSI_REPLAY_PLUGIN_ID};
use aeryon_domain::Event;
use aeryon_dsp::{DspService, DspWorkerState};
use aeryon_events::EventBus;
use aeryon_plugin_runtime::{LifecycleState, PluginId, PluginRuntime};
use aeryon_synthetic_sensor::{PLUGIN_ID as SYNTHETIC_PLUGIN_ID, SyntheticSensorPlugin};
use tokio::task::JoinHandle;

use crate::calibration_service::CalibrationService;
use crate::calibration_stats::CalibrationWorkerState;
use crate::config::AppConfig;
use crate::context::AppContext;
use crate::error::{LoggingError, RuntimeError};
use crate::health::RuntimeHealth;
use crate::logging::init_logging;
use crate::metrics::RuntimeMetrics;
use crate::signal_store::SignalSnapshotStore;

/// Coordinates application startup, shutdown, and health reporting.
pub struct Runtime {
    context: AppContext,
    health: RuntimeHealth,
    consumer_task: Option<JoinHandle<()>>,
    calibration_service: Option<CalibrationService>,
    dsp_service: Option<DspService>,
}

impl Runtime {
    /// Boots the runtime using `config`.
    ///
    /// Initializes logging, the event bus, and the plugin runtime. The runtime
    /// remains in the `Starting` state until [`start`](Self::start) is called.
    pub fn boot(config: AppConfig) -> Result<Self, RuntimeError> {
        if let Err(error) = init_logging(&config.logging) {
            if error != LoggingError::AlreadyInitialized {
                return Err(RuntimeError::Logging(error));
            }
        }

        tracing::info!("startup");
        tracing::info!(environment = %config.application.environment, "configuration loaded");

        let event_bus = EventBus::new();
        let metrics = RuntimeMetrics::new().shared();
        let signal_store = SignalSnapshotStore::default().shared();
        let plugin_runtime = PluginRuntime::new();

        tracing::info!(
            enabled = config.plugins.enabled,
            autoload = config.plugins.autoload,
            "plugin runtime initialized"
        );

        let context = AppContext::new(
            config,
            plugin_runtime,
            event_bus,
            metrics,
            signal_store,
            env!("CARGO_PKG_VERSION"),
        );

        Ok(Self {
            context,
            health: RuntimeHealth::Starting,
            consumer_task: None,
            calibration_service: None,
            dsp_service: None,
        })
    }

    /// Transitions the runtime to the `Running` state.
    ///
    /// Registers and starts the configured sensor plugin through the plugin
    /// runtime and begins consuming typed events.
    pub fn start(&mut self) -> Result<(), RuntimeError> {
        self.require_health(RuntimeHealth::Starting)?;

        if tokio::runtime::Handle::try_current().is_err() {
            return Err(RuntimeError::MissingTokioRuntime);
        }

        self.start_event_consumer();

        // Configure calibration stats for API visibility even when disabled.
        if self.context.config.calibration.enabled {
            if let Ok(profile) = self.context.config.calibration.resolve_profile() {
                self.context.metrics.calibration().configure(
                    true,
                    Some(&profile.id),
                    profile.version,
                );
            }
        } else {
            self.context.metrics.calibration().configure(false, None, 0);
            self.context
                .metrics
                .calibration()
                .set_worker_state(CalibrationWorkerState::Disabled);
        }

        // Configure DSP stats for API visibility even when disabled.
        if self.context.config.dsp.enabled {
            if let Ok(profile) = self.context.config.dsp.resolve_profile() {
                self.context.metrics.dsp().configure(
                    true,
                    Some(&profile.id),
                    profile.version,
                    self.context.config.dsp.window_size_frames,
                    self.context.config.dsp.hop_size_frames,
                );
            }
        } else {
            self.context.metrics.dsp().configure(
                false,
                None,
                0,
                self.context.config.dsp.window_size_frames,
                self.context.config.dsp.hop_size_frames,
            );
            self.context
                .metrics
                .dsp()
                .set_worker_state(DspWorkerState::Disabled);
        }

        if self.context.config.plugins.enabled {
            if self.context.config.synthetic_sensor.enabled {
                self.start_synthetic_sensor()?;
            }
            if self.context.config.sensors.csi_replay.enabled {
                self.start_csi_replay()?;
            }
        }

        tracing::info!("runtime entering running state");
        self.health = RuntimeHealth::Running;
        self.refresh_health();
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
                .filter(|(_, state)| {
                    matches!(state, LifecycleState::Running | LifecycleState::Initialized)
                })
                .map(|(id, _)| id)
                .collect();

            for plugin_id in running_plugins {
                self.context.plugin_runtime.stop(&plugin_id)?;
                if plugin_id.as_str() == SYNTHETIC_PLUGIN_ID {
                    self.context
                        .metrics
                        .set_sensor_lifecycle(LifecycleState::Stopped);
                }
                if plugin_id.as_str() == CSI_REPLAY_PLUGIN_ID {
                    self.context
                        .metrics
                        .set_csi_lifecycle(LifecycleState::Stopped);
                }
            }
        }

        if let Some(mut service) = self.dsp_service.take() {
            service.shutdown();
            self.context
                .metrics
                .dsp()
                .set_worker_state(DspWorkerState::Stopped);
        }

        if let Some(mut service) = self.calibration_service.take() {
            service.shutdown();
            self.context
                .metrics
                .calibration()
                .set_worker_state(CalibrationWorkerState::Stopped);
        }

        if let Some(task) = self.consumer_task.take() {
            task.abort();
        }
        self.context.metrics.set_consumer_running(false);

        self.health = RuntimeHealth::Stopped;
        tracing::info!("runtime stopped");
        Ok(())
    }

    /// Returns the current runtime health state.
    pub fn health(&self) -> RuntimeHealth {
        self.health
    }

    /// Recomputes health from metrics and sensor state.
    pub fn refresh_health(&mut self) {
        if matches!(
            self.health,
            RuntimeHealth::Starting | RuntimeHealth::Stopping | RuntimeHealth::Stopped
        ) {
            return;
        }

        let timeout = Duration::from_millis(self.context.config.runtime.first_frame_timeout_ms);
        let evaluated = self.context.metrics.evaluate_health(
            self.context.config.synthetic_sensor.enabled,
            self.context.config.sensors.csi_replay.enabled,
            timeout,
        );

        if evaluated == RuntimeHealth::Failed {
            self.health = RuntimeHealth::Failed;
        } else if evaluated == RuntimeHealth::Degraded {
            self.health = RuntimeHealth::Degraded;
        } else if self.health != RuntimeHealth::Failed {
            self.health = RuntimeHealth::Running;
        }
    }

    /// Returns the application context.
    pub fn context(&self) -> &AppContext {
        &self.context
    }

    /// Returns shared runtime metrics.
    pub fn metrics(&self) -> &Arc<RuntimeMetrics> {
        &self.context.metrics
    }

    /// Returns the bounded signal snapshot store.
    pub fn signal_store(&self) -> &Arc<SignalSnapshotStore> {
        &self.context.signal_store
    }

    /// Returns a concise startup summary for operator output.
    pub fn startup_summary(&self) -> String {
        format!(
            "Aeryon {} | environment={} | plugins={} | synthetic={} | csi_replay={} | calibration={} | dsp={} | status={}",
            self.context.version,
            self.context.config.application.environment,
            if self.context.config.plugins.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.context.config.synthetic_sensor.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.context.config.sensors.csi_replay.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.context.config.calibration.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.context.config.dsp.enabled {
                "enabled"
            } else {
                "disabled"
            },
            self.health
        )
    }

    fn start_event_consumer(&mut self) {
        let mut receiver = self.context.event_bus.subscribe();
        let metrics = Arc::clone(&self.context.metrics);
        let signal_store = Arc::clone(&self.context.signal_store);
        let log_every = self
            .context
            .config
            .synthetic_sensor
            .log_every_n_frames
            .max(1);

        metrics.set_consumer_running(true);

        self.consumer_task = Some(tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        signal_store.push_event(event.clone());
                        match event {
                            Event::FrameReceived(frame) => {
                                metrics.record_frame(frame.sequence, frame.timestamp.as_nanos());
                                tracing::debug!(
                                    sequence = frame.sequence,
                                    frame_id = frame.frame_id.value(),
                                    sensor_id = frame.sensor_id.value(),
                                    "frame event received"
                                );
                                let count = metrics.frames_received();
                                if count > 0 && count % log_every == 0 {
                                    tracing::info!(
                                        frames = count,
                                        last_sequence = frame.sequence,
                                        "frame event progress"
                                    );
                                }
                            }
                            Event::CsiFrameReceived(frame) => {
                                metrics.record_frame(
                                    frame.sequence,
                                    frame.capture_timestamp.as_nanos(),
                                );
                                tracing::debug!(
                                    sequence = frame.sequence,
                                    frame_id = frame.frame_id.value(),
                                    sensor_id = frame.sensor_id.value(),
                                    source = frame.source.as_str(),
                                    "CSI frame event received"
                                );
                            }
                            Event::SensorStarted(event) => {
                                tracing::info!(
                                    sensor_id = event.sensor_id.value(),
                                    "sensor started event"
                                );
                            }
                            Event::SensorStopped(event) => {
                                tracing::info!(
                                    sensor_id = event.sensor_id.value(),
                                    "sensor stopped event"
                                );
                            }
                            Event::SensorFailed(event) => {
                                tracing::error!(
                                    sensor_id = event.sensor_id.value(),
                                    kind = ?event.kind,
                                    "sensor failed event"
                                );
                                metrics.set_sensor_lifecycle(LifecycleState::Failed);
                            }
                            Event::CsiReplayStarted(event) => {
                                tracing::info!(
                                    sensor_id = event.sensor_id.value(),
                                    "CSI replay started event"
                                );
                            }
                            Event::CsiReplayCompleted(event) => {
                                tracing::info!(
                                    sensor_id = event.sensor_id.value(),
                                    frames = event.frames_accepted,
                                    "CSI replay completed event"
                                );
                            }
                            Event::CsiReplayStopped(event) => {
                                tracing::info!(
                                    sensor_id = event.sensor_id.value(),
                                    "CSI replay stopped event"
                                );
                            }
                            Event::CsiReplayFailed(event) => {
                                tracing::error!(
                                    sensor_id = event.sensor_id.value(),
                                    kind = ?event.kind,
                                    "CSI replay failed event"
                                );
                                metrics.set_csi_lifecycle(LifecycleState::Failed);
                            }
                            Event::CalibrationStarted(event) => {
                                tracing::info!(
                                    profile = %event.profile_id,
                                    version = event.profile_version,
                                    "calibration started event"
                                );
                            }
                            Event::CsiFrameCalibrated(event) => {
                                tracing::debug!(
                                    sequence = event.sequence,
                                    frame_id = event.raw_frame_id.value(),
                                    duration_ns = event.calibration_duration_ns,
                                    "CSI frame calibrated event"
                                );
                            }
                            Event::CalibrationFailed(event) => {
                                tracing::warn!(
                                    sequence = ?event.sequence,
                                    code = event.code.as_str(),
                                    "calibration failed event"
                                );
                            }
                            Event::CalibrationServiceStopped(_) => {
                                tracing::info!("calibration service stopped event");
                            }
                            Event::DspServiceStarted(event) => {
                                tracing::info!(
                                    profile = %event.profile_id,
                                    version = event.profile_version,
                                    "DSP service started event"
                                );
                            }
                            Event::CsiWindowAssembled(event) => {
                                tracing::debug!(
                                    window_id = event.window_id,
                                    first = event.first_sequence,
                                    last = event.last_sequence,
                                    "CSI window assembled event"
                                );
                            }
                            Event::DspWindowProcessed(event) => {
                                tracing::debug!(
                                    window_id = event.window_id,
                                    first = event.first_sequence,
                                    last = event.last_sequence,
                                    duration_ns = event.processing_duration_ns,
                                    "DSP window processed event"
                                );
                            }
                            Event::DspProcessingFailed(event) => {
                                tracing::warn!(
                                    code = event.code.as_str(),
                                    "DSP processing failed event"
                                );
                            }
                            Event::DspServiceIdle(event) => {
                                tracing::info!(
                                    completed = event.completed,
                                    "DSP service idle/completed event"
                                );
                            }
                            Event::DspServiceStopped(_) => {
                                tracing::info!("DSP service stopped event");
                            }
                            _ => {}
                        }
                    }
                    Err(aeryon_events::BusError::Closed) => break,
                    Err(aeryon_events::BusError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "event consumer lagged");
                    }
                    Err(aeryon_events::BusError::NoSubscribers) => {}
                }
            }
            metrics.set_consumer_running(false);
        }));
    }

    fn start_synthetic_sensor(&mut self) -> Result<(), RuntimeError> {
        let plugin = SyntheticSensorPlugin::new(
            self.context.config.synthetic_sensor.clone(),
            self.context.event_bus.clone(),
        );
        let plugin_id = PluginId::new(SYNTHETIC_PLUGIN_ID);

        self.context.plugin_runtime.register(Box::new(plugin))?;
        self.context
            .metrics
            .set_sensor_lifecycle(LifecycleState::Registered);

        match self.context.plugin_runtime.start(&plugin_id) {
            Ok(()) => {
                self.context
                    .metrics
                    .set_sensor_lifecycle(LifecycleState::Running);
                tracing::info!(
                    plugin = SYNTHETIC_PLUGIN_ID,
                    "synthetic sensor plugin started"
                );
                Ok(())
            }
            Err(error) => {
                self.context
                    .metrics
                    .set_sensor_lifecycle(LifecycleState::Failed);
                self.health = RuntimeHealth::Failed;
                Err(error.into())
            }
        }
    }

    fn start_csi_replay(&mut self) -> Result<(), RuntimeError> {
        let mut frame_tx = None;
        let mut calibrated_tx = None;

        if self.context.config.dsp.enabled {
            let profile = self
                .context
                .config
                .dsp
                .resolve_profile()
                .map_err(crate::error::ConfigError::Dsp)?;
            let mut service = DspService::start(
                self.context.event_bus.clone(),
                self.context.config.dsp.clone(),
                profile,
                Arc::clone(self.context.metrics.dsp()),
                Some(Arc::clone(&self.context.signal_store) as Arc<dyn aeryon_dsp::DspResultSink>),
            )
            .map_err(crate::error::ConfigError::Dsp)?;
            calibrated_tx = service.take_frame_tx();
            self.dsp_service = Some(service);
            tracing::info!(
                profile = %self.context.config.dsp.profile,
                "DSP service started"
            );
        }

        if self.context.config.calibration.enabled {
            let profile = self
                .context
                .config
                .calibration
                .resolve_profile()
                .map_err(crate::error::ConfigError::Calibration)?;
            let pipeline = CalibrationPipeline::try_new(profile)
                .map_err(crate::error::ConfigError::Calibration)?;
            let mut service = CalibrationService::start(
                self.context.event_bus.clone(),
                pipeline,
                Arc::clone(self.context.metrics.calibration()),
                self.context.config.calibration.queue_capacity,
                calibrated_tx,
                Some(Arc::clone(&self.context.signal_store)),
            )?;
            frame_tx = service.take_frame_tx();
            self.calibration_service = Some(service);
            tracing::info!(
                profile = %self.context.config.calibration.profile,
                "calibration service started"
            );
        }

        let plugin = CsiReplayPlugin::with_stats_and_frame_tx(
            self.context.config.sensors.csi_replay.clone(),
            self.context.event_bus.clone(),
            Arc::clone(self.context.metrics.csi_replay()),
            frame_tx,
        );
        let plugin_id = PluginId::new(CSI_REPLAY_PLUGIN_ID);

        self.context.plugin_runtime.register(Box::new(plugin))?;
        self.context
            .metrics
            .set_csi_lifecycle(LifecycleState::Registered);

        match self.context.plugin_runtime.start(&plugin_id) {
            Ok(()) => {
                self.context
                    .metrics
                    .set_csi_lifecycle(LifecycleState::Running);
                tracing::info!(plugin = CSI_REPLAY_PLUGIN_ID, "CSI replay plugin started");
                Ok(())
            }
            Err(error) => {
                if let Some(mut service) = self.dsp_service.take() {
                    service.shutdown();
                }
                if let Some(mut service) = self.calibration_service.take() {
                    service.shutdown();
                }
                self.context
                    .metrics
                    .set_csi_lifecycle(LifecycleState::Failed);
                self.health = RuntimeHealth::Failed;
                Err(error.into())
            }
        }
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
    use aeryon_domain::Event;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::time::{Duration, timeout};

    fn test_config() -> AppConfig {
        AppConfig::from_toml(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "info"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10
            first_frame_timeout_ms = 2000

            [synthetic_sensor]
            enabled = true
            interval_ms = 20
            samples_per_frame = 64
            sample_rate_hz = 1000.0
            primary_frequency_hz = 10.0
            secondary_frequency_hz = 37.0
            secondary_amplitude = 0.25
            maximum_frames = 8
            log_every_n_frames = 2
            "#,
        )
        .expect("valid test config")
    }

    fn csi_test_config(path: &str) -> AppConfig {
        AppConfig::from_toml(&format!(
            r#"
            [application]
            name = "aeryon"
            environment = "development"

            [logging]
            level = "error"

            [plugins]
            enabled = true
            autoload = false

            [runtime]
            shutdown_timeout_secs = 10
            first_frame_timeout_ms = 2000

            [synthetic_sensor]
            enabled = false

            [sensors.csi_replay]
            enabled = true
            path = "{path}"
            loop_playback = false
            frame_interval_ms = 10
            maximum_frames = 12

            [calibration]
            enabled = true
            profile = "baseline-csi-v1"
            queue_capacity = 64

            [dsp]
            enabled = true
            profile = "baseline-dsp-v1"
            queue_capacity = 64
            window_size_frames = 8
            hop_size_frames = 4
            maximum_sequence_gap = 1
            timestamp_jitter_tolerance = 0.10
            "#
        ))
        .expect("valid csi config")
    }

    fn write_csi_fixture() -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temp");
        writeln!(
            file,
            r#"{{"record_type":"header","schema":"aeryon-csi-fixture","version":1,"sensor_id":"2","description":"runtime test","sample_layout":"rx-tx-subcarrier"}}"#
        )
        .expect("header");
        for sequence in 0..16 {
            writeln!(
                file,
                r#"{{"record_type":"frame","frame_id":{},"sequence":{},"capture_timestamp_nanos":{},"center_frequency_hz":5180000000.0,"bandwidth_hz":20000000.0,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[0,1],"samples":[{{"re":{},"im":0.0}},{{"re":0.0,"im":1.0}},{{"re":2.0,"im":0.0}},{{"re":0.0,"im":2.0}}]}}"#,
                sequence + 1,
                sequence,
                1_700_000_000_000_000_000u64 + sequence * 100_000_000,
                1.0 + (sequence as f64) * 0.05
            )
            .expect("frame");
        }
        file
    }

    #[tokio::test]
    async fn boot_initializes_in_starting_state() {
        let runtime = Runtime::boot(AppConfig::default()).expect("boot succeeds");
        assert_eq!(runtime.health(), RuntimeHealth::Starting);
        assert!(runtime.context().config.plugins.enabled);
    }

    #[tokio::test]
    async fn start_and_shutdown_lifecycle() {
        let mut runtime = Runtime::boot(test_config()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        assert!(matches!(
            runtime.health(),
            RuntimeHealth::Running | RuntimeHealth::Degraded
        ));
        runtime.shutdown().expect("shutdown succeeds");
        assert_eq!(runtime.health(), RuntimeHealth::Stopped);
    }

    #[tokio::test]
    async fn start_rejects_invalid_transition() {
        let mut runtime = Runtime::boot(test_config()).expect("boot succeeds");
        runtime.start().expect("first start succeeds");
        let error = runtime.start().expect_err("second start fails");
        match error {
            RuntimeError::InvalidState {
                expected: RuntimeHealth::Starting,
                actual,
            } => {
                assert!(matches!(
                    actual,
                    RuntimeHealth::Running | RuntimeHealth::Degraded
                ));
            }
            other => panic!("unexpected error: {other}"),
        }
        runtime.shutdown().expect("shutdown");
    }

    #[tokio::test]
    async fn integration_receives_deterministic_frames() {
        let mut runtime = Runtime::boot(test_config()).expect("boot");
        let mut receiver = runtime.context().event_bus.subscribe();
        runtime.start().expect("start");

        let mut sequences = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while sequences.len() < 3 && tokio::time::Instant::now() < deadline {
            match timeout(Duration::from_millis(500), receiver.recv()).await {
                Ok(Ok(Event::FrameReceived(frame))) => sequences.push(frame.sequence),
                Ok(Ok(_)) => {}
                _ => break,
            }
        }

        assert!(
            sequences.len() >= 3,
            "expected at least 3 frames, got {sequences:?}"
        );
        assert!(sequences.windows(2).all(|pair| pair[1] == pair[0] + 1));
        assert!(runtime.metrics().frames_received() >= 3);
        assert!(runtime.metrics().last_sequence().is_some());
        assert!(runtime.metrics().consumer_running());

        runtime.shutdown().expect("shutdown");
        assert!(!runtime.metrics().consumer_running());
        assert_eq!(
            runtime
                .context()
                .plugin_runtime
                .lifecycle_state(&PluginId::new(SYNTHETIC_PLUGIN_ID)),
            Some(LifecycleState::Stopped)
        );
    }

    #[tokio::test]
    async fn csi_replay_integration_receives_ordered_frames() {
        let fixture = write_csi_fixture();
        let path = fixture.path().to_string_lossy().replace('\\', "/");
        let mut runtime = Runtime::boot(csi_test_config(&path)).expect("boot");
        let mut receiver = runtime.context().event_bus.subscribe();
        runtime.start().expect("start");

        let mut sequences = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while sequences.len() < 3 && tokio::time::Instant::now() < deadline {
            match timeout(Duration::from_millis(500), receiver.recv()).await {
                Ok(Ok(Event::CsiFrameReceived(frame))) => {
                    assert_eq!(frame.receive_antennas, 2);
                    assert_eq!(frame.transmit_antennas, 1);
                    assert_eq!(frame.subcarrier_count, 2);
                    sequences.push(frame.sequence);
                }
                Ok(Ok(_)) => {}
                _ => break,
            }
        }

        assert!(
            sequences.len() >= 3,
            "expected at least 3 CSI frames, got {sequences:?}"
        );
        assert!(sequences.windows(2).all(|pair| pair[1] == pair[0] + 1));
        assert!(runtime.metrics().frames_received() >= 3);
        assert!(runtime.metrics().csi_replay().frames_accepted() >= 3);

        runtime.shutdown().expect("shutdown");
        assert!(!runtime.metrics().consumer_running());
        assert_eq!(
            runtime
                .context()
                .plugin_runtime
                .lifecycle_state(&PluginId::new(CSI_REPLAY_PLUGIN_ID)),
            Some(LifecycleState::Stopped)
        );
    }

    #[tokio::test]
    async fn calibration_end_to_end_with_csi_replay() {
        let fixture = write_csi_fixture();
        let path = fixture.path().to_string_lossy().replace('\\', "/");
        let mut runtime = Runtime::boot(csi_test_config(&path)).expect("boot");
        let mut receiver = runtime.context().event_bus.subscribe();
        runtime.start().expect("start");

        let mut raw_sequences = Vec::new();
        let mut calibrated = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while (raw_sequences.len() < 3 || calibrated.len() < 3)
            && tokio::time::Instant::now() < deadline
        {
            match timeout(Duration::from_millis(500), receiver.recv()).await {
                Ok(Ok(Event::CsiFrameReceived(frame))) => raw_sequences.push(frame.sequence),
                Ok(Ok(Event::CsiFrameCalibrated(event))) => {
                    assert_eq!(event.profile_id, "baseline-csi-v1");
                    assert_eq!(event.profile_version, 1);
                    assert_eq!(event.source.as_str(), "csi_replay");
                    assert!(event.stage_count >= 1);
                    calibrated.push((event.sequence, event.raw_frame_id.value()));
                }
                Ok(Ok(_)) => {}
                _ => break,
            }
        }

        assert!(
            raw_sequences.len() >= 3,
            "expected >=3 raw frames, got {raw_sequences:?}"
        );
        assert!(
            calibrated.len() >= 3,
            "expected >=3 calibrated frames, got {calibrated:?}"
        );
        assert!(calibrated.windows(2).all(|pair| pair[1].0 == pair[0].0 + 1));
        assert!(
            runtime.metrics().calibration().frames_calibrated() >= 3,
            "expected calibration success counter >= 3"
        );
        assert!(runtime.metrics().calibration().raw_frames_submitted() >= 3);
        assert_eq!(
            runtime.metrics().calibration().profile_id().as_deref(),
            Some("baseline-csi-v1")
        );

        runtime.shutdown().expect("shutdown");
        assert_eq!(
            runtime.metrics().calibration().worker_state(),
            CalibrationWorkerState::Stopped
        );
        assert_eq!(
            runtime
                .context()
                .plugin_runtime
                .lifecycle_state(&PluginId::new(CSI_REPLAY_PLUGIN_ID)),
            Some(LifecycleState::Stopped)
        );
    }

    #[tokio::test]
    async fn dsp_end_to_end_with_calibration_and_csi_replay() {
        let fixture = write_csi_fixture();
        let path = fixture.path().to_string_lossy().replace('\\', "/");
        let mut runtime = Runtime::boot(csi_test_config(&path)).expect("boot");
        let mut receiver = runtime.context().event_bus.subscribe();
        runtime.start().expect("start");

        let mut processed = Vec::new();
        let mut completed_replay = false;
        let mut dsp_idle = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while tokio::time::Instant::now() < deadline {
            match timeout(Duration::from_millis(500), receiver.recv()).await {
                Ok(Ok(Event::DspWindowProcessed(event))) => {
                    assert_eq!(event.profile_id, "baseline-dsp-v1");
                    assert!(event.frame_count >= 8);
                    assert!(event.effective_sample_rate_hz.is_finite());
                    assert!(event.effective_sample_rate_hz > 0.0);
                    assert!(event.timestamp_jitter.is_finite());
                    processed.push((event.first_sequence, event.last_sequence));
                }
                Ok(Ok(Event::CsiReplayCompleted(_))) => completed_replay = true,
                Ok(Ok(Event::DspServiceIdle(event))) => {
                    assert!(event.completed);
                    dsp_idle = true;
                }
                Ok(Ok(_)) => {}
                _ => {
                    if completed_replay && dsp_idle && !processed.is_empty() {
                        break;
                    }
                }
            }
            if completed_replay && dsp_idle && !processed.is_empty() {
                break;
            }
        }

        assert!(
            !processed.is_empty(),
            "expected at least one DSP window, got {processed:?}"
        );
        assert!(
            processed.windows(2).all(|pair| pair[1].0 >= pair[0].0),
            "window order not preserved: {processed:?}"
        );
        assert!(runtime.metrics().dsp().windows_emitted() >= 1);
        assert!(runtime.metrics().dsp().calibrated_frames_received() >= 8);

        let result = runtime
            .signal_store()
            .latest_dsp()
            .expect("latest DSP result stored");
        assert!(
            result
                .motion_energy
                .signal
                .links
                .iter()
                .flat_map(|link| link.values.iter())
                .all(|value| value.is_finite())
        );
        assert!(
            result
                .spectra
                .links
                .iter()
                .flat_map(|link| link.power.iter())
                .all(|value| value.is_finite())
        );
        assert!(runtime.signal_store().latest_calibrated().is_some());
        assert!(runtime.signal_store().latest_raw().is_some());
        assert!(!runtime.signal_store().recent_events(50).is_empty());

        // Allow workers to observe channel EOF after finite replay.
        timeout(Duration::from_secs(2), async {
            loop {
                let dsp_state = runtime.metrics().dsp().worker_state();
                if matches!(dsp_state, DspWorkerState::Completed | DspWorkerState::Idle) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("dsp completion");

        assert!(
            matches!(
                runtime.metrics().dsp().worker_state(),
                DspWorkerState::Completed | DspWorkerState::Idle
            ),
            "DSP should be completed/idle after finite replay, got {:?}",
            runtime.metrics().dsp().worker_state()
        );
        assert_ne!(
            runtime.metrics().dsp().worker_state(),
            DspWorkerState::Failed
        );

        runtime.shutdown().expect("shutdown");
        assert!(!runtime.metrics().consumer_running());
        assert_eq!(
            runtime
                .context()
                .plugin_runtime
                .lifecycle_state(&PluginId::new(CSI_REPLAY_PLUGIN_ID)),
            Some(LifecycleState::Stopped)
        );
    }

    #[tokio::test]
    async fn startup_summary_contains_version_and_status() {
        let mut runtime = Runtime::boot(test_config()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        let summary = runtime.startup_summary();
        assert!(summary.contains("Aeryon"));
        assert!(summary.contains("synthetic=enabled"));
        runtime.shutdown().expect("shutdown");
    }
}
