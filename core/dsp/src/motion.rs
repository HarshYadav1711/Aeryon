//! CSI motion-energy proxy from consecutive calibrated frames.
//!
//! # Semantics
//!
//! `motion_energy[t]` measures channel change between consecutive calibrated
//! CSI matrices. It is a normalized complex-difference energy proxy — not
//! human motion, occupancy, activity, or velocity.

use std::sync::Arc;

use aeryon_calibration::{AntennaLink, CalibratedCsiFrame};

use crate::backend::{DspKernelBackend, MotionEnergyInput, RustKernelBackend};
use crate::errors::DspError;
use crate::window::CsiWindow;

/// Per-link motion-energy proxy series for one temporal window.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkMotionEnergy {
    /// Antenna link identity.
    pub link: AntennaLink,
    /// One value per consecutive frame transition (length = frame_count − 1).
    pub values: Vec<f64>,
}

/// Motion-energy proxy outputs for every RX–TX link plus an optional aggregate.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionEnergySignal {
    /// Independent per-link series.
    pub links: Vec<LinkMotionEnergy>,
    /// Mean across links at each transition, when at least one link exists.
    pub aggregate: Option<Vec<f64>>,
}

/// Calculates the motion-energy proxy using the Rust reference backend.
pub fn compute_motion_energy(window: &CsiWindow) -> Result<MotionEnergySignal, DspError> {
    compute_motion_energy_with_backend(window, &RustKernelBackend)
}

/// Calculates the motion-energy proxy using the provided kernel backend.
pub fn compute_motion_energy_with_backend(
    window: &CsiWindow,
    backend: &dyn DspKernelBackend,
) -> Result<MotionEnergySignal, DspError> {
    if window.frame_count() < 2 {
        return Err(DspError::MotionEnergy {
            message: "motion-energy requires at least two frames".to_owned(),
        });
    }

    let frames = window.frames();
    let n_sc = window.subcarrier_indices().len();
    if n_sc == 0 {
        return Err(DspError::MotionEnergy {
            message: "motion-energy requires at least one subcarrier".to_owned(),
        });
    }

    let mut links = Vec::new();
    for rx in 0..window.receive_antennas() {
        for tx in 0..window.transmit_antennas() {
            let (real, imag) = flatten_link_samples(frames, rx, tx, n_sc)?;
            let values = backend.motion_energy(MotionEnergyInput {
                real_samples: &real,
                imag_samples: &imag,
                frame_count: frames.len(),
                subcarrier_count: n_sc,
            })?;
            links.push(LinkMotionEnergy {
                link: AntennaLink::new(rx, tx),
                values,
            });
        }
    }

    let aggregate = if links.is_empty() {
        None
    } else {
        let transitions = links[0].values.len();
        let mut aggregate = Vec::with_capacity(transitions);
        for index in 0..transitions {
            let mean =
                links.iter().map(|link| link.values[index]).sum::<f64>() / links.len() as f64;
            if !mean.is_finite() {
                return Err(DspError::MotionEnergy {
                    message: "aggregate motion-energy produced a non-finite value".to_owned(),
                });
            }
            aggregate.push(mean);
        }
        Some(aggregate)
    };

    Ok(MotionEnergySignal { links, aggregate })
}

fn flatten_link_samples(
    frames: &[Arc<CalibratedCsiFrame>],
    rx: u16,
    tx: u16,
    n_sc: usize,
) -> Result<(Vec<f32>, Vec<f32>), DspError> {
    let mut real = Vec::with_capacity(frames.len() * n_sc);
    let mut imag = Vec::with_capacity(frames.len() * n_sc);
    for frame in frames {
        let link = frame.link(rx, tx).ok_or_else(|| DspError::MotionEnergy {
            message: format!("missing calibrated link rx={rx} tx={tx}"),
        })?;
        if link.len() != n_sc {
            return Err(DspError::MotionEnergy {
                message: "link sample length mismatch".to_owned(),
            });
        }
        for sample in link {
            if !sample.re.is_finite() || !sample.im.is_finite() {
                return Err(DspError::MotionEnergy {
                    message: "calibrated sample contains non-finite values".to_owned(),
                });
            }
            real.push(sample.re);
            imag.push(sample.im);
        }
    }
    Ok((real, imag))
}

