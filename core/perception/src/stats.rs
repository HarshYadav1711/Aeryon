//! Shared perception runtime statistics.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Perception worker lifecycle classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerceptionWorkerState {
    /// Perception is disabled in configuration.
    Disabled,
    /// Worker has not processed input yet.
    Idle,
    /// Worker is running.
    Running,
    /// Finite input completed cleanly.
    Completed,
    /// Worker stopped during graceful shutdown.
    Stopped,
    /// Worker failed or exited unexpectedly.
    Failed,
}

impl PerceptionWorkerState {
    /// Stable API label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// Counters and latest perception state.
#[derive(Debug, Default)]
pub struct PerceptionStats {
    enabled: AtomicBool,
    worker_state: AtomicU64,
    profile_id: Mutex<Option<String>>,
    profile_version: AtomicU64,
    feature_vectors_received: AtomicU64,
    observations_produced: AtomicU64,
    observation_failures: AtomicU64,
    latest_observation_id: AtomicU64,
    has_observation: AtomicBool,
    latest_state: Mutex<Option<String>>,
    latest_activity_score_bits: AtomicU64,
    has_score: AtomicBool,
    latest_threshold_margin_bits: AtomicU64,
    has_margin: AtomicBool,
    last_duration_ns: AtomicU64,
    average_duration_ns_bits: AtomicU64,
    average_count: AtomicU64,
    last_error: Mutex<Option<String>>,
    last_warning: Mutex<Option<String>>,
    unexpected_exit: AtomicBool,
}

impl PerceptionStats {
    /// Creates empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps statistics for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Configures identity fields when the service starts or is disabled.
    pub fn configure(&self, enabled: bool, profile_id: Option<&str>, profile_version: u32) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if let Ok(mut guard) = self.profile_id.lock() {
            *guard = profile_id.map(str::to_owned);
        }
        self.profile_version
            .store(u64::from(profile_version), Ordering::Relaxed);
        if enabled {
            self.set_worker_state(PerceptionWorkerState::Idle);
        } else {
            self.set_worker_state(PerceptionWorkerState::Disabled);
        }
    }

    /// Resets counters for a new service start.
    pub fn reset_counters(&self) {
        self.feature_vectors_received.store(0, Ordering::Relaxed);
        self.observations_produced.store(0, Ordering::Relaxed);
        self.observation_failures.store(0, Ordering::Relaxed);
        self.latest_observation_id.store(0, Ordering::Relaxed);
        self.has_observation.store(false, Ordering::Relaxed);
        self.has_score.store(false, Ordering::Relaxed);
        self.has_margin.store(false, Ordering::Relaxed);
        self.last_duration_ns.store(0, Ordering::Relaxed);
        self.average_duration_ns_bits.store(0, Ordering::Relaxed);
        self.average_count.store(0, Ordering::Relaxed);
        self.unexpected_exit.store(false, Ordering::Relaxed);
        if let Ok(mut guard) = self.latest_state.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.last_warning.lock() {
            *guard = None;
        }
    }

    /// Sets worker lifecycle state.
    pub fn set_worker_state(&self, state: PerceptionWorkerState) {
        self.worker_state
            .store(worker_state_to_u64(state), Ordering::Relaxed);
    }

    /// Returns worker lifecycle state.
    pub fn worker_state(&self) -> PerceptionWorkerState {
        u64_to_worker_state(self.worker_state.load(Ordering::Relaxed))
    }

    /// Whether perception is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Active profile identity.
    pub fn profile_id(&self) -> Option<String> {
        self.profile_id.lock().ok().and_then(|guard| guard.clone())
    }

    /// Active profile version.
    pub fn profile_version(&self) -> Option<u32> {
        if self.profile_id().is_some() {
            Some(self.profile_version.load(Ordering::Relaxed) as u32)
        } else {
            None
        }
    }

