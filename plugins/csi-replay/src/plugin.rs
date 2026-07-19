//! CSI fixture replay plugin implementing the platform [`Plugin`] contract.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aeryon_csi::{CsiFrame, FixtureReader};
use aeryon_domain::{
    CsiDataSource, CsiFrameReceived, CsiReplayCompleted, CsiReplayFailed, CsiReplayFailureKind,
    CsiReplayStarted, CsiReplayStopped, Event, SensorId, Timestamp,
};
use aeryon_events::EventBus;
use aeryon_plugin_runtime::{
    Capability, HealthStatus, LifecycleError, LifecycleState, Plugin, PluginError, PluginId,
    Version,
};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::config::CsiReplayConfig;
use crate::stats::{CsiReplayCompletion, CsiReplayStats};

/// Stable plugin identifier.
pub const PLUGIN_ID: &str = "aeryon.csi-replay";

/// Stable numeric sensor identifier used in domain events.
pub const SENSOR_ID: SensorId = SensorId::new(2);

/// Stable source marker for CSI replay development data.
pub const SOURCE_ID: &str = "csi_replay";

/// Deterministic CSI fixture replay sensor plugin.
///
/// Emits canonical CSI frames from a versioned development fixture. This is not
/// live WiFi/RF sensing hardware.
pub struct CsiReplayPlugin {
    id: PluginId,
    config: CsiReplayConfig,
    bus: EventBus,
    stats: Arc<CsiReplayStats>,
    cancel_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
    producer_alive: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

impl CsiReplayPlugin {
    /// Creates a CSI replay plugin bound to `bus`.
    pub fn new(config: CsiReplayConfig, bus: EventBus) -> Self {
        Self::with_stats(config, bus, CsiReplayStats::new().shared())
    }

