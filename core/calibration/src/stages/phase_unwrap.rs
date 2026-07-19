//! Spatial phase unwrapping across ordered subcarriers.

use std::f32::consts::PI;
use std::time::Instant;

use aeryon_csi::ComplexSample;
use num_complex::Complex;

use crate::errors::{AntennaLink, CalibrationError};
use crate::report::{StageDiagnostics, StageReport};
use crate::stage::{CalibrationBuffer, CalibrationContext, CalibrationStage, CalibrationStageId};

/// Phase unwrapping across ordered subcarriers for each antenna link.
///
/// This is **spatial** unwrapping along the subcarrier axis of a single frame.
/// It does **not** perform temporal phase tracking across frames.
///
/// Continuous unwrapped phases are retained in the working buffer's phase plane
/// so later stages observe continuity (complex `arg()` alone cannot).
#[derive(Debug, Default, Clone, Copy)]
pub struct PhaseUnwrapStage;

impl CalibrationStage for PhaseUnwrapStage {
    fn id(&self) -> CalibrationStageId {
        CalibrationStageId::PhaseUnwrap
    }

    fn apply(
        &self,
        buffer: &mut CalibrationBuffer,
        context: &CalibrationContext,
    ) -> Result<StageReport, CalibrationError> {
        let started = Instant::now();
        let mut links_processed = 0_u32;
        let mut wraps_applied = 0_u64;

        buffer.for_each_link_mut(|link, samples, phases| {
            let wraps = unwrap_link(samples, phases, context, link)?;
            wraps_applied = wraps_applied.saturating_add(wraps);
            links_processed = links_processed.saturating_add(1);
            Ok(())
        })?;

        Ok(StageReport {
            stage_id: self.id(),
            stage_name: self.name().to_owned(),
            order: 0,
            duration_ns: duration_ns(started),
            success: true,
            warning_count: 0,
            warnings: Vec::new(),
            diagnostics: StageDiagnostics::PhaseUnwrap {
                links_processed,
                wraps_applied,
            },
        })
    }
}

fn unwrap_link(
    samples: &mut [ComplexSample],
    phases: &mut [f32],
    context: &CalibrationContext,
    link: AntennaLink,
) -> Result<u64, CalibrationError> {
    if samples.is_empty() {
        return Ok(0);
    }
    if samples.len() != phases.len() {
        return Err(CalibrationError::MalformedFrame {
            frame_id: Some(context.frame_id),
            sequence: Some(context.sequence),
            message: "sample/phase length mismatch".to_owned(),
        });
    }

    let mut wraps = 0_u64;
    // Seed from continuous phase plane (initialized from principal arg).
    let mut previous_unwrapped = phases[0];
    if !previous_unwrapped.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::PhaseUnwrap),
            sample_index: Some(0),
            link: Some(link),
        });
    }

    let magnitude0 = sample_magnitude(samples[0], context, 0, link)?;
    samples[0] = Complex::from_polar(magnitude0, previous_unwrapped);
    phases[0] = previous_unwrapped;

    for offset in 1..samples.len() {
        let magnitude = sample_magnitude(samples[offset], context, offset, link)?;
        // Use current principal phase of the complex sample as the wrapped observation.
        let wrapped = sample_phase(samples[offset], context, offset, link)?;
        let mut delta = wrapped - previous_unwrapped;
        while delta > PI {
            delta -= 2.0 * PI;
            wraps = wraps.saturating_add(1);
        }
        while delta < -PI {
            delta += 2.0 * PI;
            wraps = wraps.saturating_add(1);
        }
        let unwrapped = previous_unwrapped + delta;
        if !unwrapped.is_finite() || !magnitude.is_finite() {
            return Err(CalibrationError::NonFiniteSample {
                frame_id: context.frame_id,
                sequence: context.sequence,
                stage: Some(CalibrationStageId::PhaseUnwrap),
                sample_index: Some(offset),
                link: Some(link),
            });
        }
        samples[offset] = Complex::from_polar(magnitude, unwrapped);
        phases[offset] = unwrapped;
        previous_unwrapped = unwrapped;
    }

    Ok(wraps)
}