    /// Records receipt of one feature vector.
    pub fn record_feature_received(&self) {
        self.feature_vectors_received
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Records a successfully produced observation.
    pub fn record_success(
        &self,
        observation_id: u64,
        state: &str,
        activity_score: f64,
        threshold_margin: f64,
        duration_ns: u64,
    ) {
        self.observations_produced.fetch_add(1, Ordering::Relaxed);
        self.latest_observation_id
            .store(observation_id, Ordering::Relaxed);
        self.has_observation.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.latest_state.lock() {
            *guard = Some(state.to_owned());
        }
        self.latest_activity_score_bits
            .store(activity_score.to_bits(), Ordering::Relaxed);
        self.has_score.store(true, Ordering::Relaxed);
        self.latest_threshold_margin_bits
            .store(threshold_margin.to_bits(), Ordering::Relaxed);
        self.has_margin.store(true, Ordering::Relaxed);
        self.last_duration_ns.store(duration_ns, Ordering::Relaxed);
        self.update_average_duration(duration_ns);
    }

    /// Records a failed observation.
    pub fn record_failure(&self, error: impl Into<String>) {
        self.observation_failures.fetch_add(1, Ordering::Relaxed);
        self.set_last_error(error);
    }

    /// Stores the last error summary.
    pub fn set_last_error(&self, error: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(error.into());
        }
    }

    /// Stores the last warning summary.
    pub fn set_last_warning(&self, warning: impl Into<String>) {
        if let Ok(mut guard) = self.last_warning.lock() {
            *guard = Some(warning.into());
        }
    }

    /// Marks an unexpected worker exit.
    pub fn mark_unexpected_exit(&self) {
        self.unexpected_exit.store(true, Ordering::Relaxed);
        self.set_worker_state(PerceptionWorkerState::Failed);
    }

    /// Whether the worker exited unexpectedly.
    pub fn unexpected_exit(&self) -> bool {
        self.unexpected_exit.load(Ordering::Relaxed)
    }

    /// Feature vectors received.
    pub fn feature_vectors_received(&self) -> u64 {
        self.feature_vectors_received.load(Ordering::Relaxed)
    }

    /// Observations produced.
    pub fn observations_produced(&self) -> u64 {
        self.observations_produced.load(Ordering::Relaxed)
    }

    /// Observation failures.
    pub fn observation_failures(&self) -> u64 {
        self.observation_failures.load(Ordering::Relaxed)
    }

    /// Latest observation identity.
    pub fn latest_observation_id(&self) -> Option<u64> {
        if self.has_observation.load(Ordering::Relaxed) {
            Some(self.latest_observation_id.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Latest observation state label.
    pub fn latest_observation_state(&self) -> Option<String> {
        self.latest_state
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Latest activity score.
    pub fn latest_activity_score(&self) -> Option<f64> {
        if self.has_score.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.latest_activity_score_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Latest threshold margin.
    pub fn latest_threshold_margin(&self) -> Option<f64> {
        if self.has_margin.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.latest_threshold_margin_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Last processing duration in nanoseconds.
    pub fn last_duration_ns(&self) -> Option<u64> {
        if self.has_observation.load(Ordering::Relaxed) {
            Some(self.last_duration_ns.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Running average processing duration in nanoseconds.
    pub fn average_duration_ns(&self) -> Option<u64> {
        if self.average_count.load(Ordering::Relaxed) == 0 {
            None
        } else {
            Some(f64::from_bits(self.average_duration_ns_bits.load(Ordering::Relaxed)) as u64)
        }
    }

    /// Last error summary.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|guard| guard.clone())
    }

    /// Last warning summary.
    pub fn last_warning(&self) -> Option<String> {
        self.last_warning
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn update_average_duration(&self, duration_ns: u64) {
        let count = self.average_count.fetch_add(1, Ordering::Relaxed) + 1;
        let previous = f64::from_bits(self.average_duration_ns_bits.load(Ordering::Relaxed));
        let updated = previous + (duration_ns as f64 - previous) / count as f64;
        self.average_duration_ns_bits
            .store(updated.to_bits(), Ordering::Relaxed);
    }
}

fn worker_state_to_u64(state: PerceptionWorkerState) -> u64 {
    match state {
        PerceptionWorkerState::Disabled => 0,
        PerceptionWorkerState::Idle => 1,
        PerceptionWorkerState::Running => 2,
        PerceptionWorkerState::Completed => 3,
        PerceptionWorkerState::Stopped => 4,
        PerceptionWorkerState::Failed => 5,
    }
}

fn u64_to_worker_state(value: u64) -> PerceptionWorkerState {
    match value {
        1 => PerceptionWorkerState::Idle,
        2 => PerceptionWorkerState::Running,
        3 => PerceptionWorkerState::Completed,
        4 => PerceptionWorkerState::Stopped,
        5 => PerceptionWorkerState::Failed,
        _ => PerceptionWorkerState::Disabled,
    }
}
