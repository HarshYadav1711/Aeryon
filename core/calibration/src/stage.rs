//! Calibration stage identities and the private working buffer.

use aeryon_csi::{ComplexSample, CsiFrame};
use aeryon_domain::{FrameId, SensorId};

use crate::errors::{AntennaLink, CalibrationError};
use crate::report::StageReport;

/// Strongly typed calibration stage identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalibrationStageId {
    /// Spatial phase unwrapping across ordered subcarriers per antenna link.
    PhaseUnwrap,
    /// Per-link affine phase sanitization (baseline; not full hardware calibration).
    LinearPhaseDetrend,
    /// Per-link RMS amplitude normalization.
    RmsAmplitudeNormalize,
}

impl CalibrationStageId {
    /// Stable configuration / wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PhaseUnwrap => "phase_unwrap",
            Self::LinearPhaseDetrend => "linear_phase_detrend",
            Self::RmsAmplitudeNormalize => "rms_amplitude_normalize",
        }
    }

    /// Human-readable stage name.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::PhaseUnwrap => "Phase Unwrap",
            Self::LinearPhaseDetrend => "Linear Phase Detrend",
            Self::RmsAmplitudeNormalize => "RMS Amplitude Normalize",
        }
    }
}

impl core::fmt::Display for CalibrationStageId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-frame context shared with every stage invocation.
#[derive(Debug, Clone, Copy)]
pub struct CalibrationContext {
    /// Raw frame identifier.
    pub frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Monotonic sequence number.
    pub sequence: u64,
}

/// Private mutable working representation used while calibration runs.
///
/// Not part of the public calibrated output. Uses the canonical
/// `[rx][tx][subcarrier]` sample layout from [`CsiFrame`].
///
/// Continuous phase is tracked separately from complex samples because
/// `arg()` only returns principal values in (−π, π]. Spatial unwrapping must
/// survive into later stages.
#[derive(Debug)]
pub(crate) struct CalibrationBuffer {
    receive_antennas: u16,
    transmit_antennas: u16,
    subcarrier_indices: Vec<i16>,
    samples: Vec<ComplexSample>,
    /// Continuous phase (radians) for each sample, same layout as `samples`.
    phases: Vec<f32>,
    frame_id: FrameId,
    sensor_id: SensorId,
    sequence: u64,
}

impl CalibrationBuffer {
    /// Constructs a working buffer by copying samples from a validated raw frame.
    pub(crate) fn from_frame(frame: &CsiFrame) -> Result<Self, CalibrationError> {
        let expected = usize::from(frame.receive_antennas())
            .checked_mul(usize::from(frame.transmit_antennas()))
            .and_then(|links| links.checked_mul(frame.subcarrier_count()))
            .ok_or_else(|| CalibrationError::MalformedFrame {
                frame_id: Some(frame.frame_id()),
                sequence: Some(frame.sequence()),
                message: "sample dimension overflow".to_owned(),
            })?;

        if frame.samples().len() != expected {
            return Err(CalibrationError::MalformedFrame {
                frame_id: Some(frame.frame_id()),
                sequence: Some(frame.sequence()),
                message: format!(
                    "sample count mismatch: expected {expected}, got {}",
                    frame.samples().len()
                ),
            });
        }

        let mut phases = Vec::with_capacity(frame.samples().len());
        for (index, sample) in frame.samples().iter().enumerate() {
            if !sample.re.is_finite() || !sample.im.is_finite() {
                return Err(CalibrationError::NonFiniteSample {
                    frame_id: frame.frame_id(),
                    sequence: frame.sequence(),
                    stage: None,
                    sample_index: Some(index),
                    link: None,
                });
            }
            let phase = sample.arg();
            if !phase.is_finite() {
                return Err(CalibrationError::NonFiniteSample {
                    frame_id: frame.frame_id(),
                    sequence: frame.sequence(),
                    stage: None,
                    sample_index: Some(index),
                    link: None,
                });
            }
            phases.push(phase);
        }

        Ok(Self {
            receive_antennas: frame.receive_antennas(),
            transmit_antennas: frame.transmit_antennas(),
            subcarrier_indices: frame.subcarrier_indices().to_vec(),
            samples: frame.samples().to_vec(),
            phases,
            frame_id: frame.frame_id(),
            sensor_id: frame.sensor_id(),
            sequence: frame.sequence(),
        })
    }

    pub(crate) fn context(&self) -> CalibrationContext {
        CalibrationContext {
            frame_id: self.frame_id,
            sensor_id: self.sensor_id,
            sequence: self.sequence,
        }
    }

    pub(crate) fn subcarrier_indices(&self) -> &[i16] {
        &self.subcarrier_indices
    }

    pub(crate) fn subcarrier_count(&self) -> usize {
        self.subcarrier_indices.len()
    }

    #[cfg(test)]
    pub(crate) fn samples(&self) -> &[ComplexSample] {
        &self.samples
    }

    #[cfg(test)]
    pub(crate) fn phases(&self) -> &[f32] {
        &self.phases
    }

    pub(crate) fn into_samples(self) -> Vec<ComplexSample> {
        self.samples
    }

    /// Mutable owned link views processed via copy-back (keeps the buffer private
    /// and avoids `unsafe`).
    pub(crate) fn for_each_link_mut<F>(&mut self, mut f: F) -> Result<(), CalibrationError>
    where
        F: FnMut(AntennaLink, &mut [ComplexSample], &mut [f32]) -> Result<(), CalibrationError>,
    {
        let n_sc = self.subcarrier_count();
        let n_tx = usize::from(self.transmit_antennas);
        let link_count = usize::from(self.receive_antennas) * n_tx;

        for link_idx in 0..link_count {
            let start = link_idx * n_sc;
            let end = start + n_sc;
            let rx = (link_idx / n_tx) as u16;
            let tx = (link_idx % n_tx) as u16;
            let mut samples = self.samples[start..end].to_vec();
            let mut phases = self.phases[start..end].to_vec();
            f(AntennaLink::new(rx, tx), &mut samples, &mut phases)?;
            self.samples[start..end].copy_from_slice(&samples);
            self.phases[start..end].copy_from_slice(&phases);
        }
        Ok(())
    }

    /// Test helper: overwrite one sample without validating.
    #[cfg(test)]
    pub(crate) fn set_sample_for_test(&mut self, index: usize, sample: ComplexSample) {
        self.samples[index] = sample;
    }
}

/// Stage contract applied by a calibrated profile.
pub(crate) trait CalibrationStage: Send + Sync {
    /// Strongly typed stage identity.
    fn id(&self) -> CalibrationStageId;

    /// Human-readable stage name.
    fn name(&self) -> &'static str {
        self.id().display_name()
    }

    /// Applies the stage to the working buffer.
    ///
    /// Stages are CPU-only and synchronous. Execution order is controlled by the
    /// active profile; stages must not silently reorder themselves.
    fn apply(
        &self,
        buffer: &mut CalibrationBuffer,
        context: &CalibrationContext,
    ) -> Result<StageReport, CalibrationError>;
}
