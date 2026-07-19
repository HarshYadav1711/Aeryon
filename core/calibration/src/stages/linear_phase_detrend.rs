//! Per-link linear phase detrending (baseline affine sanitization).

use std::time::Instant;

use aeryon_csi::ComplexSample;
use num_complex::Complex;

use crate::errors::{AntennaLink, CalibrationError};
use crate::report::{CalibrationWarning, StageDiagnostics, StageReport};
use crate::stage::{CalibrationBuffer, CalibrationContext, CalibrationStage, CalibrationStageId};
use crate::stages::phase_unwrap::duration_ns;

/// Per-link affine least-squares phase detrending.
///
/// This is a baseline phase-sanitization stage. It is **not** full
/// hardware-specific CSI phase calibration.
///
/// # Single-subcarrier policy
///
/// Links with fewer than two subcarriers cannot define a unique linear trend.
/// Those links are left unchanged and emit a structured warning. Slope and
/// intercept are recorded as zero for such links in the aggregate diagnostics.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinearPhaseDetrendStage;

impl CalibrationStage for LinearPhaseDetrendStage {
    fn id(&self) -> CalibrationStageId {
        CalibrationStageId::LinearPhaseDetrend
    }

    fn apply(
        &self,
        buffer: &mut CalibrationBuffer,
        context: &CalibrationContext,
    ) -> Result<StageReport, CalibrationError> {
        let started = Instant::now();
        let indices = buffer.subcarrier_indices().to_vec();
        let mut links_processed = 0_u32;
        let mut links_unchanged = 0_u32;
        let mut abs_slope_sum = 0.0_f64;
        let mut abs_intercept_sum = 0.0_f64;
        let mut fitted = 0_u32;
        let mut warnings = Vec::new();

        buffer.for_each_link_mut(|link, samples, phases| {
            let result = detrend_link(samples, phases, &indices, context, link)?;
            links_processed = links_processed.saturating_add(1);
            if result.unchanged {
                links_unchanged = links_unchanged.saturating_add(1);
                warnings.push(CalibrationWarning::SingleSubcarrierLinkUnchanged {
                    stage: CalibrationStageId::LinearPhaseDetrend,
                    rx: link.rx,
                    tx: link.tx,
                });
            } else {
                abs_slope_sum += result.slope.abs();
                abs_intercept_sum += result.intercept.abs();
                fitted = fitted.saturating_add(1);
            }
            Ok(())
        })?;

        let mean_abs_slope = if fitted == 0 {
            0.0
        } else {
            abs_slope_sum / f64::from(fitted)
        };
        let mean_abs_intercept = if fitted == 0 {
            0.0
        } else {
            abs_intercept_sum / f64::from(fitted)
        };

        let warning_count = warnings.len() as u32;
        Ok(StageReport {
            stage_id: self.id(),
            stage_name: self.name().to_owned(),
            order: 0,
            duration_ns: duration_ns(started),
            success: true,
            warning_count,
            warnings,
            diagnostics: StageDiagnostics::LinearPhaseDetrend {
                links_processed,
                links_unchanged,
                mean_abs_slope,
                mean_abs_intercept,
            },
        })
    }
}

struct DetrendResult {
    slope: f64,
    intercept: f64,
    unchanged: bool,
}

fn detrend_link(
    samples: &mut [ComplexSample],
    phases: &mut [f32],
    indices: &[i16],
    context: &CalibrationContext,
    link: AntennaLink,
) -> Result<DetrendResult, CalibrationError> {
    if samples.len() != indices.len() || samples.len() != phases.len() {
        return Err(CalibrationError::MalformedFrame {
            frame_id: Some(context.frame_id),
            sequence: Some(context.sequence),
            message: "subcarrier index count does not match link samples".to_owned(),
        });
    }

    if samples.len() < 2 {
        return Ok(DetrendResult {
            slope: 0.0,
            intercept: 0.0,
            unchanged: true,
        });
    }

    let mut xs = Vec::with_capacity(samples.len());
    let mut ys = Vec::with_capacity(samples.len());
    let mut magnitudes = Vec::with_capacity(samples.len());

    for (offset, sample) in samples.iter().enumerate() {
        if !sample.re.is_finite() || !sample.im.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::LinearPhaseDetrend),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        let magnitude = sample.norm();
        let phase = phases[offset];
        if !magnitude.is_finite() || !phase.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::LinearPhaseDetrend),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        xs.push(f64::from(indices[offset]));
        ys.push(f64::from(phase));
        magnitudes.push(magnitude);
    }

    let (slope, intercept) =
        fit_affine(&xs, &ys).map_err(|message| CalibrationError::DegenerateRegression {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: CalibrationStageId::LinearPhaseDetrend,
            link,
            message,
        })?;

    for (offset, sample) in samples.iter_mut().enumerate() {
        let residual = ys[offset] - (slope * xs[offset] + intercept);
        let residual_f32 = residual as f32;
        if !residual_f32.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::LinearPhaseDetrend),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        *sample = Complex::from_polar(magnitudes[offset], residual_f32);
        phases[offset] = residual_f32;
    }

    Ok(DetrendResult {
        slope,
        intercept,
        unchanged: false,
    })
}

