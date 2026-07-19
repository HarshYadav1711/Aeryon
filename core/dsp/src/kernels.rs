//! Pure numerical DSP kernels shared by the Rust reference backend.
//!
//! These functions operate on flat buffers only — no domain objects.

use crate::errors::DspError;

/// Per-link motion-energy on flattened `[frame][subcarrier]` f32 real/imag buffers.
///
/// ```text
/// energy[t] = sqrt( mean_k |H[t,k] − H[t−1,k]|² )
/// ```
///
/// Output length is `frame_count - 1`.
pub fn motion_energy_link(
    real_samples: &[f32],
    imag_samples: &[f32],
    frame_count: usize,
    subcarrier_count: usize,
) -> Result<Vec<f64>, DspError> {
    if frame_count < 2 {
        return Err(DspError::MotionEnergy {
            message: "motion-energy requires at least two frames".to_owned(),
        });
    }
    if subcarrier_count == 0 {
        return Err(DspError::MotionEnergy {
            message: "motion-energy requires at least one subcarrier".to_owned(),
        });
    }
    let expected =
        frame_count
            .checked_mul(subcarrier_count)
            .ok_or_else(|| DspError::MotionEnergy {
                message: "motion-energy sample count overflow".to_owned(),
            })?;
    if real_samples.len() != expected || imag_samples.len() != expected {
        return Err(DspError::MotionEnergy {
            message: format!(
                "motion-energy dimension mismatch: expected {expected} samples, \
                 real={}, imag={}",
                real_samples.len(),
                imag_samples.len()
            ),
        });
    }
    if real_samples.iter().any(|v| !v.is_finite()) || imag_samples.iter().any(|v| !v.is_finite()) {
        return Err(DspError::MotionEnergy {
            message: "motion-energy input contains non-finite values".to_owned(),
        });
    }

    let mut values = Vec::with_capacity(frame_count - 1);
    let inv_sc = 1.0 / subcarrier_count as f64;
    for t in 1..frame_count {
        let prev_base = (t - 1) * subcarrier_count;
        let curr_base = t * subcarrier_count;
        let mut sum_sq = 0.0_f64;
        for k in 0..subcarrier_count {
            // Widen before subtract so the Rust reference matches the C++ kernel.
            let dr =
                f64::from(real_samples[curr_base + k]) - f64::from(real_samples[prev_base + k]);
            let di =
                f64::from(imag_samples[curr_base + k]) - f64::from(imag_samples[prev_base + k]);
            sum_sq += dr * dr + di * di;
        }
        let energy = (sum_sq * inv_sc).sqrt();
        if !energy.is_finite() {
            return Err(DspError::MotionEnergy {
                message: "motion-energy produced a non-finite value".to_owned(),
            });
        }
        values.push(energy);
    }
    Ok(values)
}

/// Subtracts the arithmetic mean, then applies the symmetric Hann window.
///
/// Hann convention (`N > 1`):
///
/// ```text
/// w[n] = 0.5 × (1 − cos(2πn / (N − 1)))
/// ```
///
/// One-element policy: output is `[0.0]` (mean removal yields zero; weight is 1).
/// Empty input is rejected.
pub fn center_and_apply_hann(signal: &[f64]) -> Result<Vec<f64>, DspError> {
    if signal.is_empty() {
        return Err(DspError::Spectral {
            message: "center/Hann requires at least one sample".to_owned(),
            code: crate::errors::DspFailureCode::InsufficientLength,
        });
    }
    if signal.iter().any(|value| !value.is_finite()) {
        return Err(DspError::Spectral {
            message: "center/Hann input contains non-finite values".to_owned(),
            code: crate::errors::DspFailureCode::NonFinite,
        });
    }

    let n = signal.len();
    let mean = signal.iter().sum::<f64>() / n as f64;
    if n == 1 {
        return Ok(vec![0.0]);
    }

    let window = crate::spectral::hann_window(n);
    let mut output = Vec::with_capacity(n);
    for (value, weight) in signal.iter().zip(window.iter()) {
        let centered = value - mean;
        let sample = centered * weight;
        if !sample.is_finite() {
            return Err(DspError::Spectral {
                message: "center/Hann produced a non-finite value".to_owned(),
                code: crate::errors::DspFailureCode::NonFinite,
            });
        }
        output.push(sample);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_energy_sqrt2() {
        let out = motion_energy_link(&[1.0, 0.0], &[0.0, 1.0], 2, 1).expect("ok");
        assert!((out[0] - std::f64::consts::SQRT_2).abs() < 1e-12);
    }

    #[test]
    fn center_hann_constant() {
        let out = center_and_apply_hann(&[2.5; 16]).expect("ok");
        assert!(out.iter().all(|v| v.abs() < 1e-12));
    }

    #[test]
    fn one_element_policy() {
        let out = center_and_apply_hann(&[9.0]).expect("ok");
        assert_eq!(out, vec![0.0]);
    }
}
