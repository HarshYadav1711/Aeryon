//! Per-link RMS amplitude normalization.

use std::time::Instant;

use aeryon_csi::ComplexSample;

use crate::errors::{AntennaLink, CalibrationError};
use crate::report::{StageDiagnostics, StageReport};
use crate::stage::{CalibrationBuffer, CalibrationContext, CalibrationStage, CalibrationStageId};
use crate::stages::phase_unwrap::duration_ns;

/// Default positive epsilon used when rejecting zero-energy links.
pub const DEFAULT_RMS_EPSILON: f32 = 1.0e-8;

/// Per-link RMS magnitude normalization.
///
/// For each RX×TX link:
/// `rms = sqrt(sum(magnitude²) / sample_count)`, then every complex sample on
/// that link is divided by `rms`. Phase is preserved. Links with
/// `rms <= epsilon` fail calibration (zero-energy policy).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmsAmplitudeNormalizeStage {
    /// Positive finite threshold; RMS at or below this value is rejected.
    pub epsilon: f32,
}

impl Default for RmsAmplitudeNormalizeStage {
    fn default() -> Self {
        Self {
            epsilon: DEFAULT_RMS_EPSILON,
        }
    }
}

impl RmsAmplitudeNormalizeStage {
    /// Creates a stage with a validated epsilon.
    pub fn try_new(epsilon: f32) -> Result<Self, CalibrationError> {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(CalibrationError::InvalidProfile {
                message: format!(
                    "rms_amplitude_normalize.epsilon must be positive and finite (got {epsilon})"
                ),
            });
        }
        Ok(Self { epsilon })
    }
}

impl CalibrationStage for RmsAmplitudeNormalizeStage {
    fn id(&self) -> CalibrationStageId {
        CalibrationStageId::RmsAmplitudeNormalize
    }

    fn apply(
        &self,
        buffer: &mut CalibrationBuffer,
        context: &CalibrationContext,
    ) -> Result<StageReport, CalibrationError> {
        let started = Instant::now();
        let mut original_rms = Vec::new();
        let epsilon = self.epsilon;

        buffer.for_each_link_mut(|link, samples, phases| {
            let rms = normalize_link(samples, phases, epsilon, context, link)?;
            original_rms.push(rms);
            Ok(())
        })?;

        let links_processed = original_rms.len() as u32;
        Ok(StageReport {
            stage_id: self.id(),
            stage_name: self.name().to_owned(),
            order: 0,
            duration_ns: duration_ns(started),
            success: true,
            warning_count: 0,
            warnings: Vec::new(),
            diagnostics: StageDiagnostics::RmsAmplitudeNormalize {
                links_processed,
                original_rms,
            },
        })
    }
}