    /// Creates a CSI replay plugin that updates the provided shared statistics.
    pub fn with_stats(config: CsiReplayConfig, bus: EventBus, stats: Arc<CsiReplayStats>) -> Self {
        Self {
            id: PluginId::new(PLUGIN_ID),
            config,
            bus,
            stats,
            cancel_tx: None,
            task: None,
            producer_alive: Arc::new(AtomicBool::new(false)),
            failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns shared replay statistics.
    pub fn stats(&self) -> &Arc<CsiReplayStats> {
        &self.stats
    }

    /// Returns the configured display-safe fixture path.
    pub fn fixture_display_path(&self) -> String {
        self.config.display_path()
    }

    fn now() -> Timestamp {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
            .unwrap_or(0);
        Timestamp::from_nanos(nanos)
    }
}

impl Plugin for CsiReplayPlugin {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "CSI Replay"
    }

    fn version(&self) -> Version {
        Version::new(0, 1, 0)
    }

    fn description(&self) -> &str {
        "Deterministic WiFi CSI fixture replay for development and testing (not live RF)"
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
                from: LifecycleState::Running,
                to: LifecycleState::Initialized,
            }));
        }

        // Fail fast when the fixture cannot be opened or its header is invalid.
        if let Err(error) = FixtureReader::open(&self.config.path) {
            tracing::error!(%error, "CSI fixture open failed during initialize");
            self.failed.store(true, Ordering::Relaxed);
            self.stats.set_completion(CsiReplayCompletion::Failed);
            self.stats.set_last_error(error.to_string());
            return Err(PluginError::lifecycle(
                LifecycleError::InitializationFailed(self.id.clone()),
            ));
        }

        self.failed.store(false, Ordering::Relaxed);
        self.stats.reset();
        self.stats.set_completion(CsiReplayCompletion::Active);

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let bus = self.bus.clone();
        let config = self.config.clone();
        let stats = Arc::clone(&self.stats);
        let producer_alive = Arc::clone(&self.producer_alive);
        let failed = Arc::clone(&self.failed);
        let plugin_id = self.id.clone();

        if bus
            .publish(Event::CsiReplayStarted(CsiReplayStarted {
                sensor_id: SENSOR_ID,
                timestamp: Self::now(),
            }))
            .is_err()
        {
            tracing::warn!("CSI replay started with no event subscribers");
        }

        producer_alive.store(true, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            let interval = Duration::from_millis(config.frame_interval_ms);
            let mut cancel_rx = cancel_rx;
            let mut emitted = 0_u64;
            let mut completed_cleanly = false;

            'outer: loop {
                let mut reader = match FixtureReader::open(&config.path) {
                    Ok(reader) => reader,
                    Err(error) => {
                        failed.store(true, Ordering::Relaxed);
                        stats.set_completion(CsiReplayCompletion::Failed);
                        stats.set_last_error(error.to_string());
                        let _ = bus.publish(Event::CsiReplayFailed(CsiReplayFailed {
                            sensor_id: SENSOR_ID,
                            timestamp: CsiReplayPlugin::now(),
                            kind: CsiReplayFailureKind::FixtureError,
                        }));
                        tracing::error!(plugin = %plugin_id, %error, "CSI fixture open failed");
                        break;
                    }
                };

                loop {
                    if *cancel_rx.borrow() {
                        break 'outer;
                    }

                    if config.maximum_frames > 0 && emitted >= config.maximum_frames {
                        completed_cleanly = true;
                        break 'outer;
                    }

                    let frame = match reader.next_frame() {
                        Ok(Some(frame)) => {
                            stats.record_read();
                            frame
                        }
                        Ok(None) => {
                            if config.loop_playback
                                && (config.maximum_frames == 0 || emitted < config.maximum_frames)
                            {
                                tracing::info!(plugin = %plugin_id, "CSI replay looping fixture");
                                continue 'outer;
                            }
                            completed_cleanly = true;
                            break 'outer;
                        }
                        Err(error) => {
                            stats.record_read();
                            stats.record_rejected(error.to_string());
                            failed.store(true, Ordering::Relaxed);
                            stats.set_completion(CsiReplayCompletion::Failed);
                            let _ = bus.publish(Event::CsiReplayFailed(CsiReplayFailed {
                                sensor_id: SENSOR_ID,
                                timestamp: CsiReplayPlugin::now(),
                                kind: CsiReplayFailureKind::MalformedFrame,
                            }));
                            tracing::error!(plugin = %plugin_id, %error, "CSI fixture frame rejected");
                            break 'outer;
                        }
                    };

                    let receive_timestamp = CsiReplayPlugin::now();
                    if !publish_frame(&bus, &frame, receive_timestamp) {
                        failed.store(true, Ordering::Relaxed);
                        stats.set_completion(CsiReplayCompletion::Failed);
                        stats.set_last_error("failed to publish CSI frame event");
                        let _ = bus.publish(Event::CsiReplayFailed(CsiReplayFailed {
                            sensor_id: SENSOR_ID,
                            timestamp: CsiReplayPlugin::now(),
                            kind: CsiReplayFailureKind::PublishFailed,
                        }));
                        break 'outer;
                    }

                    stats.record_accepted(
                        frame.sequence(),
                        frame.capture_timestamp().as_nanos(),
                        frame.receive_antennas(),
                        frame.transmit_antennas(),
                        frame.subcarrier_count(),
                        frame.center_frequency_hz(),
                        frame.bandwidth_hz(),
                    );
                    emitted = emitted.saturating_add(1);

                    tracing::debug!(
                        sequence = frame.sequence(),
                        frame_id = frame.frame_id().value(),
                        rx = frame.receive_antennas(),
                        tx = frame.transmit_antennas(),
                        subcarriers = frame.subcarrier_count(),
                        "CSI replay frame produced"
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {}
                        changed = cancel_rx.changed() => {
                            if changed.is_err() || *cancel_rx.borrow() {
                                break 'outer;
                            }
                        }
                    }
                }
            }

            producer_alive.store(false, Ordering::Relaxed);
            let cancelled = *cancel_rx.borrow();

            if failed.load(Ordering::Relaxed) {
                return;
            }

            if cancelled {
                stats.set_completion(CsiReplayCompletion::Stopped);
                return;
            }

            if completed_cleanly {
                stats.set_completion(CsiReplayCompletion::Completed);
                let _ = bus.publish(Event::CsiReplayCompleted(CsiReplayCompleted {
                    sensor_id: SENSOR_ID,
                    timestamp: CsiReplayPlugin::now(),
                    frames_accepted: stats.frames_accepted(),
                }));
                tracing::info!(
                    frames = stats.frames_accepted(),
                    "CSI replay completed finite fixture pass"
                );
                return;
            }

            failed.store(true, Ordering::Relaxed);
            stats.set_completion(CsiReplayCompletion::Failed);
            stats.set_last_error("CSI replay producer exited unexpectedly");
            let _ = bus.publish(Event::CsiReplayFailed(CsiReplayFailed {
                sensor_id: SENSOR_ID,
                timestamp: CsiReplayPlugin::now(),
                kind: CsiReplayFailureKind::ProducerExited,
            }));
            tracing::error!(plugin = %plugin_id, "CSI replay producer exited unexpectedly");
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
        if !self.failed.load(Ordering::Relaxed)
            && self.stats.completion() != CsiReplayCompletion::Completed
        {
            self.stats.set_completion(CsiReplayCompletion::Stopped);
        }

        if self
            .bus
            .publish(Event::CsiReplayStopped(CsiReplayStopped {
                sensor_id: SENSOR_ID,
                timestamp: Self::now(),
            }))
            .is_err()
        {
            tracing::debug!("CSI replay stopped with no event subscribers");
        }

        Ok(())
    }

    fn health(&self) -> HealthStatus {
        if self.failed.load(Ordering::Relaxed)
            || self.stats.completion() == CsiReplayCompletion::Failed
        {
            return HealthStatus::Unhealthy;
        }
        if self.stats.completion() == CsiReplayCompletion::Completed {
            // Finite completion is not a failure.
            return HealthStatus::Healthy;
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

fn publish_frame(bus: &EventBus, frame: &CsiFrame, receive_timestamp: Timestamp) -> bool {
    let event = Event::CsiFrameReceived(CsiFrameReceived {
        frame_id: frame.frame_id(),
        sensor_id: frame.sensor_id(),
        sequence: frame.sequence(),
        capture_timestamp: frame.capture_timestamp(),
        receive_timestamp,
        receive_antennas: frame.receive_antennas(),
        transmit_antennas: frame.transmit_antennas(),
        subcarrier_count: frame.subcarrier_count() as u16,
        center_frequency_hz: frame.center_frequency_hz(),
        bandwidth_hz: frame.bandwidth_hz(),
        source: CsiDataSource::Replay,
        // Keep the full matrix off the bus; retain only a lightweight token.
        frame_token: Some(Arc::new(())),
    });
    bus.publish(event).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_plugin_runtime::{LifecycleState, PluginRuntime};
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::time::{Duration, timeout};

    const HEADER: &str = r#"{"record_type":"header","schema":"aeryon-csi-fixture","version":1,"sensor_id":"2","description":"test fixture","sample_layout":"rx-tx-subcarrier"}"#;

    fn write_fixture(frames: u64) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temp");
        writeln!(file, "{HEADER}").expect("header");
        for sequence in 0..frames {
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
    async fn metadata_and_capability_are_valid() {
        let plugin = CsiReplayPlugin::new(CsiReplayConfig::default(), EventBus::new());
        assert_eq!(plugin.id().as_str(), PLUGIN_ID);
        assert_eq!(plugin.capabilities(), &[Capability::Sensor]);
        assert!(plugin.description().contains("not live"));
    }

    #[tokio::test(start_paused = true)]
    async fn lifecycle_emits_frames_in_order_with_interval() {
        let fixture = write_fixture(5);
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = CsiReplayConfig {
            enabled: true,
            path: fixture.path().to_path_buf(),
            loop_playback: false,
            frame_interval_ms: 50,
            maximum_frames: 0,
        };

        runtime
            .register(Box::new(CsiReplayPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Running));

        let started = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("timeout")
            .expect("recv");
        assert!(matches!(started, Event::CsiReplayStarted(_)));

        let mut sequences = Vec::new();
        while sequences.len() < 3 {
            tokio::time::advance(Duration::from_millis(50)).await;
            let event = timeout(Duration::from_secs(1), receiver.recv())
                .await
                .expect("timeout")
                .expect("recv");
            if let Event::CsiFrameReceived(frame) = event {
                sequences.push(frame.sequence);
            }
        }
        assert_eq!(sequences, vec![0, 1, 2]);

        runtime.stop(&id).expect("stop");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Stopped));
    }

    #[tokio::test(start_paused = true)]
    async fn maximum_frames_and_completion() {
        let fixture = write_fixture(8);
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let stats = CsiReplayStats::new().shared();
        let mut runtime = PluginRuntime::new();
        let config = CsiReplayConfig {
            enabled: true,
            path: fixture.path().to_path_buf(),
            loop_playback: false,
            frame_interval_ms: 10,
            maximum_frames: 3,
        };

        runtime
            .register(Box::new(CsiReplayPlugin::with_stats(
                config,
                bus,
                Arc::clone(&stats),
            )))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");

        let mut frames = 0_u32;
        let mut completed = false;
        for _ in 0..20 {
            tokio::time::advance(Duration::from_millis(10)).await;
            if let Ok(Ok(event)) = timeout(Duration::from_millis(20), receiver.recv()).await {
                match event {
                    Event::CsiFrameReceived(_) => frames += 1,
                    Event::CsiReplayCompleted(_) => {
                        completed = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(frames, 3);
        assert!(completed);
        assert_eq!(stats.completion(), CsiReplayCompletion::Completed);
        assert_eq!(stats.frames_accepted(), 3);
        runtime.stop(&id).expect("stop");
    }

    #[tokio::test(start_paused = true)]
    async fn loop_behavior_restarts_sequences() {
        let fixture = write_fixture(2);
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = CsiReplayConfig {
            enabled: true,
            path: fixture.path().to_path_buf(),
            loop_playback: true,
            frame_interval_ms: 5,
            maximum_frames: 4,
        };

        runtime
            .register(Box::new(CsiReplayPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");

        let mut sequences = Vec::new();
        for _ in 0..40 {
            tokio::time::advance(Duration::from_millis(5)).await;
            if let Ok(Ok(Event::CsiFrameReceived(frame))) =
                timeout(Duration::from_millis(20), receiver.recv()).await
            {
                sequences.push(frame.sequence);
            }
            if sequences.len() >= 4 {
                break;
            }
        }

        assert_eq!(sequences, vec![0, 1, 0, 1]);
        runtime.stop(&id).expect("stop");
    }

    #[tokio::test]
    async fn malformed_fixture_fails_health() {
        let mut file = NamedTempFile::new().expect("temp");
        writeln!(file, "{HEADER}").expect("header");
        writeln!(
            file,
            r#"{{"record_type":"frame","frame_id":1,"sequence":0,"capture_timestamp_nanos":1,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[0],"samples":[{{"re":1.0,"im":0.0}}]}}"#
        )
        .expect("bad frame");

        let bus = EventBus::new();
        let _receiver = bus.subscribe();
        let stats = CsiReplayStats::new().shared();
        let mut runtime = PluginRuntime::new();
        let config = CsiReplayConfig {
            enabled: true,
            path: file.path().to_path_buf(),
            frame_interval_ms: 10,
            ..CsiReplayConfig::default()
        };

        runtime
            .register(Box::new(CsiReplayPlugin::with_stats(
                config,
                bus,
                Arc::clone(&stats),
            )))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");

        timeout(Duration::from_secs(2), async {
            loop {
                if stats.completion() == CsiReplayCompletion::Failed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("failed state");

        assert_eq!(
            runtime.health(&id).expect("health"),
            HealthStatus::Unhealthy
        );
        assert!(stats.frames_rejected() >= 1);
        runtime.stop(&id).expect("stop");
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_during_active_replay_leaves_no_producer() {
        let fixture = write_fixture(64);
        let bus = EventBus::new();
        let _receiver = bus.subscribe();
        let mut runtime = PluginRuntime::new();
        let config = CsiReplayConfig {
            enabled: true,
            path: fixture.path().to_path_buf(),
            frame_interval_ms: 100,
            ..CsiReplayConfig::default()
        };

        runtime
            .register(Box::new(CsiReplayPlugin::new(config, bus)))
            .expect("register");
        let id = PluginId::new(PLUGIN_ID);
        runtime.start(&id).expect("start");
        tokio::time::advance(Duration::from_millis(50)).await;
        runtime.stop(&id).expect("stop");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Stopped));
    }
}
