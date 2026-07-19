//! Canonical validated [`CsiFrame`] representation.

use aeryon_domain::{Frame, FrameId, FrameMetadata, SensorId, Timestamp};
use num_complex::Complex32;

use crate::error::CsiFrameError;

/// Minimum accepted center frequency in hertz when the field is present.
pub const CENTER_FREQUENCY_HZ_MIN: f64 = 1.0;
/// Minimum accepted channel bandwidth in hertz when the field is present.
pub const CHANNEL_BANDWIDTH_HZ_MIN: f64 = 1.0;

/// Canonical complex sample type for CSI matrices (`f32` real / imaginary).
pub type ComplexSample = Complex32;

/// Origin of a CSI frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsiSourceKind {
    /// Deterministic development fixture replay (not live RF).
    Replay,
    /// Live hardware capture (not produced by this milestone).
    Live,
}

impl CsiSourceKind {
    /// Stable wire / logging label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replay => "csi_replay",
            Self::Live => "csi_live",
        }
    }
}

/// Optional radio metadata fields common across CSI backends.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CsiRadioMetadata {
    /// Received signal strength indicator in dBm, when available.
    pub rssi_dbm: Option<f32>,
    /// Noise floor estimate in dBm, when available.
    pub noise_floor_dbm: Option<f32>,
    /// Automatic gain control value, when available.
    pub agc: Option<f32>,
    /// Packet/frame flags bitfield, when available.
    pub flags: Option<u32>,
}

/// Canonical WiFi CSI frame.
///
/// Samples are stored contiguously in `[rx][tx][subcarrier]` order. Amplitude and
/// phase are derived on demand and are never duplicated in storage.
#[derive(Debug, Clone, PartialEq)]
pub struct CsiFrame {
    metadata: FrameMetadata,
    receive_timestamp: Timestamp,
    center_frequency_hz: Option<f64>,
    bandwidth_hz: Option<f64>,
    receive_antennas: u16,
    transmit_antennas: u16,
    subcarrier_indices: Vec<i16>,
    samples: Vec<ComplexSample>,
    source: CsiSourceKind,
    radio: CsiRadioMetadata,
}

