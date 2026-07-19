//! Shared calibration runtime statistics.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Calibration worker lifecycle classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalibrationWorkerState {
    /// Calibration is disabled in configuration.
    Disabled,
    /// Worker has not started.
    Idle,
    /// Worker is running.
    Running,
    /// Worker stopped cleanly.
    Stopped,
    /// Worker failed or exited unexpectedly.
    Failed,
}

impl CalibrationWorkerState {
    /// Stable API label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// Counters and latest calibration state.
#[derive(Debug, Default)]
pub struct CalibrationStats {
    enabled: AtomicBool,
    worker_state: AtomicU64,
    profile_id: Mutex<Option<String>>,
    profile_version: AtomicU64,
    raw_frames_submitted: AtomicU64,
    frames_calibrated: AtomicU64,
    calibration_failures: AtomicU64,
    latest_sequence: AtomicU64,
    latest_timestamp_nanos: AtomicU64,
    has_calibrated: AtomicBool,
    last_duration_ns: AtomicU64,
    average_duration_ns_bits: AtomicU64,
    average_count: AtomicU64,
    queue_depth: AtomicU64,
    last_error: Mutex<Option<String>>,
    last_warning: Mutex<Option<String>>,
    unexpected_exit: AtomicBool,
}

impl CalibrationStats {
    /// Creates empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps statistics for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Configures identity fields when the service starts.
    pub fn configure(&self, enabled: bool, profile_id: Option<&str>, profile_version: u32) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if let Ok(mut guard) = self.profile_id.lock() {
            *guard = profile_id.map(str::to_owned);
        }
        self.profile_version
            .store(u64::from(profile_version), Ordering::Relaxed);
        if enabled {
            self.set_worker_state(CalibrationWorkerState::Idle);
        } else {
            self.set_worker_state(CalibrationWorkerState::Disabled);
        }
    }

    /// Resets counters for a new service start.
    pub fn reset_counters(&self) {
        self.raw_frames_submitted.store(0, Ordering::Relaxed);
        self.frames_calibrated.store(0, Ordering::Relaxed);
        self.calibration_failures.store(0, Ordering::Relaxed);
        self.latest_sequence.store(0, Ordering::Relaxed);
        self.latest_timestamp_nanos.store(0, Ordering::Relaxed);
        self.has_calibrated.store(false, Ordering::Relaxed);
        self.last_duration_ns.store(0, Ordering::Relaxed);
        self.average_duration_ns_bits.store(0, Ordering::Relaxed);
        self.average_count.store(0, Ordering::Relaxed);
        self.queue_depth.store(0, Ordering::Relaxed);
        self.unexpected_exit.store(false, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.last_warning.lock() {
            *guard = None;
        }
    }

    /// Sets worker lifecycle state.
    pub fn set_worker_state(&self, state: CalibrationWorkerState) {
        self.worker_state
            .store(worker_state_to_u64(state), Ordering::Relaxed);
    }

    /// Returns worker lifecycle state.
    pub fn worker_state(&self) -> CalibrationWorkerState {
        u64_to_worker_state(self.worker_state.load(Ordering::Relaxed))
    }

    /// Whether calibration is enabled in configuration.
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

    /// Records that a raw frame was accepted by the calibration worker.
    pub fn record_submitted(&self) {
        self.raw_frames_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates approximate queue depth when available from the runtime.
    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);
    }

    /// Records a successful calibration.
    pub fn record_success(&self, sequence: u64, timestamp_nanos: u64, duration_ns: u64) {
        self.frames_calibrated.fetch_add(1, Ordering::Relaxed);
        self.latest_sequence.store(sequence, Ordering::Relaxed);
        self.latest_timestamp_nanos
            .store(timestamp_nanos, Ordering::Relaxed);
        self.has_calibrated.store(true, Ordering::Relaxed);
        self.last_duration_ns.store(duration_ns, Ordering::Relaxed);
        self.update_average_duration(duration_ns);
    }

    /// Records a calibration failure.
    pub fn record_failure(&self, error: impl Into<String>) {
        self.calibration_failures.fetch_add(1, Ordering::Relaxed);
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
        self.set_worker_state(CalibrationWorkerState::Failed);
    }

    /// Whether the worker exited unexpectedly.
    pub fn unexpected_exit(&self) -> bool {
        self.unexpected_exit.load(Ordering::Relaxed)
    }

    /// Raw frames submitted to calibration.
    pub fn raw_frames_submitted(&self) -> u64 {
        self.raw_frames_submitted.load(Ordering::Relaxed)
    }

    /// Successfully calibrated frames.
    pub fn frames_calibrated(&self) -> u64 {
        self.frames_calibrated.load(Ordering::Relaxed)
    }

    /// Calibration failures.
    pub fn calibration_failures(&self) -> u64 {
        self.calibration_failures.load(Ordering::Relaxed)
    }

    /// Latest calibrated sequence.
    pub fn latest_sequence(&self) -> Option<u64> {
        if self.has_calibrated.load(Ordering::Relaxed) {
            Some(self.latest_sequence.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Latest calibrated timestamp (nanos).
    pub fn latest_timestamp_nanos(&self) -> Option<u64> {
        if self.has_calibrated.load(Ordering::Relaxed) {
            Some(self.latest_timestamp_nanos.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Last calibration duration in nanoseconds.
    pub fn last_duration_ns(&self) -> Option<u64> {
        if self.has_calibrated.load(Ordering::Relaxed) {
            Some(self.last_duration_ns.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Running average calibration duration in nanoseconds.
    pub fn average_duration_ns(&self) -> Option<u64> {
        if self.average_count.load(Ordering::Relaxed) == 0 {
            None
        } else {
            Some(f64::from_bits(self.average_duration_ns_bits.load(Ordering::Relaxed)) as u64)
        }
    }

    /// Approximate queue depth tracked by submit/dequeue counters.
    pub fn queue_depth(&self) -> u64 {
        self.queue_depth.load(Ordering::Relaxed)
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

    /// Evaluates calibration-specific health impact.
    pub fn evaluate_health(&self) -> Option<crate::health::RuntimeHealth> {
        if !self.enabled() {
            return None;
        }
        if self.unexpected_exit() || self.worker_state() == CalibrationWorkerState::Failed {
            return Some(crate::health::RuntimeHealth::Failed);
        }
        let failures = self.calibration_failures();
        let successes = self.frames_calibrated();
        if failures >= 3 && failures > successes {
            return Some(crate::health::RuntimeHealth::Degraded);
        }
        None
    }

    fn update_average_duration(&self, duration_ns: u64) {
        let count = self.average_count.fetch_add(1, Ordering::Relaxed) + 1;
        let previous = f64::from_bits(self.average_duration_ns_bits.load(Ordering::Relaxed));
        let updated = previous + (duration_ns as f64 - previous) / count as f64;
        self.average_duration_ns_bits
            .store(updated.to_bits(), Ordering::Relaxed);
    }
}

fn worker_state_to_u64(state: CalibrationWorkerState) -> u64 {
    match state {
        CalibrationWorkerState::Disabled => 0,
        CalibrationWorkerState::Idle => 1,
        CalibrationWorkerState::Running => 2,
        CalibrationWorkerState::Stopped => 3,
        CalibrationWorkerState::Failed => 4,
    }
}

fn u64_to_worker_state(value: u64) -> CalibrationWorkerState {
    match value {
        1 => CalibrationWorkerState::Idle,
        2 => CalibrationWorkerState::Running,
        3 => CalibrationWorkerState::Stopped,
        4 => CalibrationWorkerState::Failed,
        _ => CalibrationWorkerState::Disabled,
    }
}
