//! Application runtime lifecycle management.

use std::sync::Arc;
use std::time::Duration;

use aeryon_csi_replay::{CsiReplayPlugin, PLUGIN_ID as CSI_REPLAY_PLUGIN_ID};
use aeryon_domain::Event;
use aeryon_events::EventBus;
use aeryon_plugin_runtime::{LifecycleState, PluginId, PluginRuntime};
use aeryon_synthetic_sensor::{PLUGIN_ID as SYNTHETIC_PLUGIN_ID, SyntheticSensorPlugin};
use tokio::task::JoinHandle;

use crate::config::AppConfig;
use crate::context::AppContext;
use crate::error::{LoggingError, RuntimeError};
use crate::health::RuntimeHealth;
use crate::logging::init_logging;
use crate::metrics::RuntimeMetrics;

/// Coordinates application startup, shutdown, and health reporting.
pub struct Runtime {
    context: AppContext,
    health: RuntimeHealth,
    consumer_task: Option<JoinHandle<()>>,
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
            env!("CARGO_PKG_VERSION"),
        );

        Ok(Self {
            context,
            health: RuntimeHealth::Starting,
            consumer_task: None,
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

    /// Returns a concise startup summary for operator output.
    pub fn startup_summary(&self) -> String {
        format!(
            "Aeryon {} | environment={} | plugins={} | synthetic={} | csi_replay={} | status={}",
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
            self.health
        )
    }

    fn start_event_consumer(&mut self) {
        let mut receiver = self.context.event_bus.subscribe();
        let metrics = Arc::clone(&self.context.metrics);
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
                    Ok(Event::FrameReceived(frame)) => {
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
                    Ok(Event::CsiFrameReceived(frame)) => {
                        metrics.record_frame(frame.sequence, frame.capture_timestamp.as_nanos());
                        tracing::debug!(
                            sequence = frame.sequence,
                            frame_id = frame.frame_id.value(),
                            sensor_id = frame.sensor_id.value(),
                            source = frame.source.as_str(),
                            "CSI frame event received"
                        );
                    }
                    Ok(Event::SensorStarted(event)) => {
                        tracing::info!(sensor_id = event.sensor_id.value(), "sensor started event");
                    }
                    Ok(Event::SensorStopped(event)) => {
                        tracing::info!(sensor_id = event.sensor_id.value(), "sensor stopped event");
                    }
                    Ok(Event::SensorFailed(event)) => {
                        tracing::error!(
                            sensor_id = event.sensor_id.value(),
                            kind = ?event.kind,
                            "sensor failed event"
                        );
                        metrics.set_sensor_lifecycle(LifecycleState::Failed);
                    }
                    Ok(Event::CsiReplayStarted(event)) => {
                        tracing::info!(
                            sensor_id = event.sensor_id.value(),
                            "CSI replay started event"
                        );
                    }
                    Ok(Event::CsiReplayCompleted(event)) => {
                        tracing::info!(
                            sensor_id = event.sensor_id.value(),
                            frames = event.frames_accepted,
                            "CSI replay completed event"
                        );
                    }
                    Ok(Event::CsiReplayStopped(event)) => {
                        tracing::info!(
                            sensor_id = event.sensor_id.value(),
                            "CSI replay stopped event"
                        );
                    }
                    Ok(Event::CsiReplayFailed(event)) => {
                        tracing::error!(
                            sensor_id = event.sensor_id.value(),
                            kind = ?event.kind,
                            "CSI replay failed event"
                        );
                        metrics.set_csi_lifecycle(LifecycleState::Failed);
                    }
                    Ok(_) => {}
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
        let plugin = CsiReplayPlugin::with_stats(
            self.context.config.sensors.csi_replay.clone(),
            self.context.event_bus.clone(),
            Arc::clone(self.context.metrics.csi_replay()),
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
            maximum_frames = 5
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
        for sequence in 0..8 {
            writeln!(
                file,
                r#"{{"record_type":"frame","frame_id":{},"sequence":{},"capture_timestamp_nanos":{},"center_frequency_hz":5180000000.0,"bandwidth_hz":20000000.0,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[0,1],"samples":[{{"re":1.0,"im":0.0}},{{"re":0.0,"im":1.0}},{{"re":2.0,"im":0.0}},{{"re":0.0,"im":2.0}}]}}"#,
                sequence + 1,
                sequence,
                1_000 + sequence
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
    async fn startup_summary_contains_version_and_status() {
        let mut runtime = Runtime::boot(test_config()).expect("boot succeeds");
        runtime.start().expect("start succeeds");
        let summary = runtime.startup_summary();
        assert!(summary.contains("Aeryon"));
        assert!(summary.contains("synthetic=enabled"));
        runtime.shutdown().expect("shutdown");
    }
}