impl CsiFrame {
    /// Validates and constructs a CSI frame.
    ///
    /// Subcarrier indices must already be unique and strictly ascending. Sample
    /// count must equal `receive_antennas × transmit_antennas × subcarrier_count`.
    /// Malformed input is rejected; it is never sorted, padded, or repaired.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        metadata: FrameMetadata,
        receive_timestamp: Timestamp,
        center_frequency_hz: Option<f64>,
        bandwidth_hz: Option<f64>,
        receive_antennas: u16,
        transmit_antennas: u16,
        subcarrier_indices: Vec<i16>,
        samples: Vec<ComplexSample>,
        source: CsiSourceKind,
        radio: CsiRadioMetadata,
    ) -> Result<Self, CsiFrameError> {
        if receive_antennas == 0 {
            return Err(CsiFrameError::ZeroReceiveAntennas);
        }
        if transmit_antennas == 0 {
            return Err(CsiFrameError::ZeroTransmitAntennas);
        }
        if subcarrier_indices.is_empty() {
            return Err(CsiFrameError::EmptySubcarriers);
        }

        for window in subcarrier_indices.windows(2) {
            let previous = window[0];
            let current = window[1];
            if current == previous {
                return Err(CsiFrameError::DuplicateSubcarrierIndex { index: current });
            }
            if current < previous {
                let position = subcarrier_indices
                    .iter()
                    .position(|value| *value == current)
                    .unwrap_or(1);
                return Err(CsiFrameError::SubcarriersNotAscending { position });
            }
        }

        let expected = usize::from(receive_antennas)
            .checked_mul(usize::from(transmit_antennas))
            .and_then(|links| links.checked_mul(subcarrier_indices.len()))
            .ok_or(CsiFrameError::SampleCountMismatch {
                expected: 0,
                actual: samples.len(),
            })?;

        if samples.len() != expected {
            return Err(CsiFrameError::SampleCountMismatch {
                expected,
                actual: samples.len(),
            });
        }

        for (index, sample) in samples.iter().enumerate() {
            if !sample.re.is_finite() || !sample.im.is_finite() {
                return Err(CsiFrameError::NonFiniteSample { index });
            }
        }

        if let Some(frequency) = center_frequency_hz {
            if !frequency.is_finite() || frequency < CENTER_FREQUENCY_HZ_MIN {
                return Err(CsiFrameError::InvalidCenterFrequency);
            }
        }

        if let Some(bandwidth) = bandwidth_hz {
            if !bandwidth.is_finite() || bandwidth < CHANNEL_BANDWIDTH_HZ_MIN {
                return Err(CsiFrameError::InvalidBandwidth);
            }
        }

        if radio.rssi_dbm.is_some_and(|value| !value.is_finite()) {
            return Err(CsiFrameError::InvalidRssi);
        }
        if radio
            .noise_floor_dbm
            .is_some_and(|value| !value.is_finite())
        {
            return Err(CsiFrameError::InvalidNoiseFloor);
        }
        if radio.agc.is_some_and(|value| !value.is_finite()) {
            return Err(CsiFrameError::InvalidAgc);
        }

        Ok(Self {
            metadata,
            receive_timestamp,
            center_frequency_hz,
            bandwidth_hz,
            receive_antennas,
            transmit_antennas,
            subcarrier_indices,
            samples,
            source,
            radio,
        })
    }

    /// Frame identifier.
    pub fn frame_id(&self) -> FrameId {
        self.metadata.frame_id
    }

    /// Sensor identifier.
    pub fn sensor_id(&self) -> SensorId {
        self.metadata.sensor_id
    }

    /// Monotonic sequence number from frame metadata.
    pub fn sequence(&self) -> u64 {
        self.metadata.sequence
    }

    /// Capture / acquisition timestamp from frame metadata.
    pub fn capture_timestamp(&self) -> Timestamp {
        self.metadata.timestamp
    }

    /// Receive or replay timestamp.
    pub fn receive_timestamp(&self) -> Timestamp {
        self.receive_timestamp
    }

    /// Optional center frequency in hertz.
    pub fn center_frequency_hz(&self) -> Option<f64> {
        self.center_frequency_hz
    }

    /// Optional channel bandwidth in hertz.
    pub fn bandwidth_hz(&self) -> Option<f64> {
        self.bandwidth_hz
    }

    /// Receive antenna count.
    pub fn receive_antennas(&self) -> u16 {
        self.receive_antennas
    }

    /// Transmit antenna count.
    pub fn transmit_antennas(&self) -> u16 {
        self.transmit_antennas
    }

    /// Canonical subcarrier indices (strictly ascending).
    pub fn subcarrier_indices(&self) -> &[i16] {
        &self.subcarrier_indices
    }

    /// Contiguous complex samples in `[rx][tx][subcarrier]` order.
    pub fn samples(&self) -> &[ComplexSample] {
        &self.samples
    }

    /// Frame origin (replay vs live).
    pub fn source(&self) -> CsiSourceKind {
        self.source
    }

    /// Optional common radio metadata.
    pub fn radio(&self) -> &CsiRadioMetadata {
        &self.radio
    }

    /// Number of subcarriers.
    pub fn subcarrier_count(&self) -> usize {
        self.subcarrier_indices.len()
    }

    /// Number of RX×TX antenna links.
    pub fn link_count(&self) -> usize {
        usize::from(self.receive_antennas) * usize::from(self.transmit_antennas)
    }

    /// Returns a sample for `(rx, tx, subcarrier_position)`, or `None` if out of bounds.
    pub fn sample(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<ComplexSample> {
        let index = self.sample_index(rx, tx, subcarrier_position)?;
        self.samples.get(index).copied()
    }

    /// Returns the contiguous sample slice for one RX×TX link, or `None` if out of bounds.
    pub fn link(&self, rx: u16, tx: u16) -> Option<&[ComplexSample]> {
        if rx >= self.receive_antennas || tx >= self.transmit_antennas {
            return None;
        }
        let start = self.sample_index(rx, tx, 0)?;
        let end = start + self.subcarrier_count();
        self.samples.get(start..end)
    }

    /// Amplitude (magnitude) of a selected sample, if indices are in bounds.
    pub fn amplitude(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<f32> {
        self.sample(rx, tx, subcarrier_position)
            .map(|sample| sample.norm())
    }

    /// Phase (radians) of a selected sample, if indices are in bounds.
    pub fn phase(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<f32> {
        self.sample(rx, tx, subcarrier_position)
            .map(|sample| sample.arg())
    }

    /// Iterator over amplitudes for one antenna link.
    pub fn amplitude_iter(&self, rx: u16, tx: u16) -> Option<impl Iterator<Item = f32> + '_> {
        self.link(rx, tx)
            .map(|samples| samples.iter().map(|sample| sample.norm()))
    }

    /// Iterator over phases (radians) for one antenna link.
    pub fn phase_iter(&self, rx: u16, tx: u16) -> Option<impl Iterator<Item = f32> + '_> {
        self.link(rx, tx)
            .map(|samples| samples.iter().map(|sample| sample.arg()))
    }

    /// Typed bounds-checked sample lookup.
    pub fn sample_checked(
        &self,
        rx: u16,
        tx: u16,
        subcarrier_position: usize,
    ) -> Result<ComplexSample, CsiFrameError> {
        if rx >= self.receive_antennas {
            return Err(CsiFrameError::OutOfBounds {
                component: "receive_antenna",
                requested: usize::from(rx),
                bound: usize::from(self.receive_antennas),
            });
        }
        if tx >= self.transmit_antennas {
            return Err(CsiFrameError::OutOfBounds {
                component: "transmit_antenna",
                requested: usize::from(tx),
                bound: usize::from(self.transmit_antennas),
            });
        }
        if subcarrier_position >= self.subcarrier_count() {
            return Err(CsiFrameError::OutOfBounds {
                component: "subcarrier_position",
                requested: subcarrier_position,
                bound: self.subcarrier_count(),
            });
        }
        Ok(self
            .sample(rx, tx, subcarrier_position)
            .expect("bounds already checked"))
    }

    fn sample_index(&self, rx: u16, tx: u16, subcarrier_position: usize) -> Option<usize> {
        if rx >= self.receive_antennas
            || tx >= self.transmit_antennas
            || subcarrier_position >= self.subcarrier_count()
        {
            return None;
        }
        let links_before = usize::from(rx) * usize::from(self.transmit_antennas) + usize::from(tx);
        Some(links_before * self.subcarrier_count() + subcarrier_position)
    }
}

