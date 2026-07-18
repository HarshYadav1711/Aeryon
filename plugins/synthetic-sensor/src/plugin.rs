//! Synthetic sensor plugin implementing the platform [`Plugin`] contract.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aeryon_domain::{
    Event, Frame, FrameId, FrameMetadata, FrameReceived, Metadata, SensorFailed, SensorFailureKind,
    SensorId, SensorStarted, SensorStopped, Timestamp,
};
use aeryon_events::EventBus;
use aeryon_plugin_runtime::{
    Capability, HealthStatus, LifecycleError, Plugin, PluginError, PluginId, Version,
};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::config::SyntheticSensorConfig;
use crate::frame::SyntheticFrame;
use crate::signal::generate_samples;

/// Stable plugin identifier.
pub const PLUGIN_ID: &str = "aeryon.synthetic-sensor";

/// Stable numeric sensor identifier used in domain events.
pub const SENSOR_ID: SensorId = SensorId::new(1);

/// Deterministic synthetic sensor plugin.
///
/// Generates dual-sine numerical frames for integration testing. It is not a
/// real environmental perception sensor.
pub struct SyntheticSensorPlugin {
    id: PluginId,
    config: SyntheticSensorConfig,
    bus: EventBus,
    cancel_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
    frames_produced: Arc<AtomicU64>,
    last_sequence: Arc<AtomicU64>,
    producer_alive: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

impl SyntheticSensorPlugin {
    /// Creates a synthetic sensor plugin bound to `bus`.
    pub fn new(config: SyntheticSensorConfig, bus: EventBus) -> Self {
        Self {
            id: PluginId::new(PLUGIN_ID),
            config,
            bus,
            cancel_tx: None,
            task: None,
            frames_produced: Arc::new(AtomicU64::new(0)),
            last_sequence: Arc::new(AtomicU64::new(0)),
            producer_alive: Arc::new(AtomicBool::new(false)),
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns frames produced since the last start.
    pub fn frames_produced(&self) -> u64 {
        self.frames_produced.load(Ordering::Relaxed)
    }

    /// Returns the most recent sequence number, if any frames were produced.
    pub fn last_sequence(&self) -> Option<u64> {
        if self.frames_produced() == 0 {
            None
        } else {
            Some(self.last_sequence.load(Ordering::Relaxed))
        }
    }

    fn now() -> Timestamp {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
            .unwrap_or(0);
        Timestamp::from_nanos(nanos)
    }
}

impl Plugin for SyntheticSensorPlugin {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Synthetic Sensor"
    }

    fn version(&self) -> Version {
        Version::new(0, 1, 0)
    }

    fn description(&self) -> &str {
        "Deterministic dual-sine synthetic sensor for integration testing"
    }

    fn author(&self) -> &str {
        "Aeryon Contributors"
    }

    fn capabilities(&self) -> &[Capability] {
        &[Capability::Sensor]
    }

    fn initialize(&mut self) -> Result<(), PluginError> {
        if self.task.is_some() {
            return Err(PluginError::lifecycle(LifecycleError::InvalidTransition {
                plugin_id: self.id.clone(),
                from: aeryon_plugin_runtime::LifecycleState::Running,
                to: aeryon_plugin_runtime::LifecycleState::Initialized,
            }));
        }

        self.failed.store(false, Ordering::Relaxed);
        self.frames_produced.store(0, Ordering::Relaxed);
        self.last_sequence.store(0, Ordering::Relaxed);

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let bus = self.bus.clone();
        let config = self.config.clone();
        let frames_produced = Arc::clone(&self.frames_produced);
        let last_sequence = Arc::clone(&self.last_sequence);
        let producer_alive = Arc::clone(&self.producer_alive);
        let failed = Arc::clone(&self.failed);
        let plugin_id = self.id.clone();

        let started = Event::SensorStarted(SensorStarted {
            sensor_id: SENSOR_ID,
            timestamp: Self::now(),
        });
        if bus.publish(started).is_err() {
            tracing::warn!("synthetic sensor started with no event subscribers");
        }

        producer_alive.store(true, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            let mut sequence = 0_u64;
            let interval = Duration::from_millis(config.interval_ms);
            let mut cancel_rx = cancel_rx;

            loop {
                if *cancel_rx.borrow() {
                    break;
                }

                if let Some(maximum) = config.maximum_frames {
                    if sequence >= maximum {
                        break;
                    }
                }

                let samples = generate_samples(&config, sequence);
                let timestamp = SyntheticSensorPlugin::now();
                let frame = SyntheticFrame::new(
                    FrameMetadata {
                        frame_id: FrameId::new(sequence + 1),
                        sensor_id: SENSOR_ID,
                        timestamp,
                        sequence,
                        mission_id: None,
                        metadata: Metadata::new(),
                    },
                    samples,
                );

                let event = Event::FrameReceived(FrameReceived {
                    frame_id: frame.metadata().frame_id,
                    sensor_id: frame.metadata().sensor_id,
                    timestamp: frame.metadata().timestamp,
                    sequence: frame.metadata().sequence,
                });

                if bus.publish(event).is_err() {
                    failed.store(true, Ordering::Relaxed);
                    let _ = bus.publish(Event::SensorFailed(SensorFailed {
                        sensor_id: SENSOR_ID,
                        timestamp: SyntheticSensorPlugin::now(),
                        kind: SensorFailureKind::PublishFailed,
                    }));
                    tracing::error!(plugin = %plugin_id, "failed to publish synthetic frame");
                    break;
                }

                // Keep payload generation for determinism tests; events stay lightweight.
                let _ = frame.payload();

                frames_produced.fetch_add(1, Ordering::Relaxed);
                last_sequence.store(sequence, Ordering::Relaxed);

                tracing::debug!(
                    sequence,
                    frame_id = frame.metadata().frame_id.value(),
                    samples = frame.payload().len(),
                    "synthetic frame produced"
                );

                if sequence > 0 && sequence % config.log_every_n_frames == 0 {
                    tracing::info!(
                        sequence,
                        frames = frames_produced.load(Ordering::Relaxed),
                        "synthetic sensor frame progress"
                    );
                }

                sequence = sequence.saturating_add(1);

                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    changed = cancel_rx.changed() => {
                        if changed.is_err() || *cancel_rx.borrow() {
                            break;
                        }
                    }
                }
            }

            producer_alive.store(false, Ordering::Relaxed);

            let cancelled = *cancel_rx.borrow();
            let completed_maximum = config
                .maximum_frames
                .is_some_and(|maximum| sequence >= maximum);

            if cancelled || completed_maximum || failed.load(Ordering::Relaxed) {
                if completed_maximum {
                    tracing::info!(
                        maximum = config.maximum_frames,
                        "synthetic sensor reached maximum_frames"
                    );
                }
                return;
            }

            failed.store(true, Ordering::Relaxed);
            let _ = bus.publish(Event::SensorFailed(SensorFailed {
                sensor_id: SENSOR_ID,
                timestamp: SyntheticSensorPlugin::now(),
                kind: SensorFailureKind::ProducerExited,
            }));
            tracing::error!(plugin = %plugin_id, "synthetic sensor producer exited unexpectedly");
        });