/// Convenience helper used by tests: motion energy for an ordered frame list.
pub fn compute_motion_energy_frames(
    frames: &[Arc<CalibratedCsiFrame>],
) -> Result<MotionEnergySignal, DspError> {
    let window = CsiWindow::try_new(0, frames.to_vec())?;
    compute_motion_energy(&window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_calibration::{CalibrationPipeline, baseline_csi_v1};
    use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};

    fn frame_with_samples(
        sequence: u64,
        samples: Vec<ComplexSample>,
        rx: u16,
        tx: u16,
        n_sc: usize,
    ) -> Arc<CalibratedCsiFrame> {
        let indices: Vec<i16> = (0..n_sc as i16).collect();
        let metadata = FrameMetadata {
            frame_id: FrameId::new(sequence + 1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(sequence * 100),
            sequence,
            mission_id: None,
            metadata: Metadata::new(),
        };
        let raw = CsiFrame::try_new(
            metadata,
            Timestamp::from_nanos(sequence * 100),
            None,
            None,
            rx,
            tx,
            indices,
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("raw");
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        Arc::new(pipeline.calibrate(Arc::new(raw)).expect("calibrated"))
    }

    #[test]
    fn identical_frames_produce_zero_energy() {
        let samples = vec![ComplexSample::new(1.0, 0.5); 4];
        let a = frame_with_samples(0, samples.clone(), 1, 1, 4);
        let b = frame_with_samples(1, samples, 1, 1, 4);
        let signal = compute_motion_energy_frames(&[a, b]).expect("energy");
        assert_eq!(signal.links.len(), 1);
        assert!((signal.links[0].values[0]).abs() < 1e-6);
    }

    #[test]
    fn known_real_and_imaginary_differences() {
        let a = frame_with_samples(0, vec![ComplexSample::new(2.0, 0.0)], 1, 1, 1);
        let b = frame_with_samples(1, vec![ComplexSample::new(0.0, 2.0)], 1, 1, 1);
        let signal = compute_motion_energy_frames(&[a, b]).expect("energy");
        assert!((signal.links[0].values[0] - std::f64::consts::SQRT_2).abs() < 1e-5);
    }

    #[test]
    fn independent_links_and_aggregate() {
        let samples_a = vec![
            ComplexSample::new(1.0, 0.0),
            ComplexSample::new(1.0, 0.0),
            ComplexSample::new(1.0, 0.0),
            ComplexSample::new(1.0, 0.0),
        ];
        let samples_b = vec![
            ComplexSample::new(0.0, 1.0),
            ComplexSample::new(0.0, 1.0),
            ComplexSample::new(1.0, 0.0),
            ComplexSample::new(1.0, 0.0),
        ];
        let a = frame_with_samples(0, samples_a, 2, 1, 2);
        let b = frame_with_samples(1, samples_b, 2, 1, 2);
        let signal = compute_motion_energy_frames(&[a, b]).expect("energy");
        assert_eq!(signal.links.len(), 2);
        assert_eq!(signal.links[0].link.rx, 0);
        assert_eq!(signal.links[1].link.rx, 1);
        assert!(signal.links[0].values[0].is_finite());
        assert!(signal.links[1].values[0].is_finite());
        assert!(signal.links[1].values[0].abs() < 1e-5);
        let aggregate = signal.aggregate.expect("aggregate");
        assert_eq!(aggregate.len(), 1);
        let expected = (signal.links[0].values[0] + signal.links[1].values[0]) / 2.0;
        assert!((aggregate[0] - expected).abs() < 1e-9);
    }
}
