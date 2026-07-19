//! Calibration pipeline: ordered stages → validated [`CalibratedCsiFrame`].

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use aeryon_csi::CsiFrame;
use aeryon_domain::Timestamp;

use crate::errors::CalibrationError;
use crate::frame::CalibratedCsiFrame;
use crate::profile::CalibrationProfile;
use crate::report::{CalibrationReport, CalibrationStatus, CalibrationWarning};
use crate::stage::CalibrationBuffer;
use crate::stages::phase_unwrap::duration_ns;

/// Deterministic CSI calibration pipeline for a single profile.
#[derive(Debug, Clone)]
pub struct CalibrationPipeline {
    profile: CalibrationProfile,
}

impl CalibrationPipeline {
    /// Creates a pipeline from a validated profile.
    pub fn try_new(profile: CalibrationProfile) -> Result<Self, CalibrationError> {
        profile.validate()?;
        Ok(Self { profile })
    }

    /// Active profile.
    pub fn profile(&self) -> &CalibrationProfile {
        &self.profile
    }

    /// Calibrates one raw frame.
    ///
    /// On failure, no partial [`CalibratedCsiFrame`] is returned. The input raw
    /// frame is never mutated.
    pub fn calibrate(&self, raw: Arc<CsiFrame>) -> Result<CalibratedCsiFrame, CalibrationError> {
        let started_at = now();
        let wall_start = std::time::Instant::now();
        let input_sample_count = raw.samples().len();
        let context_preview = (
            raw.frame_id(),
            raw.sensor_id(),
            raw.sequence(),
            raw.receive_antennas(),
            raw.transmit_antennas(),
            raw.subcarrier_indices().to_vec(),
        );

        let mut buffer = CalibrationBuffer::from_frame(raw.as_ref())?;
        let context = buffer.context();
        let stages = self.profile.build_stages()?;

        let mut stage_reports = Vec::with_capacity(stages.len());
        let mut warnings: Vec<CalibrationWarning> = Vec::new();

        // Record skipped (disabled) stages as warnings without executing them.
        for (order, config) in self.profile.stages.iter().enumerate() {
            if !config.enabled() {
                warnings.push(CalibrationWarning::StageSkipped {
                    stage: config.stage_id(),
                    order: order as u16,
                });
            }
        }

        for (execution_order, configured) in stages.iter().enumerate() {
            let stage = configured.as_stage();
            let mut report = stage.apply(&mut buffer, &context)?;
            report.order = execution_order as u16;
            warnings.extend(report.warnings.iter().cloned());
            stage_reports.push(report);
        }

        let samples = buffer.into_samples();
        validate_output(
            raw.as_ref(),
            &samples,
            &context_preview.5,
            &stage_reports,
            self.profile.stages.iter().filter(|s| s.enabled()).count(),
        )?;

        let completed_at = now();
        let duration_ns = duration_ns(wall_start);
        let output_sample_count = samples.len();

        let report = CalibrationReport {
            raw_frame_id: context_preview.0,
            sequence: context_preview.2,
            profile_id: self.profile.id.clone(),
            profile_version: self.profile.version,
            started_at,
            completed_at,
            duration_ns,
            stages: stage_reports,
            input_sample_count,
            output_sample_count,
            warnings,
            status: CalibrationStatus::Success,
        };

        Ok(CalibratedCsiFrame::new(
            raw,
            samples,
            self.profile.id.clone(),
            self.profile.version,
            completed_at,
            report,
        ))
    }
}

