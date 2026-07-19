//! Bounded in-memory signal snapshot and recent event history.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use aeryon_calibration::CalibratedCsiFrame;
use aeryon_csi::CsiFrame;
use aeryon_domain::Event;
use aeryon_dsp::{DspResultSink, DspWindowResult};

/// Default capacity for recent metadata events.
pub const DEFAULT_RECENT_EVENT_CAPACITY: usize = 100;

/// Single bounded store for latest engineering snapshots and recent events.
#[derive(Debug)]
pub struct SignalSnapshotStore {
    latest_raw: Mutex<Option<Arc<CsiFrame>>>,
    latest_calibrated: Mutex<Option<Arc<CalibratedCsiFrame>>>,
    latest_dsp: Mutex<Option<Arc<DspWindowResult>>>,
    recent_events: Mutex<VecDeque<Event>>,
    recent_capacity: usize,
}

impl Default for SignalSnapshotStore {
    fn default() -> Self {
        Self::new(DEFAULT_RECENT_EVENT_CAPACITY)
    }
}

impl SignalSnapshotStore {
    /// Creates a store with the given recent-event capacity.
    pub fn new(recent_capacity: usize) -> Self {
        Self {
            latest_raw: Mutex::new(None),
            latest_calibrated: Mutex::new(None),
            latest_dsp: Mutex::new(None),
            recent_events: Mutex::new(VecDeque::with_capacity(recent_capacity.max(1))),
            recent_capacity: recent_capacity.max(1),
        }
    }

    /// Wraps the store for shared ownership.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Records the latest raw and calibrated frames from a successful calibration.
    pub fn store_calibrated(&self, calibrated: Arc<CalibratedCsiFrame>) {
        if let Ok(mut guard) = self.latest_raw.lock() {
            *guard = Some(Arc::clone(calibrated.raw()));
        }
        if let Ok(mut guard) = self.latest_calibrated.lock() {
            *guard = Some(calibrated);
        }
    }

    /// Records the latest successful DSP result.
    pub fn store_dsp(&self, result: Arc<DspWindowResult>) {
        if let Ok(mut guard) = self.latest_dsp.lock() {
            *guard = Some(result);
        }
    }

    /// Appends a metadata event, evicting the oldest when capacity is reached.
    pub fn push_event(&self, event: Event) {
        if let Ok(mut guard) = self.recent_events.lock() {
            if guard.len() >= self.recent_capacity {
                guard.pop_front();
            }
            guard.push_back(event);
        }
    }

    /// Latest raw CSI frame, if any.
    pub fn latest_raw(&self) -> Option<Arc<CsiFrame>> {
        self.latest_raw.lock().ok().and_then(|guard| guard.clone())
    }

    /// Latest calibrated CSI frame, if any.
    pub fn latest_calibrated(&self) -> Option<Arc<CalibratedCsiFrame>> {
        self.latest_calibrated
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Latest DSP window result, if any.
    pub fn latest_dsp(&self) -> Option<Arc<DspWindowResult>> {
        self.latest_dsp.lock().ok().and_then(|guard| guard.clone())
    }

    /// Recent events in chronological order, optionally limited.
    pub fn recent_events(&self, limit: usize) -> Vec<Event> {
        let Ok(guard) = self.recent_events.lock() else {
            return Vec::new();
        };
        let limit = limit.min(guard.len());
        guard
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

impl DspResultSink for SignalSnapshotStore {
    fn store_result(&self, result: Arc<DspWindowResult>) {
        self.store_dsp(result);
    }
}