fn normalize_link(
    samples: &mut [ComplexSample],
    phases: &mut [f32],
    epsilon: f32,
    context: &CalibrationContext,
    link: AntennaLink,
) -> Result<f32, CalibrationError> {
    if samples.is_empty() {
        return Err(CalibrationError::InsufficientSubcarriers {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: CalibrationStageId::RmsAmplitudeNormalize,
            link: Some(link),
            count: 0,
        });
    }

    let mut sum_sq = 0.0_f64;
    for (offset, sample) in samples.iter().enumerate() {
        if !sample.re.is_finite() || !sample.im.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::RmsAmplitudeNormalize),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        let re = f64::from(sample.re);
        let im = f64::from(sample.im);
        sum_sq += re * re + im * im;
    }

    let mean_sq = sum_sq / samples.len() as f64;
    if !mean_sq.is_finite() || mean_sq < 0.0 {
        return Err(CalibrationError::StageFailure {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: CalibrationStageId::RmsAmplitudeNormalize,
            message: "non-finite mean-square magnitude".to_owned(),
        });
    }

    let rms = mean_sq.sqrt() as f32;
    if !rms.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::RmsAmplitudeNormalize),
            sample_index: None,
            link: Some(link),
        });
    }

    if rms <= epsilon {
        return Err(CalibrationError::ZeroEnergyLink {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: CalibrationStageId::RmsAmplitudeNormalize,
            link,
            rms,
            epsilon,
        });
    }

    for (offset, sample) in samples.iter_mut().enumerate() {
        sample.re /= rms;
        sample.im /= rms;
        if !sample.re.is_finite() || !sample.im.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::RmsAmplitudeNormalize),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        // Phase is preserved by complex scaling with a positive real; keep plane synced.
        phases[offset] = sample.arg();
    }

    Ok(rms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::CalibrationBuffer;
    use aeryon_csi::{CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};
    use num_complex::Complex32;
    use std::f32::consts::FRAC_PI_2;

    fn frame(samples: Vec<Complex32>, indices: Vec<i16>, rx: u16) -> CsiFrame {
        CsiFrame::try_new(
            FrameMetadata {
                frame_id: FrameId::new(1),
                sensor_id: SensorId::new(2),
                timestamp: Timestamp::from_nanos(1),
                sequence: 0,
                mission_id: None,
                metadata: Metadata::new(),
            },
            Timestamp::from_nanos(2),
            None,
            None,
            rx,
            1,
            indices,
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("frame")
    }

    fn run(frame: &CsiFrame, epsilon: f32) -> Result<CalibrationBuffer, CalibrationError> {
        let mut buffer = CalibrationBuffer::from_frame(frame).expect("buffer");
        let context = buffer.context();
        let stage = RmsAmplitudeNormalizeStage::try_new(epsilon)?;
        stage.apply(&mut buffer, &context)?;
        Ok(buffer)
    }

    fn link_rms(samples: &[Complex32]) -> f32 {
        let sum_sq: f64 = samples
            .iter()
            .map(|s| f64::from(s.re) * f64::from(s.re) + f64::from(s.im) * f64::from(s.im))
            .sum();
        (sum_sq / samples.len() as f64).sqrt() as f32
    }

    #[test]
    fn normalized_rms_is_approximately_one() {
        let samples = vec![
            Complex32::new(2.0, 0.0),
            Complex32::new(0.0, 2.0),
            Complex32::new(-2.0, 0.0),
            Complex32::new(0.0, -2.0),
        ];
        let buffer = run(&frame(samples, vec![0, 1, 2, 3], 1), 1e-8).expect("ok");
        let rms = link_rms(buffer.samples());
        assert!((rms - 1.0).abs() < 1e-5, "rms={rms}");
    }

    #[test]
    fn phase_preserved() {
        let samples = vec![
            Complex32::from_polar(3.0, 0.4),
            Complex32::from_polar(3.0, FRAC_PI_2),
            Complex32::from_polar(3.0, -0.7),
        ];
        let expected: Vec<_> = samples.iter().map(|s| s.arg()).collect();
        let buffer = run(&frame(samples, vec![0, 1, 2], 1), 1e-8).expect("ok");
        for (sample, phase) in buffer.samples().iter().zip(expected) {
            assert!((sample.arg() - phase).abs() < 1e-5);
        }
    }

    #[test]
    fn links_normalize_independently() {
        let mut samples = vec![
            Complex32::new(4.0, 0.0),
            Complex32::new(0.0, 4.0),
            Complex32::new(1.0, 0.0),
            Complex32::new(0.0, 1.0),
        ];
        // pad to 2x1x2
        let _ = &mut samples;
        let buffer = run(&frame(samples, vec![0, 1], 2), 1e-8).expect("ok");
        let link0_rms = link_rms(&buffer.samples()[0..2]);
        let link1_rms = link_rms(&buffer.samples()[2..4]);
        assert!((link0_rms - 1.0).abs() < 1e-5);
        assert!((link1_rms - 1.0).abs() < 1e-5);
    }

    #[test]
    fn zero_energy_link_rejected() {
        let samples = vec![Complex32::new(0.0, 0.0), Complex32::new(0.0, 0.0)];
        let error = run(&frame(samples, vec![0, 1], 1), 1e-8).expect_err("zero");
        assert!(matches!(error, CalibrationError::ZeroEnergyLink { .. }));
    }

    #[test]
    fn near_zero_energy_honors_epsilon() {
        let samples = vec![Complex32::new(1e-10, 0.0), Complex32::new(0.0, 1e-10)];
        let error = run(&frame(samples, vec![0, 1], 1), 1e-8).expect_err("near zero");
        assert!(matches!(error, CalibrationError::ZeroEnergyLink { .. }));
    }

    #[test]
    fn non_finite_rejected() {
        let frame = frame(
            vec![Complex32::new(1.0, 0.0), Complex32::new(0.0, 1.0)],
            vec![0, 1],
            1,
        );
        let mut buffer = CalibrationBuffer::from_frame(&frame).expect("buffer");
        buffer.set_sample_for_test(0, Complex32::new(f32::INFINITY, 0.0));
        let context = buffer.context();
        let error = RmsAmplitudeNormalizeStage::default()
            .apply(&mut buffer, &context)
            .expect_err("inf");
        assert!(matches!(error, CalibrationError::NonFiniteSample { .. }));
    }

    #[test]
    fn deterministic_output() {
        let samples = vec![
            Complex32::new(1.0, 1.0),
            Complex32::new(2.0, -1.0),
            Complex32::new(-1.5, 0.5),
        ];
        let frame = frame(samples, vec![0, 1, 2], 1);
        let a = run(&frame, 1e-8).expect("a");
        let b = run(&frame, 1e-8).expect("b");
        assert_eq!(a.samples(), b.samples());
    }

    #[test]
    fn invalid_epsilon_rejected() {
        let error = RmsAmplitudeNormalizeStage::try_new(0.0).expect_err("zero eps");
        assert!(matches!(error, CalibrationError::InvalidProfile { .. }));
        let error = RmsAmplitudeNormalizeStage::try_new(-1.0).expect_err("neg");
        assert!(matches!(error, CalibrationError::InvalidProfile { .. }));
        let error = RmsAmplitudeNormalizeStage::try_new(f32::NAN).expect_err("nan");
        assert!(matches!(error, CalibrationError::InvalidProfile { .. }));
    }
}
