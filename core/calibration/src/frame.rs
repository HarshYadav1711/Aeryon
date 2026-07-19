//! Immutable calibrated CSI frame and provenance.

use std::sync::Arc;

use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
use aeryon_domain::{FrameId, SensorId, Timestamp};

use crate::report::CalibrationReport;

/// Immutable output of a successful calibration run.
///
/// The original raw [`CsiFrame`] is retained via [`Arc`] for comparison, replay
/// debugging, provenance, and regression testing. Raw frames are never mutated.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibratedCsiFrame {
    raw: Arc<CsiFrame>,
    samples: Vec<ComplexSample>,
    profile_id: String,
    profile_version: u32,
    calibrated_at: Timestamp,
    report: CalibrationReport,
}

impl CalibratedCsiFrame {
    /// Constructs a calibrated frame after pipeline validation.
    pub(crate) fn new(
        raw: Arc<CsiFrame>,
        samples: Vec<ComplexSample>,
        profile_id: String,
        profile_version: u32,
        calibrated_at: Timestamp,
        report: CalibrationReport,
    ) -> Self {
        Self {
            raw,
            samples,
            profile_id,
            profile_version,
            calibrated_at,
            report,
        }
    }

    /// Shared ownership of the original raw frame.
    pub fn raw(&self) -> &Arc<CsiFrame> {
        &self.raw
    }

    /// Raw frame identifier.
    pub fn raw_frame_id(&self) -> FrameId {
        self.raw.frame_id()
    }

    /// Sensor identifier.
    pub fn sensor_id(&self) -> SensorId {
        self.raw.sensor_id()
    }

    /// Monotonic sequence number.
    pub fn sequence(&self) -> u64 {
        self.raw.sequence()
    }

    /// Capture / acquisition timestamp from the raw frame.
    pub fn capture_timestamp(&self) -> Timestamp {
        self.raw.capture_timestamp()
    }

    /// Replay or receive timestamp from the raw frame.
    pub fn receive_timestamp(&self) -> Timestamp {
        self.raw.receive_timestamp()
    }

    /// Optional center frequency in hertz.
    pub fn center_frequency_hz(&self) -> Option<f64> {
        self.raw.center_frequency_hz()
    }

    /// Optional channel bandwidth in hertz.
    pub fn bandwidth_hz(&self) -> Option<f64> {
        self.raw.bandwidth_hz()
    }

    /// Receive antenna count.
    pub fn receive_antennas(&self) -> u16 {
        self.raw.receive_antennas()
    }

    /// Transmit antenna count.
    pub fn transmit_antennas(&self) -> u16 {
        self.raw.transmit_antennas()
    }

    /// Ordered subcarrier indices (unchanged from the raw frame).
    pub fn subcarrier_indices(&self) -> &[i16] {
        self.raw.subcarrier_indices()
    }

    /// Calibrated complex samples in canonical `[rx][tx][subcarrier]` order.
    pub fn samples(&self) -> &[ComplexSample] {
        &self.samples
    }

    /// Frame origin classification from the raw frame.
    pub fn source(&self) -> CsiSourceKind {
        self.raw.source()
    }

    /// Optional radio metadata from the raw frame.
    pub fn radio(&self) -> &CsiRadioMetadata {
        self.raw.radio()
    }

    /// Calibration profile identity.
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Calibration profile version.
    pub fn profile_version(&self) -> u32 {
        self.profile_version
    }

    /// Pipeline execution timestamp.
    pub fn calibrated_at(&self) -> Timestamp {
        self.calibrated_at
    }

    /// Structured calibration report and provenance.
    pub fn report(&self) -> &CalibrationReport {
        &self.report
    }

    /// Number of subcarriers.
    pub fn subcarrier_count(&self) -> usize {
        self.raw.subcarrier_count()
    }
}