/// Numerically stable affine least-squares fit for modest vector lengths.
fn fit_affine(xs: &[f64], ys: &[f64]) -> Result<(f64, f64), String> {
    let n = xs.len();
    if n != ys.len() || n < 2 {
        return Err("need at least two samples for affine fit".to_owned());
    }

    let n_f = n as f64;
    let mean_x = xs.iter().sum::<f64>() / n_f;
    let mean_y = ys.iter().sum::<f64>() / n_f;

    let mut ss_xx = 0.0_f64;
    let mut ss_xy = 0.0_f64;
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        let dx = x - mean_x;
        ss_xx += dx * dx;
        ss_xy += dx * (y - mean_y);
    }

    if !ss_xx.is_finite() || !ss_xy.is_finite() {
        return Err("non-finite sums during regression".to_owned());
    }
    if ss_xx <= f64::EPSILON {
        return Err("insufficient variation in subcarrier indices".to_owned());
    }

    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    if !slope.is_finite() || !intercept.is_finite() {
        return Err("non-finite slope or intercept".to_owned());
    }
    Ok((slope, intercept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::CalibrationBuffer;
    use crate::stages::PhaseUnwrapStage;
    use aeryon_csi::{CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};
    use num_complex::Complex32;

    const TOL: f32 = 1e-4;

    fn frame(samples: Vec<Complex32>, indices: Vec<i16>, rx: u16, tx: u16) -> CsiFrame {
        CsiFrame::try_new(
            FrameMetadata {
                frame_id: FrameId::new(7),
                sensor_id: SensorId::new(2),
                timestamp: Timestamp::from_nanos(1),
                sequence: 3,
                mission_id: None,
                metadata: Metadata::new(),
            },
            Timestamp::from_nanos(2),
            None,
            None,
            rx,
            tx,
            indices,
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("frame")
    }

    fn run(frame: &CsiFrame) -> CalibrationBuffer {
        let mut buffer = CalibrationBuffer::from_frame(frame).expect("buffer");
        let context = buffer.context();
        // Ensure continuous phase when constructing from from_polar of large trends.
        PhaseUnwrapStage
            .apply(&mut buffer, &context)
            .expect("unwrap");
        LinearPhaseDetrendStage
            .apply(&mut buffer, &context)
            .expect("detrend");
        buffer
    }

    fn phases_from_linear(indices: &[i16], slope: f32, intercept: f32) -> Vec<Complex32> {
        indices
            .iter()
            .map(|x| {
                let phase = slope * f32::from(*x) + intercept;
                Complex32::new(phase.cos(), phase.sin())
            })
            .collect()
    }

    #[test]
    fn exact_positive_linear_trend_becomes_near_zero() {
        let indices = vec![-4, -2, 0, 2, 4];
        let samples = phases_from_linear(&indices, 0.35, 0.0);
        let buffer = run(&frame(samples, indices, 1, 1));
        for phase in buffer.phases() {
            assert!(phase.abs() < TOL, "phase={phase}");
        }
        for sample in buffer.samples() {
            assert!((sample.norm() - 1.0).abs() < TOL);
        }
    }

    #[test]
    fn exact_negative_linear_trend() {
        let indices = vec![0, 1, 2, 3];
        let samples = phases_from_linear(&indices, -0.4, 0.0);
        let buffer = run(&frame(samples, indices, 1, 1));
        for phase in buffer.phases() {
            assert!(phase.abs() < TOL);
        }
    }

    #[test]
    fn linear_trend_with_constant_offset() {
        let indices = vec![-3, -1, 1, 3];
        let samples = phases_from_linear(&indices, 0.2, 1.1);
        let buffer = run(&frame(samples, indices, 1, 1));
        for phase in buffer.phases() {
            assert!(phase.abs() < TOL, "phase={phase}");
        }
    }

    #[test]
    fn non_uniform_subcarrier_indices() {
        let indices = vec![-8, -2, 1, 7];
        let samples = phases_from_linear(&indices, 0.15, -0.5);
        let buffer = run(&frame(samples, indices, 1, 1));
        for phase in buffer.phases() {
            assert!(phase.abs() < TOL);
        }
    }

    #[test]
    fn independent_links_with_different_slopes() {
        let indices = vec![0, 1, 2, 3];
        let mut samples = phases_from_linear(&indices, 0.3, 0.0);
        samples.extend(phases_from_linear(&indices, -0.25, 0.7));
        let buffer = run(&frame(samples, indices, 2, 1));
        for phase in buffer.phases() {
            assert!(phase.abs() < TOL, "phase={phase}");
        }
    }

    #[test]
    fn magnitude_preserved() {
        let indices = vec![0, 2, 4];
        let samples: Vec<_> = indices
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let phase = 0.5 * f32::from(*x) + 0.1;
                let mag = 1.0 + i as f32;
                Complex32::new(mag * phase.cos(), mag * phase.sin())
            })
            .collect();
        let expected_mags: Vec<_> = samples.iter().map(|s| s.norm()).collect();
        let buffer = run(&frame(samples, indices, 1, 1));
        for (sample, expected) in buffer.samples().iter().zip(expected_mags) {
            assert!((sample.norm() - expected).abs() < TOL);
        }
    }

    #[test]
    fn single_subcarrier_left_unchanged() {
        let frame = frame(vec![Complex32::from_polar(2.0, 0.77)], vec![5], 1, 1);
        let mut buffer = CalibrationBuffer::from_frame(&frame).expect("buffer");
        let context = buffer.context();
        LinearPhaseDetrendStage
            .apply(&mut buffer, &context)
            .expect("detrend");
        assert!((buffer.phases()[0] - 0.77).abs() < TOL);
        assert!((buffer.samples()[0].norm() - 2.0).abs() < TOL);
    }

    #[test]
    fn degenerate_identical_x_rejected_when_forced() {
        let error = fit_affine(&[1.0, 1.0, 1.0], &[0.1, 0.2, 0.3]).expect_err("degenerate");
        assert!(error.contains("insufficient variation"));
    }

    #[test]
    fn finite_output() {
        let indices = vec![-1, 0, 1];
        let samples = phases_from_linear(&indices, 0.1, -0.2);
        let buffer = run(&frame(samples, indices, 1, 1));
        for sample in buffer.samples() {
            assert!(sample.re.is_finite() && sample.im.is_finite());
        }
    }
}
