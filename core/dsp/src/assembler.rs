//! Stateful overlapping temporal window assembler.

use std::collections::VecDeque;
use std::sync::Arc;

use aeryon_calibration::CalibratedCsiFrame;
use aeryon_domain::SensorId;

use crate::errors::{DspError, DspFailureCode};
use crate::window::CsiWindow;

/// Configuration for [`WindowAssembler`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssemblerConfig {
    /// Number of frames in each emitted window (must be > 1).
    pub window_size_frames: usize,
    /// Frame advance between consecutive windows (must be > 0 and ≤ window size).
    pub hop_size_frames: usize,
    /// Maximum calibrated frames retained for future overlapping windows.
    pub queue_capacity: usize,
    /// Maximum allowed gap between consecutive accepted sequences.
    pub maximum_sequence_gap: u64,
    /// Maximum allowed relative deviation of intervals from the median.
    ///
    /// Jitter metric: `max_i |interval_i − median| / median`. Values must be
    /// finite and non-negative. Spectral analysis may reject windows that
    /// exceed this tolerance.
    pub timestamp_jitter_tolerance: f64,
}

impl AssemblerConfig {
    /// Validates assembler configuration invariants.
    pub fn validate(&self) -> Result<(), DspError> {
        if self.window_size_frames <= 1 {
            return Err(DspError::InvalidConfig {
                message: "window_size_frames must be greater than one".to_owned(),
            });
        }
        if self.hop_size_frames == 0 {
            return Err(DspError::InvalidConfig {
                message: "hop_size_frames must be greater than zero".to_owned(),
            });
        }
        if self.hop_size_frames > self.window_size_frames {
            return Err(DspError::InvalidConfig {
                message: "hop_size_frames must not exceed window_size_frames".to_owned(),
            });
        }
        if self.queue_capacity == 0 {
            return Err(DspError::InvalidConfig {
                message: "queue_capacity must be greater than zero".to_owned(),
            });
        }
        if self.queue_capacity < self.window_size_frames {
            return Err(DspError::InvalidConfig {
                message: "queue_capacity must be at least window_size_frames".to_owned(),
            });
        }
        if !self.timestamp_jitter_tolerance.is_finite() || self.timestamp_jitter_tolerance < 0.0 {
            return Err(DspError::InvalidConfig {
                message: "timestamp_jitter_tolerance must be finite and non-negative".to_owned(),
            });
        }
        Ok(())
    }
}

/// Counters exposed for runtime health and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AssemblerCounters {
    /// Calibrated frames accepted into the buffer.
    pub frames_accepted: u64,
    /// Frames rejected with a typed error.
    pub frames_rejected: u64,
    /// Complete windows emitted.
    pub windows_emitted: u64,
}

/// Stateful assembler that emits overlapping [`CsiWindow`] values.
#[derive(Debug)]
pub struct WindowAssembler {
    config: AssemblerConfig,
    buffer: VecDeque<Arc<CalibratedCsiFrame>>,
    next_window_id: u64,
    expected_sensor: Option<SensorId>,
    expected_rx: Option<u16>,
    expected_tx: Option<u16>,
    expected_subcarriers: Option<Vec<i16>>,
    expected_profile_id: Option<String>,
    expected_profile_version: Option<u32>,
    last_sequence: Option<u64>,
    last_capture_nanos: Option<u64>,
    counters: AssemblerCounters,
}

impl WindowAssembler {
    /// Creates an assembler from validated configuration.
    pub fn try_new(config: AssemblerConfig) -> Result<Self, DspError> {
        config.validate()?;
        Ok(Self {
            config,
            buffer: VecDeque::with_capacity(config.window_size_frames),
            next_window_id: 1,
            expected_sensor: None,
            expected_rx: None,
            expected_tx: None,
            expected_subcarriers: None,
            expected_profile_id: None,
            expected_profile_version: None,
            last_sequence: None,
            last_capture_nanos: None,
            counters: AssemblerCounters::default(),
        })
    }

