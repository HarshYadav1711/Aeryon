//! Shared feature-extraction runtime statistics.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Feature worker lifecycle classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureWorkerState {
    /// Feature extraction is disabled in configuration.
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

impl FeatureWorkerState {
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

/// Counters and latest feature-extraction state.
#[derive(Debug, Default)]
pub struct FeatureStats {
    enabled: AtomicBool,
    worker_state: AtomicU64,
    profile_id: Mutex<Option<String>>,
    profile_version: AtomicU64,
    schema_id: Mutex<Option<String>>,
    schema_version: AtomicU64,
    dsp_results_received: AtomicU64,
    feature_vectors_produced: AtomicU64,
    feature_failures: AtomicU64,
    latest_feature_vector_id: AtomicU64,
    has_vector: AtomicBool,
    latest_first_sequence: AtomicU64,
    latest_last_sequence: AtomicU64,
    last_duration_ns: AtomicU64,
    average_duration_ns_bits: AtomicU64,
    average_count: AtomicU64,
    last_error: Mutex<Option<String>>,
    last_warning: Mutex<Option<String>>,
    unexpected_exit: AtomicBool,
}

impl FeatureStats {
    /// Creates empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps statistics for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Configures identity fields when the service starts or is disabled.
    pub fn configure(
        &self,
        enabled: bool,
        profile_id: Option<&str>,
        profile_version: u32,
        schema_id: Option<&str>,
        schema_version: u32,
    ) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if let Ok(mut guard) = self.profile_id.lock() {
            *guard = profile_id.map(str::to_owned);
        }
        self.profile_version
            .store(u64::from(profile_version), Ordering::Relaxed);
        if let Ok(mut guard) = self.schema_id.lock() {
            *guard = schema_id.map(str::to_owned);
        }
        self.schema_version
            .store(u64::from(schema_version), Ordering::Relaxed);
        if enabled {
            self.set_worker_state(FeatureWorkerState::Idle);
        } else {
            self.set_worker_state(FeatureWorkerState::Disabled);
        }
    }

    /// Resets counters for a new service start.
    pub fn reset_counters(&self) {
        self.dsp_results_received.store(0, Ordering::Relaxed);
        self.feature_vectors_produced.store(0, Ordering::Relaxed);
        self.feature_failures.store(0, Ordering::Relaxed);
        self.latest_feature_vector_id.store(0, Ordering::Relaxed);
        self.has_vector.store(false, Ordering::Relaxed);
        self.latest_first_sequence.store(0, Ordering::Relaxed);
        self.latest_last_sequence.store(0, Ordering::Relaxed);
        self.last_duration_ns.store(0, Ordering::Relaxed);
        self.average_duration_ns_bits.store(0, Ordering::Relaxed);
        self.average_count.store(0, Ordering::Relaxed);
        self.unexpected_exit.store(false, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.last_warning.lock() {
            *guard = None;
        }
    }

    /// Sets worker lifecycle state.
    pub fn set_worker_state(&self, state: FeatureWorkerState) {
        self.worker_state
            .store(worker_state_to_u64(state), Ordering::Relaxed);
    }

    /// Returns worker lifecycle state.
    pub fn worker_state(&self) -> FeatureWorkerState {
        u64_to_worker_state(self.worker_state.load(Ordering::Relaxed))
    }

    /// Whether features are enabled.
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

    /// Active schema identity.
    pub fn schema_id(&self) -> Option<String> {
        self.schema_id.lock().ok().and_then(|guard| guard.clone())
    }

    /// Active schema version.
    pub fn schema_version(&self) -> Option<u32> {
        if self.schema_id().is_some() {
            Some(self.schema_version.load(Ordering::Relaxed) as u32)
        } else {
            None
        }
    }

    /// Records receipt of one DSP result.
    pub fn record_dsp_received(&self) {
        self.dsp_results_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a successfully produced feature vector.
    pub fn record_success(
        &self,
        feature_vector_id: u64,
        first_sequence: u64,
        last_sequence: u64,
        duration_ns: u64,
    ) {
        self.feature_vectors_produced
            .fetch_add(1, Ordering::Relaxed);
        self.latest_feature_vector_id
            .store(feature_vector_id, Ordering::Relaxed);
        self.latest_first_sequence
            .store(first_sequence, Ordering::Relaxed);
        self.latest_last_sequence
            .store(last_sequence, Ordering::Relaxed);
        self.has_vector.store(true, Ordering::Relaxed);
        self.last_duration_ns.store(duration_ns, Ordering::Relaxed);
        self.update_average_duration(duration_ns);
    }

    /// Records a failed extraction.
    pub fn record_failure(&self, error: impl Into<String>) {
        self.feature_failures.fetch_add(1, Ordering::Relaxed);
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
        self.set_worker_state(FeatureWorkerState::Failed);
    }

    /// Whether the worker exited unexpectedly.
    pub fn unexpected_exit(&self) -> bool {
        self.unexpected_exit.load(Ordering::Relaxed)
    }

    /// DSP results received.
    pub fn dsp_results_received(&self) -> u64 {
        self.dsp_results_received.load(Ordering::Relaxed)
    }

    /// Feature vectors produced.
    pub fn feature_vectors_produced(&self) -> u64 {
        self.feature_vectors_produced.load(Ordering::Relaxed)
    }

    /// Feature failures.
    pub fn feature_failures(&self) -> u64 {
        self.feature_failures.load(Ordering::Relaxed)
    }

    /// Latest feature-vector identity.
    pub fn latest_feature_vector_id(&self) -> Option<u64> {
        if self.has_vector.load(Ordering::Relaxed) {
            Some(self.latest_feature_vector_id.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Latest sequence range.
    pub fn latest_sequence_range(&self) -> Option<(u64, u64)> {
        if self.has_vector.load(Ordering::Relaxed) {
            Some((
                self.latest_first_sequence.load(Ordering::Relaxed),
                self.latest_last_sequence.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Last processing duration in nanoseconds.
    pub fn last_duration_ns(&self) -> Option<u64> {
        if self.has_vector.load(Ordering::Relaxed) {
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

fn worker_state_to_u64(state: FeatureWorkerState) -> u64 {
    match state {
        FeatureWorkerState::Disabled => 0,
        FeatureWorkerState::Idle => 1,
        FeatureWorkerState::Running => 2,
        FeatureWorkerState::Completed => 3,
        FeatureWorkerState::Stopped => 4,
        FeatureWorkerState::Failed => 5,
    }
}

fn u64_to_worker_state(value: u64) -> FeatureWorkerState {
    match value {
        1 => FeatureWorkerState::Idle,
        2 => FeatureWorkerState::Running,
        3 => FeatureWorkerState::Completed,
        4 => FeatureWorkerState::Stopped,
        5 => FeatureWorkerState::Failed,
        _ => FeatureWorkerState::Disabled,
    }
}
