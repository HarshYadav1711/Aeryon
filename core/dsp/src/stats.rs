//! Shared DSP runtime statistics.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// DSP worker lifecycle classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DspWorkerState {
    /// DSP is disabled in configuration.
    Disabled,
    /// Worker has not processed input yet.
    Idle,
    /// Worker is running.
    Running,
    /// Finite input completed cleanly; no further frames expected.
    Completed,
    /// Worker stopped during graceful shutdown.
    Stopped,
    /// Worker failed or exited unexpectedly.
    Failed,
}

impl DspWorkerState {
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

/// Counters and latest DSP state.
#[derive(Debug, Default)]
pub struct DspStats {
    enabled: AtomicBool,
    worker_state: AtomicU64,
    profile_id: Mutex<Option<String>>,
    profile_version: AtomicU64,
    window_size_frames: AtomicU64,
    hop_size_frames: AtomicU64,
    calibrated_frames_received: AtomicU64,
    windows_emitted: AtomicU64,
    windows_rejected: AtomicU64,
    latest_first_sequence: AtomicU64,
    latest_last_sequence: AtomicU64,
    has_window: AtomicBool,
    latest_window_timestamp_nanos: AtomicU64,
    last_duration_ns: AtomicU64,
    average_duration_ns_bits: AtomicU64,
    average_count: AtomicU64,
    effective_sample_rate_bits: AtomicU64,
    has_sample_rate: AtomicBool,
    latest_jitter_bits: AtomicU64,
    has_jitter: AtomicBool,
    latest_dominant_hz_bits: AtomicU64,
    has_dominant: AtomicBool,
    last_error: Mutex<Option<String>>,
    last_warning: Mutex<Option<String>>,
    unexpected_exit: AtomicBool,
    consecutive_failures: AtomicU64,
}

impl DspStats {
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
        window_size_frames: usize,
        hop_size_frames: usize,
    ) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if let Ok(mut guard) = self.profile_id.lock() {
            *guard = profile_id.map(str::to_owned);
        }
        self.profile_version
            .store(u64::from(profile_version), Ordering::Relaxed);
        self.window_size_frames
            .store(window_size_frames as u64, Ordering::Relaxed);
        self.hop_size_frames
            .store(hop_size_frames as u64, Ordering::Relaxed);
        if enabled {
            self.set_worker_state(DspWorkerState::Idle);
        } else {
            self.set_worker_state(DspWorkerState::Disabled);
        }
    }

    /// Resets counters for a new service start.
    pub fn reset_counters(&self) {
        self.calibrated_frames_received.store(0, Ordering::Relaxed);
        self.windows_emitted.store(0, Ordering::Relaxed);
        self.windows_rejected.store(0, Ordering::Relaxed);
        self.latest_first_sequence.store(0, Ordering::Relaxed);
        self.latest_last_sequence.store(0, Ordering::Relaxed);
        self.has_window.store(false, Ordering::Relaxed);
        self.latest_window_timestamp_nanos
            .store(0, Ordering::Relaxed);
        self.last_duration_ns.store(0, Ordering::Relaxed);
        self.average_duration_ns_bits.store(0, Ordering::Relaxed);
        self.average_count.store(0, Ordering::Relaxed);
        self.effective_sample_rate_bits.store(0, Ordering::Relaxed);
        self.has_sample_rate.store(false, Ordering::Relaxed);
        self.latest_jitter_bits.store(0, Ordering::Relaxed);
        self.has_jitter.store(false, Ordering::Relaxed);
        self.latest_dominant_hz_bits.store(0, Ordering::Relaxed);
        self.has_dominant.store(false, Ordering::Relaxed);
        self.unexpected_exit.store(false, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.last_warning.lock() {
            *guard = None;
        }
    }

    /// Sets worker lifecycle state.
    pub fn set_worker_state(&self, state: DspWorkerState) {
        self.worker_state
            .store(worker_state_to_u64(state), Ordering::Relaxed);
    }

    /// Returns worker lifecycle state.
    pub fn worker_state(&self) -> DspWorkerState {
        u64_to_worker_state(self.worker_state.load(Ordering::Relaxed))
    }

    /// Whether DSP is enabled in configuration.
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

    /// Configured window size.
    pub fn window_size_frames(&self) -> usize {
        self.window_size_frames.load(Ordering::Relaxed) as usize
    }

    /// Configured hop size.
    pub fn hop_size_frames(&self) -> usize {
        self.hop_size_frames.load(Ordering::Relaxed) as usize
    }

    /// Records receipt of one calibrated frame.
    pub fn record_frame_received(&self) {
        self.calibrated_frames_received
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Records a successfully processed window.
    #[allow(clippy::too_many_arguments)]
    pub fn record_window_success(
        &self,
        first_sequence: u64,
        last_sequence: u64,
        window_timestamp_nanos: u64,
        duration_ns: u64,
        sample_rate_hz: f64,
        jitter: f64,
        dominant_hz: Option<f64>,
    ) {
        self.windows_emitted.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.latest_first_sequence
            .store(first_sequence, Ordering::Relaxed);
        self.latest_last_sequence
            .store(last_sequence, Ordering::Relaxed);
        self.latest_window_timestamp_nanos
            .store(window_timestamp_nanos, Ordering::Relaxed);
        self.has_window.store(true, Ordering::Relaxed);
        self.last_duration_ns.store(duration_ns, Ordering::Relaxed);
        self.update_average_duration(duration_ns);
        self.effective_sample_rate_bits
            .store(sample_rate_hz.to_bits(), Ordering::Relaxed);
        self.has_sample_rate.store(true, Ordering::Relaxed);
        self.latest_jitter_bits
            .store(jitter.to_bits(), Ordering::Relaxed);
        self.has_jitter.store(true, Ordering::Relaxed);
        if let Some(dominant) = dominant_hz {
            self.latest_dominant_hz_bits
                .store(dominant.to_bits(), Ordering::Relaxed);
            self.has_dominant.store(true, Ordering::Relaxed);
        } else {
            self.has_dominant.store(false, Ordering::Relaxed);
        }
    }

    /// Records a rejected or failed window.
    pub fn record_window_failure(&self, error: impl Into<String>) {
        self.windows_rejected.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
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
        self.set_worker_state(DspWorkerState::Failed);
    }

    /// Whether the worker exited unexpectedly.
    pub fn unexpected_exit(&self) -> bool {
        self.unexpected_exit.load(Ordering::Relaxed)
    }

    /// Calibrated frames received.
    pub fn calibrated_frames_received(&self) -> u64 {
        self.calibrated_frames_received.load(Ordering::Relaxed)
    }

    /// Windows emitted.
    pub fn windows_emitted(&self) -> u64 {
        self.windows_emitted.load(Ordering::Relaxed)
    }

    /// Windows rejected / failed.
    pub fn windows_rejected(&self) -> u64 {
        self.windows_rejected.load(Ordering::Relaxed)
    }

    /// Latest window sequence range.
    pub fn latest_sequence_range(&self) -> Option<(u64, u64)> {
        if self.has_window.load(Ordering::Relaxed) {
            Some((
                self.latest_first_sequence.load(Ordering::Relaxed),
                self.latest_last_sequence.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Latest window processing timestamp (nanos).
    pub fn latest_window_timestamp_nanos(&self) -> Option<u64> {
        if self.has_window.load(Ordering::Relaxed) {
            Some(self.latest_window_timestamp_nanos.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Last processing duration in nanoseconds.
    pub fn last_duration_ns(&self) -> Option<u64> {
        if self.has_window.load(Ordering::Relaxed) {
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

    /// Latest effective sample rate in hertz.
    pub fn effective_sample_rate_hz(&self) -> Option<f64> {
        if self.has_sample_rate.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.effective_sample_rate_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Latest timestamp jitter metric.
    pub fn latest_timestamp_jitter(&self) -> Option<f64> {
        if self.has_jitter.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.latest_jitter_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Latest dominant non-DC frequency in hertz.
    pub fn latest_dominant_non_dc_hz(&self) -> Option<f64> {
        if self.has_dominant.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.latest_dominant_hz_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
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

    /// Consecutive failure count.
    pub fn consecutive_failures(&self) -> u64 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    fn update_average_duration(&self, duration_ns: u64) {
        let count = self.average_count.fetch_add(1, Ordering::Relaxed) + 1;
        let previous = f64::from_bits(self.average_duration_ns_bits.load(Ordering::Relaxed));
        let updated = previous + (duration_ns as f64 - previous) / count as f64;
        self.average_duration_ns_bits
            .store(updated.to_bits(), Ordering::Relaxed);
    }
}

fn worker_state_to_u64(state: DspWorkerState) -> u64 {
    match state {
        DspWorkerState::Disabled => 0,
        DspWorkerState::Idle => 1,
        DspWorkerState::Running => 2,
        DspWorkerState::Completed => 3,
        DspWorkerState::Stopped => 4,
        DspWorkerState::Failed => 5,
    }
}

fn u64_to_worker_state(value: u64) -> DspWorkerState {
    match value {
        1 => DspWorkerState::Idle,
        2 => DspWorkerState::Running,
        3 => DspWorkerState::Completed,
        4 => DspWorkerState::Stopped,
        5 => DspWorkerState::Failed,
        _ => DspWorkerState::Disabled,
    }
}
