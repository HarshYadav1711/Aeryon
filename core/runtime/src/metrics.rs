//! In-memory runtime statistics for the first-signal vertical slice.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use aeryon_csi_replay::{CsiReplayCompletion, CsiReplayStats};
use aeryon_plugin_runtime::LifecycleState;

use crate::health::RuntimeHealth;

/// Shared counters updated by the event consumer and plugin lifecycle.
#[derive(Debug)]
pub struct RuntimeMetrics {
    frames_received: AtomicU64,
    last_sequence: AtomicU64,
    last_frame_nanos: AtomicU64,
    has_frame: AtomicBool,
    consumer_running: AtomicBool,
    sensor_lifecycle: AtomicU64,
    csi_lifecycle: AtomicU64,
    started_at: std::sync::Mutex<Option<Instant>>,
    csi_started_at: std::sync::Mutex<Option<Instant>>,
    /// Dedicated CSI replay statistics (shared with the replay plugin).
    csi_replay: Arc<CsiReplayStats>,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeMetrics {
    /// Creates empty metrics.
    pub fn new() -> Self {
        Self {
            frames_received: AtomicU64::new(0),
            last_sequence: AtomicU64::new(0),
            last_frame_nanos: AtomicU64::new(0),
            has_frame: AtomicBool::new(false),
            consumer_running: AtomicBool::new(false),
            sensor_lifecycle: AtomicU64::new(0),
            csi_lifecycle: AtomicU64::new(0),
            started_at: std::sync::Mutex::new(None),
            csi_started_at: std::sync::Mutex::new(None),
            csi_replay: CsiReplayStats::new().shared(),
        }
    }

    /// Wraps metrics in an [`Arc`] for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Returns the shared CSI replay statistics handle.
    pub fn csi_replay(&self) -> &Arc<CsiReplayStats> {
        &self.csi_replay
    }

    /// Records that the event consumer task is running.
    pub fn set_consumer_running(&self, running: bool) {
        self.consumer_running.store(running, Ordering::Relaxed);
    }

    /// Returns whether the event consumer task is running.
    pub fn consumer_running(&self) -> bool {
        self.consumer_running.load(Ordering::Relaxed)
    }

    /// Records the synthetic sensor lifecycle state.
    pub fn set_sensor_lifecycle(&self, state: LifecycleState) {
        self.sensor_lifecycle
            .store(lifecycle_to_u64(state), Ordering::Relaxed);
        if state == LifecycleState::Running {
            if let Ok(mut guard) = self.started_at.lock() {
                *guard = Some(Instant::now());
            }
        }
    }

    /// Returns the tracked synthetic sensor lifecycle state, if known.
    pub fn sensor_lifecycle(&self) -> Option<LifecycleState> {
        u64_to_lifecycle(self.sensor_lifecycle.load(Ordering::Relaxed))
    }

    /// Records the CSI replay plugin lifecycle state.
    pub fn set_csi_lifecycle(&self, state: LifecycleState) {
        self.csi_lifecycle
            .store(lifecycle_to_u64(state), Ordering::Relaxed);
        if state == LifecycleState::Running {
            if let Ok(mut guard) = self.csi_started_at.lock() {
                *guard = Some(Instant::now());
            }
        }
    }

    /// Returns the tracked CSI replay lifecycle state, if known.
    pub fn csi_lifecycle(&self) -> Option<LifecycleState> {
        u64_to_lifecycle(self.csi_lifecycle.load(Ordering::Relaxed))
    }

    /// Records a received frame event (synthetic or CSI metadata).
    pub fn record_frame(&self, sequence: u64, timestamp_nanos: u64) {
        self.frames_received.fetch_add(1, Ordering::Relaxed);
        self.last_sequence.store(sequence, Ordering::Relaxed);
        self.last_frame_nanos
            .store(timestamp_nanos, Ordering::Relaxed);
        self.has_frame.store(true, Ordering::Relaxed);
    }

    /// Returns the number of frames received by the runtime subscriber.
    pub fn frames_received(&self) -> u64 {
        self.frames_received.load(Ordering::Relaxed)
    }

    /// Returns the last observed frame sequence.
    pub fn last_sequence(&self) -> Option<u64> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.last_sequence.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Returns the timestamp of the most recent frame in nanoseconds since epoch.
    pub fn last_frame_nanos(&self) -> Option<u64> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.last_frame_nanos.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Evaluates whether the runtime should report degraded or failed health.
    pub fn evaluate_health(
        &self,
        synthetic_enabled: bool,
        csi_replay_enabled: bool,
        first_frame_timeout: Duration,
    ) -> RuntimeHealth {
        if !self.consumer_running() && (synthetic_enabled || csi_replay_enabled) {
            return RuntimeHealth::Failed;
        }

        if synthetic_enabled {
            match self.evaluate_source_health(
                self.sensor_lifecycle(),
                &self.started_at,
                first_frame_timeout,
            ) {
                RuntimeHealth::Failed => return RuntimeHealth::Failed,
                RuntimeHealth::Degraded => return RuntimeHealth::Degraded,
                _ => {}
            }
        }

        if csi_replay_enabled {
            // Finite CSI replay completion must not be treated as failure.
            match self.csi_replay.completion() {
                CsiReplayCompletion::Failed => return RuntimeHealth::Failed,
                CsiReplayCompletion::Completed | CsiReplayCompletion::Stopped => {
                    return RuntimeHealth::Running;
                }
                CsiReplayCompletion::Active | CsiReplayCompletion::Idle => {}
            }

            return self.evaluate_source_health(
                self.csi_lifecycle(),
                &self.csi_started_at,
                first_frame_timeout,
            );
        }

        RuntimeHealth::Running
    }

    fn evaluate_source_health(
        &self,
        lifecycle: Option<LifecycleState>,
        started_at: &std::sync::Mutex<Option<Instant>>,
        first_frame_timeout: Duration,
    ) -> RuntimeHealth {
        match lifecycle {
            Some(LifecycleState::Failed) => RuntimeHealth::Failed,
            Some(LifecycleState::Running) => {
                if self.has_frame.load(Ordering::Relaxed) {
                    RuntimeHealth::Running
                } else if let Ok(guard) = started_at.lock() {
                    if guard.is_some_and(|started| started.elapsed() > first_frame_timeout) {
                        RuntimeHealth::Degraded
                    } else {
                        RuntimeHealth::Running
                    }
                } else {
                    RuntimeHealth::Running
                }
            }
            Some(LifecycleState::Stopped)
            | Some(LifecycleState::Registered)
            | Some(LifecycleState::Initialized)
            | None => RuntimeHealth::Running,
        }
    }
}

fn lifecycle_to_u64(state: LifecycleState) -> u64 {
    match state {
        LifecycleState::Registered => 1,
        LifecycleState::Initialized => 2,
        LifecycleState::Running => 3,
        LifecycleState::Stopped => 4,
        LifecycleState::Failed => 5,
    }
}

fn u64_to_lifecycle(value: u64) -> Option<LifecycleState> {
    match value {
        1 => Some(LifecycleState::Registered),
        2 => Some(LifecycleState::Initialized),
        3 => Some(LifecycleState::Running),
        4 => Some(LifecycleState::Stopped),
        5 => Some(LifecycleState::Failed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_track_frame_progression() {
        let metrics = RuntimeMetrics::new();
        metrics.record_frame(0, 10);
        metrics.record_frame(1, 20);
        assert_eq!(metrics.frames_received(), 2);
        assert_eq!(metrics.last_sequence(), Some(1));
        assert_eq!(metrics.last_frame_nanos(), Some(20));
    }

    #[test]
    fn missing_frames_after_timeout_are_degraded() {
        let metrics = RuntimeMetrics::new();
        metrics.set_consumer_running(true);
        metrics.set_sensor_lifecycle(LifecycleState::Running);
        if let Ok(mut guard) = metrics.started_at.lock() {
            *guard = Some(Instant::now() - Duration::from_secs(5));
        }
        assert_eq!(
            metrics.evaluate_health(true, false, Duration::from_secs(1)),
            RuntimeHealth::Degraded
        );
    }

    #[test]
    fn csi_finite_completion_is_not_failure() {
        let metrics = RuntimeMetrics::new();
        metrics.set_consumer_running(true);
        metrics.set_csi_lifecycle(LifecycleState::Running);
        metrics
            .csi_replay()
            .set_completion(CsiReplayCompletion::Completed);
        assert_eq!(
            metrics.evaluate_health(false, true, Duration::from_secs(1)),
            RuntimeHealth::Running
        );
    }
}