impl Frame for CsiFrame {
    type Payload = Vec<ComplexSample>;

    fn metadata(&self) -> &FrameMetadata {
        &self.metadata
    }

    fn payload(&self) -> &Self::Payload {
        &self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_domain::Metadata;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn metadata(sequence: u64) -> FrameMetadata {
        FrameMetadata {
            frame_id: FrameId::new(sequence + 1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(1_000 + sequence),
            sequence,
            mission_id: None,
            metadata: Metadata::new(),
        }
    }

    fn valid_frame() -> CsiFrame {
        CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(2_000),
            Some(5_180_000_000.0),
            Some(20_000_000.0),
            2,
            1,
            vec![-8, -4, 0, 4],
            vec![
                Complex32::new(1.0, 0.0),
                Complex32::new(0.0, 1.0),
                Complex32::new(-1.0, 0.0),
                Complex32::new(0.0, -1.0),
                Complex32::new(2.0, 0.0),
                Complex32::new(0.0, 2.0),
                Complex32::new(-2.0, 0.0),
                Complex32::new(0.0, -2.0),
            ],
            CsiSourceKind::Replay,
            CsiRadioMetadata {
                rssi_dbm: Some(-40.0),
                noise_floor_dbm: Some(-90.0),
                agc: Some(12.0),
                flags: Some(0x1),
            },
        )
        .expect("valid frame")
    }

    #[test]
    fn valid_frame_construction() {
        let frame = valid_frame();
        assert_eq!(frame.receive_antennas(), 2);
        assert_eq!(frame.transmit_antennas(), 1);
        assert_eq!(frame.subcarrier_count(), 4);
        assert_eq!(frame.link_count(), 2);
        assert_eq!(frame.source(), CsiSourceKind::Replay);
        assert_eq!(frame.samples().len(), 8);
    }

    #[test]
    fn zero_receive_antennas_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            0,
            1,
            vec![0],
            vec![Complex32::new(1.0, 0.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("zero rx");
        assert_eq!(error, CsiFrameError::ZeroReceiveAntennas);
    }

    #[test]
    fn zero_transmit_antennas_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            1,
            0,
            vec![0],
            vec![Complex32::new(1.0, 0.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("zero tx");
        assert_eq!(error, CsiFrameError::ZeroTransmitAntennas);
    }

    #[test]
    fn empty_subcarriers_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            1,
            1,
            vec![],
            vec![],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("empty sc");
        assert_eq!(error, CsiFrameError::EmptySubcarriers);
    }

    #[test]
    fn duplicate_subcarrier_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            1,
            1,
            vec![0, 0],
            vec![Complex32::new(1.0, 0.0), Complex32::new(0.0, 1.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("duplicate");
        assert_eq!(error, CsiFrameError::DuplicateSubcarrierIndex { index: 0 });
    }

    #[test]
    fn sample_count_mismatch_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            2,
            1,
            vec![0, 1],
            vec![Complex32::new(1.0, 0.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("mismatch");
        assert_eq!(
            error,
            CsiFrameError::SampleCountMismatch {
                expected: 4,
                actual: 1
            }
        );
    }

    #[test]
    fn non_finite_sample_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            1,
            1,
            vec![0],
            vec![Complex32::new(f32::NAN, 0.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("nan");
        assert_eq!(error, CsiFrameError::NonFiniteSample { index: 0 });
    }

    #[test]
    fn safe_indexing_returns_none_out_of_bounds() {
        let frame = valid_frame();
        assert!(frame.sample(9, 0, 0).is_none());
        assert!(frame.link(0, 9).is_none());
        assert!(matches!(
            frame.sample_checked(0, 0, 99),
            Err(CsiFrameError::OutOfBounds { .. })
        ));
    }

    #[test]
    fn canonical_layout_indexes_rx_tx_subcarrier() {
        let frame = valid_frame();
        assert_eq!(frame.sample(0, 0, 0), Some(Complex32::new(1.0, 0.0)));
        assert_eq!(frame.sample(0, 0, 1), Some(Complex32::new(0.0, 1.0)));
        assert_eq!(frame.sample(1, 0, 0), Some(Complex32::new(2.0, 0.0)));
        assert_eq!(frame.link(1, 0).map(|link| link.len()), Some(4));
    }

    #[test]
    fn amplitude_and_phase_use_known_values() {
        let frame = valid_frame();
        let amplitude = frame.amplitude(0, 0, 0).expect("amp");
        let phase = frame.phase(0, 0, 1).expect("phase");
        assert!((amplitude - 1.0).abs() < 1e-6);
        assert!((phase - FRAC_PI_2).abs() < 1e-5);
        assert!((frame.phase(0, 0, 2).expect("pi") - PI).abs() < 1e-5);
    }

    #[test]
    fn link_iteration_produces_expected_amplitudes() {
        let frame = valid_frame();
        let amplitudes: Vec<_> = frame.amplitude_iter(0, 0).expect("iter").collect();
        assert_eq!(amplitudes.len(), 4);
        assert!((amplitudes[0] - 1.0).abs() < 1e-6);
        assert!((amplitudes[1] - 1.0).abs() < 1e-6);
        let phases: Vec<_> = frame.phase_iter(1, 0).expect("phases").collect();
        assert_eq!(phases.len(), 4);
    }

    #[test]
    fn non_ascending_subcarriers_rejected() {
        let error = CsiFrame::try_new(
            metadata(0),
            Timestamp::from_nanos(1),
            None,
            None,
            1,
            1,
            vec![1, -1],
            vec![Complex32::new(1.0, 0.0), Complex32::new(0.0, 1.0)],
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect_err("descending");
        assert!(matches!(
            error,
            CsiFrameError::SubcarriersNotAscending { .. }
        ));
    }
}