fn validate_output(
    raw: &CsiFrame,
    samples: &[aeryon_csi::ComplexSample],
    expected_indices: &[i16],
    stage_reports: &[crate::report::StageReport],
    expected_stage_count: usize,
) -> Result<(), CalibrationError> {
    if samples.len() != raw.samples().len() {
        return Err(CalibrationError::OutputValidation {
            frame_id: raw.frame_id(),
            sequence: raw.sequence(),
            message: format!(
                "sample count changed: raw={}, calibrated={}",
                raw.samples().len(),
                samples.len()
            ),
        });
    }

    if raw.subcarrier_indices() != expected_indices {
        return Err(CalibrationError::OutputValidation {
            frame_id: raw.frame_id(),
            sequence: raw.sequence(),
            message: "subcarrier ordering changed during calibration".to_owned(),
        });
    }

    let expected = usize::from(raw.receive_antennas())
        * usize::from(raw.transmit_antennas())
        * raw.subcarrier_count();
    if samples.len() != expected {
        return Err(CalibrationError::OutputValidation {
            frame_id: raw.frame_id(),
            sequence: raw.sequence(),
            message: "calibrated dimensions do not match raw frame".to_owned(),
        });
    }

    for (index, sample) in samples.iter().enumerate() {
        if !sample.re.is_finite() || !sample.im.is_finite() {
            return Err(CalibrationError::OutputValidation {
                frame_id: raw.frame_id(),
                sequence: raw.sequence(),
                message: format!("non-finite calibrated sample at index {index}"),
            });
        }
    }

    if stage_reports.len() != expected_stage_count {
        return Err(CalibrationError::OutputValidation {
            frame_id: raw.frame_id(),
            sequence: raw.sequence(),
            message: "stage report count does not match executed stages".to_owned(),
        });
    }

    for (idx, report) in stage_reports.iter().enumerate() {
        if report.order as usize != idx {
            return Err(CalibrationError::OutputValidation {
                frame_id: raw.frame_id(),
                sequence: raw.sequence(),
                message: "stage report order does not match execution order".to_owned(),
            });
        }
        if !report.success {
            return Err(CalibrationError::OutputValidation {
                frame_id: raw.frame_id(),
                sequence: raw.sequence(),
                message: format!("stage {} reported failure", report.stage_id),
            });
        }
    }

    Ok(())
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{StageConfig, baseline_csi_v1};
    use aeryon_csi::{ComplexSample, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId};
    use num_complex::Complex32;

    fn sample_frame() -> Arc<CsiFrame> {
        let indices = vec![-2, -1, 0, 1];
        let mut samples = Vec::new();
        for rx in 0..2_u16 {
            for sc in 0..4 {
                let phase = 0.3 * sc as f32 + 0.1 * f32::from(rx);
                samples.push(Complex32::from_polar(1.5 + f32::from(rx), phase));
            }
        }
        Arc::new(
            CsiFrame::try_new(
                FrameMetadata {
                    frame_id: FrameId::new(42),
                    sensor_id: SensorId::new(2),
                    timestamp: Timestamp::from_nanos(1000),
                    sequence: 5,
                    mission_id: None,
                    metadata: Metadata::new(),
                },
                Timestamp::from_nanos(2000),
                Some(5_180_000_000.0),
                Some(20_000_000.0),
                2,
                1,
                indices,
                samples,
                CsiSourceKind::Replay,
                CsiRadioMetadata::default(),
            )
            .expect("frame"),
        )
    }

    #[test]
    fn stages_execute_in_configured_order() {
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        let calibrated = pipeline.calibrate(sample_frame()).expect("calibrate");
        let names: Vec<_> = calibrated
            .report()
            .stages
            .iter()
            .map(|s| s.stage_id.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "phase_unwrap",
                "linear_phase_detrend",
                "rms_amplitude_normalize"
            ]
        );
        for (idx, stage) in calibrated.report().stages.iter().enumerate() {
            assert_eq!(stage.order as usize, idx);
        }
    }

    #[test]
    fn raw_frame_remains_unchanged() {
        let raw = sample_frame();
        let before: Vec<ComplexSample> = raw.samples().to_vec();
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        let calibrated = pipeline.calibrate(Arc::clone(&raw)).expect("calibrate");
        assert_eq!(raw.samples(), before.as_slice());
        assert_eq!(calibrated.raw().samples(), before.as_slice());
        assert_ne!(calibrated.samples(), before.as_slice());
    }

    #[test]
    fn metadata_and_dimensions_preserved() {
        let raw = sample_frame();
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        let calibrated = pipeline.calibrate(Arc::clone(&raw)).expect("calibrate");
        assert_eq!(calibrated.raw_frame_id(), raw.frame_id());
        assert_eq!(calibrated.sensor_id(), raw.sensor_id());
        assert_eq!(calibrated.sequence(), raw.sequence());
        assert_eq!(calibrated.capture_timestamp(), raw.capture_timestamp());
        assert_eq!(calibrated.receive_timestamp(), raw.receive_timestamp());
        assert_eq!(calibrated.receive_antennas(), raw.receive_antennas());
        assert_eq!(calibrated.transmit_antennas(), raw.transmit_antennas());
        assert_eq!(calibrated.subcarrier_indices(), raw.subcarrier_indices());
        assert_eq!(calibrated.samples().len(), raw.samples().len());
        assert_eq!(calibrated.profile_id(), "baseline-csi-v1");
        assert_eq!(calibrated.profile_version(), 1);
        assert!(
            calibrated
                .samples()
                .iter()
                .all(|s| s.re.is_finite() && s.im.is_finite())
        );
    }

    #[test]
    fn failure_prevents_partial_output() {
        let mut profile = baseline_csi_v1();
        // Force RMS with large epsilon so typical magnitudes fail.
        if let StageConfig::RmsAmplitudeNormalize { epsilon, .. } = &mut profile.stages[2] {
            *epsilon = 100.0;
        }
        let pipeline = CalibrationPipeline::try_new(profile).expect("pipeline");
        let error = pipeline.calibrate(sample_frame()).expect_err("should fail");
        assert!(matches!(error, CalibrationError::ZeroEnergyLink { .. }));
    }

    #[test]
    fn deterministic_for_same_frame_and_profile() {
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        let raw = sample_frame();
        let a = pipeline.calibrate(Arc::clone(&raw)).expect("a");
        let b = pipeline.calibrate(Arc::clone(&raw)).expect("b");
        assert_eq!(a.samples(), b.samples());
        assert_eq!(a.profile_id(), b.profile_id());
    }
}
