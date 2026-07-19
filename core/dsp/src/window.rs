//! Immutable temporal CSI window assembled from calibrated frames.

use std::sync::Arc;

use aeryon_calibration::CalibratedCsiFrame;
use aeryon_domain::{SensorId, Timestamp};

use crate::errors::DspError;

/// Canonical sample layout label preserved by temporal windows.
pub const SAMPLE_LAYOUT: &str = "rx-tx-subcarrier";

/// Immutable ordered sequence of calibrated CSI frames for DSP.
#[derive(Debug, Clone, PartialEq)]
pub struct CsiWindow {
    sensor_id: SensorId,
    first_sequence: u64,
    last_sequence: u64,
    first_capture_timestamp: Timestamp,
    last_capture_timestamp: Timestamp,
    receive_antennas: u16,
    transmit_antennas: u16,
    subcarrier_indices: Vec<i16>,
    sample_layout: &'static str,
    frames: Vec<Arc<CalibratedCsiFrame>>,
    calibration_profile_id: String,
    calibration_profile_version: u32,
    window_id: u64,
}

impl CsiWindow {
    /// Validates and constructs an immutable temporal window.
    ///
    /// Frames are never reordered, padded, or repaired. All frames must share
    /// sensor identity, geometry, subcarrier indices, sample layout semantics,
    /// and calibration provenance. Sequences must be strictly increasing and
    /// capture timestamps must be monotonic.
    pub fn try_new(window_id: u64, frames: Vec<Arc<CalibratedCsiFrame>>) -> Result<Self, DspError> {
        if frames.is_empty() {
            return Err(DspError::InvalidWindow {
                message: "window frame count must be greater than zero".to_owned(),
            });
        }

        let first = &frames[0];
        let sensor_id = first.sensor_id();
        let receive_antennas = first.receive_antennas();
        let transmit_antennas = first.transmit_antennas();
        let subcarrier_indices = first.subcarrier_indices().to_vec();
        let calibration_profile_id = first.profile_id().to_owned();
        let calibration_profile_version = first.profile_version();

        for (index, frame) in frames.iter().enumerate() {
            if frame.sensor_id() != sensor_id {
                return Err(DspError::InvalidWindow {
                    message: format!(
                        "sensor mismatch at position {index}: expected {}, got {}",
                        sensor_id.value(),
                        frame.sensor_id().value()
                    ),
                });
            }
            if frame.receive_antennas() != receive_antennas
                || frame.transmit_antennas() != transmit_antennas
            {
                return Err(DspError::InvalidWindow {
                    message: format!("antenna geometry mismatch at position {index}"),
                });
            }
            if frame.subcarrier_indices() != subcarrier_indices.as_slice() {
                return Err(DspError::InvalidWindow {
                    message: format!("subcarrier indices mismatch at position {index}"),
                });
            }
            if frame.profile_id() != calibration_profile_id
                || frame.profile_version() != calibration_profile_version
            {
                return Err(DspError::InvalidWindow {
                    message: format!("calibration profile mismatch at position {index}"),
                });
            }
            for (sample_index, sample) in frame.samples().iter().enumerate() {
                if !sample.re.is_finite() || !sample.im.is_finite() {
                    return Err(DspError::InvalidWindow {
                        message: format!(
                            "non-finite calibrated sample at position {index}, sample {sample_index}"
                        ),
                    });
                }
            }
            if index > 0 {
                let previous = &frames[index - 1];
                if frame.sequence() <= previous.sequence() {
                    return Err(DspError::InvalidWindow {
                        message: format!(
                            "sequence numbers must be strictly increasing ({} then {})",
                            previous.sequence(),
                            frame.sequence()
                        ),
                    });
                }
                if frame.capture_timestamp().as_nanos() < previous.capture_timestamp().as_nanos() {
                    return Err(DspError::InvalidWindow {
                        message: format!(
                            "capture timestamps must be monotonic ({} then {})",
                            previous.capture_timestamp().as_nanos(),
                            frame.capture_timestamp().as_nanos()
                        ),
                    });
                }
            }
        }

        let last = frames.last().expect("non-empty");
        Ok(Self {
            sensor_id,
            first_sequence: first.sequence(),
            last_sequence: last.sequence(),
            first_capture_timestamp: first.capture_timestamp(),
            last_capture_timestamp: last.capture_timestamp(),
            receive_antennas,
            transmit_antennas,
            subcarrier_indices,
            sample_layout: SAMPLE_LAYOUT,
            frames,
            calibration_profile_id,
            calibration_profile_version,
            window_id,
        })
    }

    /// Window identity assigned by the assembler.
    pub fn window_id(&self) -> u64 {
        self.window_id
    }

    /// Shared sensor identity.
    pub fn sensor_id(&self) -> SensorId {
        self.sensor_id
    }

    /// First sequence in the window.
    pub fn first_sequence(&self) -> u64 {
        self.first_sequence
    }

    /// Last sequence in the window.
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    /// First capture timestamp.
    pub fn first_capture_timestamp(&self) -> Timestamp {
        self.first_capture_timestamp
    }

    /// Last capture timestamp.
    pub fn last_capture_timestamp(&self) -> Timestamp {
        self.last_capture_timestamp
    }

    /// Number of frames in the window.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Receive antenna count.
    pub fn receive_antennas(&self) -> u16 {
        self.receive_antennas
    }

    /// Transmit antenna count.
    pub fn transmit_antennas(&self) -> u16 {
        self.transmit_antennas
    }

    /// Ordered subcarrier indices.
    pub fn subcarrier_indices(&self) -> &[i16] {
        &self.subcarrier_indices
    }

    /// Canonical sample layout label.
    pub fn sample_layout(&self) -> &'static str {
        self.sample_layout
    }

    /// Shared calibrated frames in capture order.
    pub fn frames(&self) -> &[Arc<CalibratedCsiFrame>] {
        &self.frames
    }

    /// Calibration profile identity shared by every frame.
    pub fn calibration_profile_id(&self) -> &str {
        &self.calibration_profile_id
    }

    /// Calibration profile version shared by every frame.
    pub fn calibration_profile_version(&self) -> u32 {
        self.calibration_profile_version
    }
}
