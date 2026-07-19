//! Deterministic numerical helpers for feature extraction.
//!
//! # Conventions
//!
//! - **Standard deviation:** population (`N` denominator), not sample (`N-1`).
//! - **Percentiles:** linear interpolation on a sorted copy (`method = linear`).
//! - One-value inputs: mean/median/min/max/rms equal the value; stddev is `0`;
//!   mean absolute delta is `0`; percentiles equal the value.
//! - Empty inputs are rejected. Non-finite inputs are rejected.
//! - Source slices are never sorted in place.

use crate::errors::FeatureError;

/// Validates that every sample is finite and the series is non-empty.
pub fn require_finite_non_empty(values: &[f64], context: &str) -> Result<(), FeatureError> {
    if values.is_empty() {
        return Err(FeatureError::EmptySignal {
            context: context.to_owned(),
        });
    }
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(FeatureError::NonFiniteInput {
                context: context.to_owned(),
                message: format!("non-finite sample at index {index}"),
            });
        }
    }
    Ok(())
}

/// Arithmetic mean.
pub fn mean(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    let sum: f64 = values.iter().sum();
    Ok(sum / values.len() as f64)
}

/// Population standard deviation (uses `N`, not `N-1`).
pub fn population_std(values: &[f64]) -> Result<f64, FeatureError> {
    let mean = mean(values)?;
    if values.len() == 1 {
        return Ok(0.0);
    }
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    let std = variance.sqrt();
    if !std.is_finite() {
        return Err(FeatureError::NonFiniteInput {
            context: String::new(),
            message: "population standard deviation is non-finite".to_owned(),
        });
    }
    Ok(std)
}

/// Root-mean-square.
pub fn rms(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    let mean_square = values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64;
    let result = mean_square.sqrt();
    if !result.is_finite() {
        return Err(FeatureError::NonFiniteInput {
            context: String::new(),
            message: "rms is non-finite".to_owned(),
        });
    }
    Ok(result)
}

/// Minimum value.
pub fn min(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    Ok(values.iter().copied().fold(f64::INFINITY, f64::min))
}

/// Maximum value.
pub fn max(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max))
}

/// Median via sorted copy (average of two central values for even length).
pub fn median(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Ok((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Ok(sorted[mid])
    }
}

/// Linear-interpolated percentile in `[0, 100]` on a sorted copy.
///
/// Position is `p/100 * (n - 1)`. Values between adjacent ranks are linearly
/// interpolated. For `n == 1`, returns the single sample for every percentile.
pub fn percentile(values: &[f64], percentile: f64) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    if !percentile.is_finite() || !(0.0..=100.0).contains(&percentile) {
        return Err(FeatureError::NonFiniteInput {
            context: String::new(),
            message: format!("percentile {percentile} is outside [0, 100]"),
        });
    }
    if values.len() == 1 {
        return Ok(values[0]);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = (percentile / 100.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        Ok(sorted[lower])
    } else {
        let weight = rank - lower as f64;
        Ok(sorted[lower] * (1.0 - weight) + sorted[upper] * weight)
    }
}

/// Maximum − minimum.
pub fn range(values: &[f64]) -> Result<f64, FeatureError> {
    Ok(max(values)? - min(values)?)
}

/// Mean absolute consecutive difference.
///
/// For a single sample, returns `0.0`.
pub fn mean_absolute_delta(values: &[f64]) -> Result<f64, FeatureError> {
    require_finite_non_empty(values, "")?;
    if values.len() == 1 {
        return Ok(0.0);
    }
    let sum = values
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .sum::<f64>();
    Ok(sum / (values.len() - 1) as f64)
}

/// Maximum / mean. Returns `0.0` when mean is zero.
pub fn peak_to_mean_ratio(values: &[f64]) -> Result<f64, FeatureError> {
    let mean = mean(values)?;
    let peak = max(values)?;
    if mean == 0.0 {
        return Ok(0.0);
    }
    let ratio = peak / mean;
    if !ratio.is_finite() {
        return Err(FeatureError::NonFiniteInput {
            context: String::new(),
            message: "peak-to-mean ratio is non-finite".to_owned(),
        });
    }
    Ok(ratio)
}

/// Ensures a computed feature value is finite.
pub fn require_finite_output(value: f64, feature: &str) -> Result<f64, FeatureError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FeatureError::OutputValidation {
            message: format!("feature `{feature}` produced a non-finite value"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn mean_std_rms_median_basic() {
        let values = [1.0, 2.0, 3.0, 4.0];
        assert!((mean(&values).unwrap() - 2.5).abs() < EPS);
        assert!((population_std(&values).unwrap() - (1.25_f64).sqrt()).abs() < EPS);
        assert!((rms(&values).unwrap() - (7.5_f64).sqrt()).abs() < EPS);
        assert!((median(&values).unwrap() - 2.5).abs() < EPS);
        assert!((median(&[1.0, 2.0, 3.0]).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn percentiles_use_linear_interpolation() {
        let values = [10.0, 20.0, 30.0, 40.0];
        assert!((percentile(&values, 0.0).unwrap() - 10.0).abs() < EPS);
        assert!((percentile(&values, 100.0).unwrap() - 40.0).abs() < EPS);
        assert!((percentile(&values, 50.0).unwrap() - 25.0).abs() < EPS);
        assert!((percentile(&values, 90.0).unwrap() - 37.0).abs() < EPS);
        assert!((percentile(&values, 95.0).unwrap() - 38.5).abs() < EPS);
    }

    #[test]
    fn deltas_range_and_peak_ratio() {
        let values = [1.0, 3.0, 2.0];
        assert!((mean_absolute_delta(&values).unwrap() - 1.5).abs() < EPS);
        assert!((range(&values).unwrap() - 2.0).abs() < EPS);
        assert!((peak_to_mean_ratio(&values).unwrap() - 1.5).abs() < EPS);
        assert_eq!(peak_to_mean_ratio(&[0.0, 0.0]).unwrap(), 0.0);
    }

    #[test]
    fn one_value_and_empty_and_non_finite() {
        assert_eq!(mean(&[7.0]).unwrap(), 7.0);
        assert_eq!(population_std(&[7.0]).unwrap(), 0.0);
        assert_eq!(mean_absolute_delta(&[7.0]).unwrap(), 0.0);
        assert_eq!(percentile(&[7.0], 95.0).unwrap(), 7.0);
        assert!(matches!(mean(&[]), Err(FeatureError::EmptySignal { .. })));
        assert!(matches!(
            mean(&[1.0, f64::NAN]),
            Err(FeatureError::NonFiniteInput { .. })
        ));
    }

    #[test]
    fn does_not_mutate_source() {
        let values = [3.0, 1.0, 2.0];
        let _ = percentile(&values, 50.0).unwrap();
        assert_eq!(values, [3.0, 1.0, 2.0]);
    }
}
