//! In-memory runtime statistics for the first-signal vertical slice.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use aeryon_plugin_runtime::LifecycleState;

use crate::health::RuntimeHealth;

/// Shared counters updated by the event consumer and plugin lifecycle.
#[derive(Debug, Default)]
pub struct RuntimeMetrics {
    frames_received: AtomicU64,
    last_sequence: AtomicU64,
    last_frame_nanos: AtomicU64,
    has_frame: AtomicBool,
    consumer_running: AtomicBool,
    sensor_lifecycle: AtomicU64,
    started_at: std::sync::Mutex<Option<Instant>>,
}

impl RuntimeMetrics {
    /// Creates empty metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps metrics in an [`Arc`] for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
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

    /// Records a received frame event.
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
        sensor_enabled: bool,
        first_frame_timeout: Duration,
    ) -> RuntimeHealth {
        if !sensor_enabled {
            return RuntimeHealth::Running;
        }

        if !self.consumer_running() {
            return RuntimeHealth::Failed;
        }

        match self.sensor_lifecycle() {
            Some(LifecycleState::Failed) => RuntimeHealth::Failed,
            Some(LifecycleState::Running) => {
                if self.has_frame.load(Ordering::Relaxed) {
                    RuntimeHealth::Running
                } else if let Ok(guard) = self.started_at.lock() {
                    if guard.is_some_and(|started| started.elapsed() > first_frame_timeout) {
                        RuntimeHealth::Degraded
                    } else {
                        RuntimeHealth::Running
                    }
                } else {
                    RuntimeHealth::Running
                }
            }
            Some(LifecycleState::Stopped) | Some(LifecycleState::Registered) => {
                RuntimeHealth::Running
            }
            Some(LifecycleState::Initialized) => RuntimeHealth::Running,
            None => RuntimeHealth::Running,
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
            metrics.evaluate_health(true, Duration::from_secs(1)),
            RuntimeHealth::Degraded
        );
    }
}
