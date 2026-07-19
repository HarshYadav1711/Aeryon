//! Shared CSI replay statistics updated by the replay plugin.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Finite replay completion classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsiReplayCompletion {
    /// Replay has not started or is idle.
    Idle,
    /// Replay is actively producing frames.
    Active,
    /// A finite fixture pass completed without error.
    Completed,
    /// Replay failed.
    Failed,
    /// Replay was stopped by lifecycle shutdown.
    Stopped,
}

impl CsiReplayCompletion {
    /// Stable API label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

/// Counters and latest CSI replay state.
#[derive(Debug, Default)]
pub struct CsiReplayStats {
    frames_read: AtomicU64,
    frames_accepted: AtomicU64,
    frames_rejected: AtomicU64,
    latest_sequence: AtomicU64,
    latest_frame_nanos: AtomicU64,
    has_frame: AtomicBool,
    receive_antennas: AtomicU64,
    transmit_antennas: AtomicU64,
    subcarrier_count: AtomicU64,
    center_frequency_bits: AtomicU64,
    has_center_frequency: AtomicBool,
    bandwidth_bits: AtomicU64,
    has_bandwidth: AtomicBool,
    completion: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl CsiReplayStats {
    /// Creates empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps statistics for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Resets counters for a new plugin start.
    pub fn reset(&self) {
        self.frames_read.store(0, Ordering::Relaxed);
        self.frames_accepted.store(0, Ordering::Relaxed);
        self.frames_rejected.store(0, Ordering::Relaxed);
        self.latest_sequence.store(0, Ordering::Relaxed);
        self.latest_frame_nanos.store(0, Ordering::Relaxed);
        self.has_frame.store(false, Ordering::Relaxed);
        self.receive_antennas.store(0, Ordering::Relaxed);
        self.transmit_antennas.store(0, Ordering::Relaxed);
        self.subcarrier_count.store(0, Ordering::Relaxed);
        self.has_center_frequency.store(false, Ordering::Relaxed);
        self.has_bandwidth.store(false, Ordering::Relaxed);
        self.set_completion(CsiReplayCompletion::Idle);
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
    }

    /// Records that a fixture record was read from disk.
    pub fn record_read(&self) {
        self.frames_read.fetch_add(1, Ordering::Relaxed);
    }

    /// Records that a validated frame was accepted and published.
    #[allow(clippy::too_many_arguments)]
    pub fn record_accepted(
        &self,
        sequence: u64,
        timestamp_nanos: u64,
        receive_antennas: u16,
        transmit_antennas: u16,
        subcarrier_count: usize,
        center_frequency_hz: Option<f64>,
        bandwidth_hz: Option<f64>,
    ) {
        self.frames_accepted.fetch_add(1, Ordering::Relaxed);
        self.latest_sequence.store(sequence, Ordering::Relaxed);
        self.latest_frame_nanos
            .store(timestamp_nanos, Ordering::Relaxed);
        self.has_frame.store(true, Ordering::Relaxed);
        self.receive_antennas
            .store(u64::from(receive_antennas), Ordering::Relaxed);
        self.transmit_antennas
            .store(u64::from(transmit_antennas), Ordering::Relaxed);
        self.subcarrier_count
            .store(subcarrier_count as u64, Ordering::Relaxed);
        if let Some(frequency) = center_frequency_hz {
            self.center_frequency_bits
                .store(frequency.to_bits(), Ordering::Relaxed);
            self.has_center_frequency.store(true, Ordering::Relaxed);
        }
        if let Some(bandwidth) = bandwidth_hz {
            self.bandwidth_bits
                .store(bandwidth.to_bits(), Ordering::Relaxed);
            self.has_bandwidth.store(true, Ordering::Relaxed);
        }
    }

    /// Records a rejected fixture frame.
    pub fn record_rejected(&self, error: impl Into<String>) {
        self.frames_rejected.fetch_add(1, Ordering::Relaxed);
        self.set_last_error(error);
    }

    /// Sets the completion/state classification.
    pub fn set_completion(&self, completion: CsiReplayCompletion) {
        self.completion
            .store(completion_to_u64(completion), Ordering::Relaxed);
    }

    /// Returns the completion classification.
    pub fn completion(&self) -> CsiReplayCompletion {
        u64_to_completion(self.completion.load(Ordering::Relaxed))
    }

    /// Stores the last replay error message.
    pub fn set_last_error(&self, error: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(error.into());
        }
    }

    /// Returns the last replay error, if any.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|guard| guard.clone())
    }

    /// Frames read from the fixture.
    pub fn frames_read(&self) -> u64 {
        self.frames_read.load(Ordering::Relaxed)
    }

    /// Frames accepted and published.
    pub fn frames_accepted(&self) -> u64 {
        self.frames_accepted.load(Ordering::Relaxed)
    }

    /// Frames rejected due to validation failures.
    pub fn frames_rejected(&self) -> u64 {
        self.frames_rejected.load(Ordering::Relaxed)
    }

    /// Latest accepted sequence.
    pub fn latest_sequence(&self) -> Option<u64> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.latest_sequence.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Latest accepted frame timestamp (nanos since epoch).
    pub fn latest_frame_nanos(&self) -> Option<u64> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.latest_frame_nanos.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// Latest receive antenna count.
    pub fn receive_antennas(&self) -> Option<u16> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.receive_antennas.load(Ordering::Relaxed) as u16)
        } else {
            None
        }
    }

    /// Latest transmit antenna count.
    pub fn transmit_antennas(&self) -> Option<u16> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.transmit_antennas.load(Ordering::Relaxed) as u16)
        } else {
            None
        }
    }

    /// Latest subcarrier count.
    pub fn subcarrier_count(&self) -> Option<u16> {
        if self.has_frame.load(Ordering::Relaxed) {
            Some(self.subcarrier_count.load(Ordering::Relaxed) as u16)
        } else {
            None
        }
    }

    /// Latest center frequency in hertz, when present on an accepted frame.
    pub fn center_frequency_hz(&self) -> Option<f64> {
        if self.has_center_frequency.load(Ordering::Relaxed) {
            Some(f64::from_bits(
                self.center_frequency_bits.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    /// Latest bandwidth in hertz, when present on an accepted frame.
    pub fn bandwidth_hz(&self) -> Option<f64> {
        if self.has_bandwidth.load(Ordering::Relaxed) {
            Some(f64::from_bits(self.bandwidth_bits.load(Ordering::Relaxed)))
        } else {
            None
        }
    }
}

fn completion_to_u64(completion: CsiReplayCompletion) -> u64 {
    match completion {
        CsiReplayCompletion::Idle => 0,
        CsiReplayCompletion::Active => 1,
        CsiReplayCompletion::Completed => 2,
        CsiReplayCompletion::Failed => 3,
        CsiReplayCompletion::Stopped => 4,
    }
}

fn u64_to_completion(value: u64) -> CsiReplayCompletion {
    match value {
        1 => CsiReplayCompletion::Active,
        2 => CsiReplayCompletion::Completed,
        3 => CsiReplayCompletion::Failed,
        4 => CsiReplayCompletion::Stopped,
        _ => CsiReplayCompletion::Idle,
    }
}