    /// Assembler configuration.
    pub fn config(&self) -> &AssemblerConfig {
        &self.config
    }

    /// Runtime counters.
    pub fn counters(&self) -> AssemblerCounters {
        self.counters
    }

    /// Number of frames currently retained for future windows.
    pub fn buffered_frames(&self) -> usize {
        self.buffer.len()
    }

    /// Clears retained frames and geometry expectations while preserving configuration.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.expected_sensor = None;
        self.expected_rx = None;
        self.expected_tx = None;
        self.expected_subcarriers = None;
        self.expected_profile_id = None;
        self.expected_profile_version = None;
        self.last_sequence = None;
        self.last_capture_nanos = None;
    }

    /// Accepts the next calibrated frame and optionally emits a completed window.
    ///
    /// Frames are never silently dropped. Incompatible or non-monotonic input
    /// returns a typed error and leaves prior buffer contents untouched.
    pub fn push(&mut self, frame: Arc<CalibratedCsiFrame>) -> Result<Option<CsiWindow>, DspError> {
        self.validate_candidate(&frame).inspect_err(|_| {
            self.counters.frames_rejected = self.counters.frames_rejected.saturating_add(1);
        })?;

        if self.buffer.len() >= self.config.queue_capacity {
            self.counters.frames_rejected = self.counters.frames_rejected.saturating_add(1);
            return Err(DspError::AssemblerRejected {
                frame_id: Some(frame.raw_frame_id()),
                sensor_id: Some(frame.sensor_id()),
                sequence: Some(frame.sequence()),
                message: "assembler queue_capacity exceeded; refuse silent frame drop".to_owned(),
                code: DspFailureCode::InvalidConfig,
            });
        }

        self.last_sequence = Some(frame.sequence());
        self.last_capture_nanos = Some(frame.capture_timestamp().as_nanos());
        if self.expected_sensor.is_none() {
            self.expected_sensor = Some(frame.sensor_id());
            self.expected_rx = Some(frame.receive_antennas());
            self.expected_tx = Some(frame.transmit_antennas());
            self.expected_subcarriers = Some(frame.subcarrier_indices().to_vec());
            self.expected_profile_id = Some(frame.profile_id().to_owned());
            self.expected_profile_version = Some(frame.profile_version());
        }

        self.buffer.push_back(frame);
        self.counters.frames_accepted = self.counters.frames_accepted.saturating_add(1);

        if self.buffer.len() < self.config.window_size_frames {
            return Ok(None);
        }

        let window_frames: Vec<_> = self
            .buffer
            .iter()
            .take(self.config.window_size_frames)
            .cloned()
            .collect();
        let window_id = self.next_window_id;
        self.next_window_id = self.next_window_id.saturating_add(1);
        let window = CsiWindow::try_new(window_id, window_frames)?;

        for _ in 0..self.config.hop_size_frames {
            self.buffer.pop_front();
        }

        // After hopping, refresh monotonic trackers from retained tail when present.
        if let Some(front) = self.buffer.back() {
            self.last_sequence = Some(front.sequence());
            self.last_capture_nanos = Some(front.capture_timestamp().as_nanos());
        } else {
            self.last_sequence = None;
            self.last_capture_nanos = None;
        }

        self.counters.windows_emitted = self.counters.windows_emitted.saturating_add(1);
        Ok(Some(window))
    }

    fn validate_candidate(&self, frame: &CalibratedCsiFrame) -> Result<(), DspError> {
        let reject = |code: DspFailureCode, message: String| DspError::AssemblerRejected {
            frame_id: Some(frame.raw_frame_id()),
            sensor_id: Some(frame.sensor_id()),
            sequence: Some(frame.sequence()),
            message,
            code,
        };

        for sample in frame.samples() {
            if !sample.re.is_finite() || !sample.im.is_finite() {
                return Err(reject(
                    DspFailureCode::NonFinite,
                    "calibrated frame contains non-finite samples".to_owned(),
                ));
            }
        }

        if let Some(sensor) = self.expected_sensor {
            if frame.sensor_id() != sensor {
                return Err(reject(
                    DspFailureCode::SensorMismatch,
                    format!(
                        "sensor mismatch: expected {}, got {}",
                        sensor.value(),
                        frame.sensor_id().value()
                    ),
                ));
            }
        }
        if let (Some(rx), Some(tx)) = (self.expected_rx, self.expected_tx) {
            if frame.receive_antennas() != rx || frame.transmit_antennas() != tx {
                return Err(reject(
                    DspFailureCode::GeometryMismatch,
                    "antenna geometry mismatch".to_owned(),
                ));
            }
        }
        if let Some(indices) = &self.expected_subcarriers {
            if frame.subcarrier_indices() != indices.as_slice() {
                return Err(reject(
                    DspFailureCode::GeometryMismatch,
                    "subcarrier indices mismatch".to_owned(),
                ));
            }
        }
        if let (Some(profile_id), Some(version)) = (
            self.expected_profile_id.as_deref(),
            self.expected_profile_version,
        ) {
            if frame.profile_id() != profile_id || frame.profile_version() != version {
                return Err(reject(
                    DspFailureCode::CalibrationProfileMismatch,
                    "calibration profile mismatch".to_owned(),
                ));
            }
        }

        if let Some(previous) = self.last_sequence {
            if frame.sequence() <= previous {
                return Err(reject(
                    DspFailureCode::NonMonotonicSequence,
                    format!(
                        "non-monotonic sequence: previous {previous}, got {}",
                        frame.sequence()
                    ),
                ));
            }
            let gap = frame.sequence() - previous;
            if gap > self.config.maximum_sequence_gap.saturating_add(1) {
                // gap == 1 means consecutive (seq n then n+1). Allowed gap in
                // missing frames is maximum_sequence_gap.
                return Err(reject(
                    DspFailureCode::SequenceGap,
                    format!(
                        "sequence gap {gap} exceeds maximum {}",
                        self.config.maximum_sequence_gap.saturating_add(1)
                    ),
                ));
            }
        }

        if let Some(previous_ts) = self.last_capture_nanos {
            let current = frame.capture_timestamp().as_nanos();
            if current < previous_ts {
                return Err(reject(
                    DspFailureCode::NonMonotonicTimestamp,
                    format!(
                        "non-monotonic capture timestamp: previous {previous_ts}, got {current}"
                    ),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_calibration::{CalibrationPipeline, baseline_csi_v1};
    use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};

    fn calibrated(
        sequence: u64,
        capture_nanos: u64,
        sensor: u64,
        rx: u16,
        tx: u16,
        subcarriers: &[i16],
        profile_override: Option<(&str, u32)>,
    ) -> Arc<CalibratedCsiFrame> {
        let n_sc = subcarriers.len();
        let samples = vec![ComplexSample::new(1.0, 0.0); usize::from(rx) * usize::from(tx) * n_sc];
        let metadata = FrameMetadata {
            frame_id: FrameId::new(sequence + 1),
            sensor_id: SensorId::new(sensor),
            timestamp: Timestamp::from_nanos(capture_nanos),
            sequence,
            mission_id: None,
            metadata: Metadata::new(),
        };
        let raw = CsiFrame::try_new(
            metadata,
            Timestamp::from_nanos(capture_nanos),
            Some(5_180_000_000.0),
            Some(20_000_000.0),
            rx,
            tx,
            subcarriers.to_vec(),
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("raw");
        let mut profile = baseline_csi_v1();
        if let Some((id, version)) = profile_override {
            profile.id = id.to_owned();
            profile.version = version;
        }
        let pipeline = CalibrationPipeline::try_new(profile).expect("pipeline");
        Arc::new(pipeline.calibrate(Arc::new(raw)).expect("calibrated"))
    }

    fn default_config() -> AssemblerConfig {
        AssemblerConfig {
            window_size_frames: 4,
            hop_size_frames: 2,
            queue_capacity: 8,
            maximum_sequence_gap: 1,
            timestamp_jitter_tolerance: 0.10,
        }
    }

    #[test]
    fn emits_exact_and_overlapping_windows() {
        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        let sc = [-2, -1, 0, 1];
        let mut windows = Vec::new();
        for sequence in 0..8 {
            let frame = calibrated(sequence, sequence * 100, 2, 2, 1, &sc, None);
            if let Some(window) = assembler.push(frame).expect("push") {
                windows.push(window);
            }
        }
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].first_sequence(), 0);
        assert_eq!(windows[0].last_sequence(), 3);
        assert_eq!(windows[1].first_sequence(), 2);
        assert_eq!(windows[1].last_sequence(), 5);
        assert_eq!(windows[2].first_sequence(), 4);
        assert_eq!(windows[2].last_sequence(), 7);
        assert!(assembler.buffered_frames() <= default_config().window_size_frames);
    }

    #[test]
    fn rejects_geometry_and_profile_mismatches() {
        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        let sc = [-1, 0, 1];
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let geo = assembler
            .push(calibrated(1, 100, 2, 1, 1, &sc, None))
            .expect_err("geometry");
        assert_eq!(geo.code(), DspFailureCode::GeometryMismatch);

        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let sc2 = [-2, -1, 0];
        let sc_err = assembler
            .push(calibrated(1, 100, 2, 2, 1, &sc2, None))
            .expect_err("subcarriers");
        assert_eq!(sc_err.code(), DspFailureCode::GeometryMismatch);

        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let sensor = assembler
            .push(calibrated(1, 100, 9, 2, 1, &sc, None))
            .expect_err("sensor");
        assert_eq!(sensor.code(), DspFailureCode::SensorMismatch);

        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let profile = assembler
            .push(calibrated(1, 100, 2, 2, 1, &sc, Some(("other", 1))))
            .expect_err("profile");
        assert_eq!(profile.code(), DspFailureCode::CalibrationProfileMismatch);
    }

    #[test]
    fn rejects_sequence_and_timestamp_faults() {
        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        let sc = [-1, 0, 1];
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let mono = assembler
            .push(calibrated(0, 100, 2, 2, 1, &sc, None))
            .expect_err("sequence");
        assert_eq!(mono.code(), DspFailureCode::NonMonotonicSequence);

        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        assembler
            .push(calibrated(0, 0, 2, 2, 1, &sc, None))
            .expect("first");
        let gap = assembler
            .push(calibrated(3, 100, 2, 2, 1, &sc, None))
            .expect_err("gap");
        assert_eq!(gap.code(), DspFailureCode::SequenceGap);

        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        assembler
            .push(calibrated(0, 200, 2, 2, 1, &sc, None))
            .expect("first");
        let ts = assembler
            .push(calibrated(1, 100, 2, 2, 1, &sc, None))
            .expect_err("timestamp");
        assert_eq!(ts.code(), DspFailureCode::NonMonotonicTimestamp);
    }

    #[test]
    fn reset_clears_buffer_deterministically() {
        let mut assembler = WindowAssembler::try_new(default_config()).expect("assembler");
        let sc = [-1, 0, 1];
        for sequence in 0..3 {
            let _ = assembler
                .push(calibrated(sequence, sequence * 100, 2, 2, 1, &sc, None))
                .expect("push");
        }
        assert_eq!(assembler.buffered_frames(), 3);
        assembler.reset();
        assert_eq!(assembler.buffered_frames(), 0);
        let window = assembler
            .push(calibrated(10, 1000, 2, 2, 1, &sc, None))
            .expect("after reset");
        assert!(window.is_none());
    }
}