        self.cancel_tx = Some(cancel_tx);
        self.task = Some(task);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), PluginError> {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(true);
        }

        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.producer_alive.store(false, Ordering::Relaxed);

        let stopped = Event::SensorStopped(SensorStopped {
            sensor_id: SENSOR_ID,
            timestamp: Self::now(),
        });
        if self.bus.publish(stopped).is_err() {
            tracing::debug!("synthetic sensor stopped with no event subscribers");
        }

        Ok(())
    }

    fn health(&self) -> HealthStatus {
        if self.failed.load(Ordering::Relaxed) {
            return HealthStatus::Unhealthy;
        }
        if self.task.is_some() && self.producer_alive.load(Ordering::Relaxed) {
            return HealthStatus::Healthy;
        }
        if self.task.is_some() {
            return HealthStatus::Degraded;
        }
        HealthStatus::Unhealthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_plugin_runtime::{LifecycleState, PluginRuntime};
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn metadata_and_capability_are_valid() {
        let plugin = SyntheticSensorPlugin::new(SyntheticSensorConfig::default(), EventBus::new());
        assert_eq!(plugin.id().as_str(), PLUGIN_ID);
        assert_eq!(plugin.capabilities(), &[Capability::Sensor]);
        assert!(!plugin.description().is_empty());
    }

    #[tokio::test]
    async fn lifecycle_start_and_stop_through_plugin_runtime() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = SyntheticSensorConfig {
            interval_ms: 20,
            maximum_frames: Some(5),
            ..SyntheticSensorConfig::default()
        };

        runtime
            .register(Box::new(SyntheticSensorPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Running));

        let started = timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("timeout")
            .expect("recv");
        assert!(matches!(started, Event::SensorStarted(_)));

        let mut frames = 0_u32;
        while frames < 3 {
            let event = timeout(Duration::from_secs(2), receiver.recv())
                .await
                .expect("timeout")
                .expect("recv");
            if matches!(event, Event::FrameReceived(_)) {
                frames += 1;
            }
        }

        runtime.stop(&id).expect("stop");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Stopped));
        assert_eq!(
            runtime.health(&id).expect("health"),
            HealthStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn duplicate_start_is_idempotent_at_runtime_level() {
        let bus = EventBus::new();
        let _receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = SyntheticSensorConfig {
            interval_ms: 50,
            maximum_frames: Some(2),
            ..SyntheticSensorConfig::default()
        };

        runtime
            .register(Box::new(SyntheticSensorPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");
        runtime.start(&id).expect("second start is idempotent");
        runtime.stop(&id).expect("stop");
    }

    #[tokio::test]
    async fn maximum_frames_is_respected() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = SyntheticSensorConfig {
            interval_ms: 10,
            maximum_frames: Some(3),
            ..SyntheticSensorConfig::default()
        };

        runtime
            .register(Box::new(SyntheticSensorPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");

        let mut sequences = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while sequences.len() < 3 && tokio::time::Instant::now() < deadline {
            if let Ok(Ok(Event::FrameReceived(frame))) =
                timeout(Duration::from_millis(500), receiver.recv()).await
            {
                sequences.push(frame.sequence);
            }
        }

        assert_eq!(sequences, vec![0, 1, 2]);
        runtime.stop(&id).expect("stop");
    }
}