fn sample_phase(
    sample: ComplexSample,
    context: &CalibrationContext,
    offset: usize,
    link: AntennaLink,
) -> Result<f32, CalibrationError> {
    if !sample.re.is_finite() || !sample.im.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::PhaseUnwrap),
            sample_index: Some(offset),
            link: Some(link),
        });
    }
    let phase = sample.arg();
    if !phase.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::PhaseUnwrap),
            sample_index: Some(offset),
            link: Some(link),
        });
    }
    Ok(phase)
}

fn sample_magnitude(
    sample: ComplexSample,
    context: &CalibrationContext,
    offset: usize,
    link: AntennaLink,
) -> Result<f32, CalibrationError> {
    if !sample.re.is_finite() || !sample.im.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::PhaseUnwrap),
            sample_index: Some(offset),
            link: Some(link),
        });
    }
    let magnitude = sample.norm();
    if !magnitude.is_finite() {
        return Err(CalibrationError::NonFiniteSample {
            frame_id: context.frame_id,
            sequence: context.sequence,
            stage: Some(CalibrationStageId::PhaseUnwrap),
            sample_index: Some(offset),
            link: Some(link),
        });
    }
    Ok(magnitude)
}

pub(crate) fn duration_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::CalibrationBuffer;
    use aeryon_csi::{CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};
    use num_complex::Complex32;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn frame_from_link(samples: Vec<Complex32>, indices: Vec<i16>) -> CsiFrame {
        CsiFrame::try_new(
            FrameMetadata {
                frame_id: FrameId::new(1),
                sensor_id: SensorId::new(2),
                timestamp: Timestamp::from_nanos(10),
                sequence: 0,
                mission_id: None,
                metadata: Metadata::new(),
            },
            Timestamp::from_nanos(20),
            Some(5_180_000_000.0),
            Some(20_000_000.0),
            1,
            1,
            indices,
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("frame")
    }

    fn two_link_frame(link0: Vec<Complex32>, link1: Vec<Complex32>, indices: Vec<i16>) -> CsiFrame {
        let mut samples = link0;
        samples.extend(link1);
        CsiFrame::try_new(
            FrameMetadata {
                frame_id: FrameId::new(1),
                sensor_id: SensorId::new(2),
                timestamp: Timestamp::from_nanos(10),
                sequence: 0,
                mission_id: None,
                metadata: Metadata::new(),
            },
            Timestamp::from_nanos(20),
            None,
            None,
            2,
            1,
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
        PhaseUnwrapStage
            .apply(&mut buffer, &context)
            .expect("unwrap");
        buffer
    }

    #[test]
    fn no_wrap_preserves_phases() {
        let frame = frame_from_link(
            vec![
                Complex32::from_polar(1.0, 0.1),
                Complex32::from_polar(1.0, 0.2),
                Complex32::from_polar(1.0, 0.3),
            ],
            vec![0, 1, 2],
        );
        let buffer = run(&frame);
        let phases = buffer.phases();
        assert!((phases[0] - 0.1).abs() < 1e-5);
        assert!((phases[1] - 0.2).abs() < 1e-5);
        assert!((phases[2] - 0.3).abs() < 1e-5);
    }

    #[test]
    fn positive_pi_crossing_unwraps() {
        let frame = frame_from_link(
            vec![
                Complex32::from_polar(1.0, 2.5),
                Complex32::from_polar(1.0, -2.5),
            ],
            vec![0, 1],
        );
        let buffer = run(&frame);
        let phases = buffer.phases();
        let delta = phases[1] - phases[0];
        assert!(delta > 0.0 && delta < PI, "delta={delta}");
        assert!((buffer.samples()[0].norm() - 1.0).abs() < 1e-6);
        assert!((buffer.samples()[1].norm() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn negative_pi_crossing_unwraps() {
        let frame = frame_from_link(
            vec![
                Complex32::from_polar(1.0, -2.5),
                Complex32::from_polar(1.0, 2.5),
            ],
            vec![0, 1],
        );
        let buffer = run(&frame);
        let phases = buffer.phases();
        let delta = phases[1] - phases[0];
        assert!(delta < 0.0 && delta > -PI, "delta={delta}");
    }

    #[test]
    fn multiple_wraps_accumulate() {
        let true_phases = [0.0_f32, 2.0, 4.0, 6.0, 8.0];
        let samples: Vec<_> = true_phases
            .iter()
            .map(|p| Complex32::new(1.5 * p.cos(), 1.5 * p.sin()))
            .collect();
        let frame = frame_from_link(samples, vec![0, 1, 2, 3, 4]);
        let buffer = run(&frame);
        let phases = buffer.phases();
        for (idx, expected) in true_phases.iter().enumerate() {
            assert!(
                (phases[idx] - expected).abs() < 1e-4,
                "idx={idx} got={} expected={expected}",
                phases[idx]
            );
            assert!((buffer.samples()[idx].norm() - 1.5).abs() < 1e-5);
        }
    }

    #[test]
    fn magnitude_preserved() {
        let frame = frame_from_link(
            vec![
                Complex32::from_polar(2.0, 2.8),
                Complex32::from_polar(3.0, -2.8),
            ],
            vec![-1, 1],
        );
        let buffer = run(&frame);
        assert!((buffer.samples()[0].norm() - 2.0).abs() < 1e-5);
        assert!((buffer.samples()[1].norm() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn one_subcarrier_succeeds() {
        let frame = frame_from_link(vec![Complex32::from_polar(1.0, -1.2)], vec![0]);
        let buffer = run(&frame);
        assert_eq!(buffer.samples().len(), 1);
        assert!((buffer.phases()[0] + 1.2).abs() < 1e-5);
    }

    #[test]
    fn links_are_independent() {
        let frame = two_link_frame(
            vec![
                Complex32::from_polar(1.0, 2.5),
                Complex32::from_polar(1.0, -2.5),
            ],
            vec![
                Complex32::from_polar(1.0, -2.5),
                Complex32::from_polar(1.0, 2.5),
            ],
            vec![0, 1],
        );
        let buffer = run(&frame);
        let phases = buffer.phases();
        let p0 = phases[1] - phases[0];
        let p1 = phases[3] - phases[2];
        assert!(p0 > 0.0, "p0={p0}");
        assert!(p1 < 0.0, "p1={p1}");
    }

    #[test]
    fn non_finite_rejected() {
        let frame = frame_from_link(
            vec![Complex32::new(1.0, 0.0), Complex32::new(0.0, 1.0)],
            vec![0, 1],
        );
        let mut buffer = CalibrationBuffer::from_frame(&frame).expect("buffer");
        buffer.set_sample_for_test(1, Complex32::new(f32::NAN, 0.0));
        let context = buffer.context();
        let error = PhaseUnwrapStage
            .apply(&mut buffer, &context)
            .expect_err("nan");
        assert!(matches!(error, CalibrationError::NonFiniteSample { .. }));
    }

    #[test]
    fn deterministic_output() {
        let frame = frame_from_link(
            vec![
                Complex32::new(1.0, 0.0),
                Complex32::new(0.0, 1.0),
                Complex32::new(-1.0, 0.0),
            ],
            vec![0, 1, 2],
        );
        let a = run(&frame);
        let b = run(&frame);
        assert_eq!(a.samples(), b.samples());
        assert_eq!(a.phases(), b.phases());
        assert!((a.phases()[1] - FRAC_PI_2).abs() < 1e-5);
        assert!((a.phases()[2] - PI).abs() < 1e-5);
    }
}
