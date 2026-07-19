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

    /// Number of RX–TX antenna links.
    pub fn link_count(&self) -> usize {
        self.raw.link_count()
    }

    /// Returns a calibrated sample for `(rx, tx, subcarrier_position)`, if in bounds.
    pub fn sample(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<ComplexSample> {
        let index = self.sample_index(rx, tx, subcarrier_position)?;
        self.samples.get(index).copied()
    }

    /// Contiguous calibrated samples for one RX–TX link, if in bounds.
    pub fn link(&self, rx: u16, tx: u16) -> Option<&[ComplexSample]> {
        if rx >= self.receive_antennas() || tx >= self.transmit_antennas() {
            return None;
        }
        let start = self.sample_index(rx, tx, 0)?;
        let end = start + self.subcarrier_count();
        self.samples.get(start..end)
    }

    /// Calibrated amplitude (magnitude) for a selected sample.
    pub fn amplitude(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<f32> {
        self.sample(rx, tx, subcarrier_position)
            .map(|sample| sample.norm())
    }

    /// Calibrated phase (radians) for a selected sample.
    pub fn phase(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<f32> {
        self.sample(rx, tx, subcarrier_position)
            .map(|sample| sample.arg())
    }

    fn sample_index(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<usize> {
        if rx >= self.receive_antennas()
            || tx >= self.transmit_antennas()
            || subcarrier_position >= self.subcarrier_count()
        {
            return None;
        }
        let links_before =
            usize::from(rx) * usize::from(self.transmit_antennas()) + usize::from(tx);
        Some(links_before * self.subcarrier_count() + subcarrier_position)
    }
}
