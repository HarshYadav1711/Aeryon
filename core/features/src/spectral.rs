//! Spectral feature helpers derived from existing DSP power spectra.
//!
//! Frequency bands are relative thirds of the available non-DC frequency range.
//! They are not labeled as breathing, walking, gesture, or heartbeat bands.

use crate::errors::FeatureError;
use crate::statistics::require_finite_output;

/// Relative third-band policy over non-DC spectral bins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyBandPolicy {
    /// Low / middle / high = first / second / final third of non-DC bins.
    RelativeNonDcThirds,
}

impl FrequencyBandPolicy {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelativeNonDcThirds => "relative_non_dc_thirds",
        }
    }
}

/// Spectral statistics extracted from one power spectrum (DC excluded).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralFeatures {
    /// Sum of non-DC power.
    pub total_non_dc_power: f64,
    /// Dominant non-DC frequency in hertz (`0.0` when total power is zero).
    pub dominant_non_dc_frequency_hz: f64,
    /// Power at the dominant non-DC bin (`0.0` when total power is zero).
    pub dominant_non_dc_power: f64,
    /// Power-weighted centroid in hertz (`0.0` when total power is zero).
    pub spectral_centroid_hz: f64,
    /// Power-weighted bandwidth in hertz (`0.0` when total power is zero).
    pub spectral_bandwidth_hz: f64,
    /// Normalized spectral entropy in `[0, 1]` when defined; `0.0` for zero energy.
    pub spectral_entropy: f64,
    /// Spectral flatness in `[0, 1]` typically; `0.0` for zero energy.
    pub spectral_flatness: f64,
    /// Low-band power ratio.
    pub low_frequency_power_ratio: f64,
    /// Middle-band power ratio.
    pub middle_frequency_power_ratio: f64,
    /// High-band power ratio.
    pub high_frequency_power_ratio: f64,
}

/// Extracts spectral features from aligned frequency and power vectors.
///
/// `frequencies_hz[0]` / `power[0]` are treated as DC and excluded from analysis.
pub fn extract_spectral_features(
    frequencies_hz: &[f64],
    power: &[f64],
    flatness_epsilon: f64,
    band_policy: FrequencyBandPolicy,
) -> Result<SpectralFeatures, FeatureError> {
    if frequencies_hz.len() != power.len() {
        return Err(FeatureError::MismatchedLinkData {
            message: format!(
                "spectrum length mismatch: {} frequencies vs {} power bins",
                frequencies_hz.len(),
                power.len()
            ),
        });
    }
    if power.len() < 2 {
        return Err(FeatureError::EmptySignal {
            context: " (spectrum requires DC plus at least one non-DC bin)".to_owned(),
        });
    }
    for (index, value) in power.iter().enumerate() {
        if !value.is_finite() {
            return Err(FeatureError::NonFiniteInput {
                context: String::new(),
                message: format!("power bin {index} is non-finite"),
            });
        }
        if *value < 0.0 {
            return Err(FeatureError::InvalidPower {
                context: String::new(),
                message: format!("power bin {index} is negative"),
            });
        }
    }
    for (index, value) in frequencies_hz.iter().enumerate() {
        if !value.is_finite() {
            return Err(FeatureError::NonFiniteInput {
                context: String::new(),
                message: format!("frequency bin {index} is non-finite"),
            });
        }
    }
    if !flatness_epsilon.is_finite() || flatness_epsilon <= 0.0 {
        return Err(FeatureError::InvalidProfile {
            message: "flatness_epsilon must be finite and positive".to_owned(),
        });
    }

    let non_dc_freq = &frequencies_hz[1..];
    let non_dc_power = &power[1..];
    let total: f64 = non_dc_power.iter().sum();
    if !total.is_finite() {
        return Err(FeatureError::NonFiniteInput {
            context: String::new(),
            message: "total non-DC power is non-finite".to_owned(),
        });
    }

    if total == 0.0 {
        // Defined zero-energy policy: all spectral descriptors are zero; ratios are zero.
        return Ok(SpectralFeatures {
            total_non_dc_power: 0.0,
            dominant_non_dc_frequency_hz: 0.0,
            dominant_non_dc_power: 0.0,
            spectral_centroid_hz: 0.0,
            spectral_bandwidth_hz: 0.0,
            spectral_entropy: 0.0,
            spectral_flatness: 0.0,
            low_frequency_power_ratio: 0.0,
            middle_frequency_power_ratio: 0.0,
            high_frequency_power_ratio: 0.0,
        });
    }

    let (dominant_idx, dominant_power) = non_dc_power
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, value)| (index, *value))
        .expect("non-empty non-DC");

    let centroid = non_dc_freq
        .iter()
        .zip(non_dc_power.iter())
        .map(|(freq, p)| freq * p)
        .sum::<f64>()
        / total;

    let bandwidth = (non_dc_freq
        .iter()
        .zip(non_dc_power.iter())
        .map(|(freq, p)| {
            let delta = freq - centroid;
            p * delta * delta
        })
        .sum::<f64>()
        / total)
        .sqrt();

    let entropy = normalized_spectral_entropy(non_dc_power, total)?;
    let flatness = spectral_flatness(non_dc_power, flatness_epsilon)?;
    let (low, middle, high) = band_power_ratios(non_dc_power, band_policy, total)?;

    Ok(SpectralFeatures {
        total_non_dc_power: require_finite_output(total, "total_non_dc_power")?,
        dominant_non_dc_frequency_hz: require_finite_output(
            non_dc_freq[dominant_idx],
            "dominant_non_dc_frequency_hz",
        )?,
        dominant_non_dc_power: require_finite_output(dominant_power, "dominant_non_dc_power")?,
        spectral_centroid_hz: require_finite_output(centroid, "spectral_centroid_hz")?,
        spectral_bandwidth_hz: require_finite_output(bandwidth, "spectral_bandwidth_hz")?,
        spectral_entropy: require_finite_output(entropy, "spectral_entropy")?,
        spectral_flatness: require_finite_output(flatness, "spectral_flatness")?,
        low_frequency_power_ratio: require_finite_output(low, "low_frequency_power_ratio")?,
        middle_frequency_power_ratio: require_finite_output(
            middle,
            "middle_frequency_power_ratio",
        )?,
        high_frequency_power_ratio: require_finite_output(high, "high_frequency_power_ratio")?,
    })
}

/// Normalized spectral entropy over non-negative power values.
///
/// ```text
/// p_i = power_i / total_power
/// entropy = -Σ p_i log(p_i)   (skip zero-probability terms)
/// normalized = entropy / log(number_of_nonzero_bins)
/// ```
///
/// One valid bin → `0.0`. Zero total power is rejected here (caller uses the
/// zero-energy policy before calling).
pub fn normalized_spectral_entropy(power: &[f64], total_power: f64) -> Result<f64, FeatureError> {
    if power.is_empty() {
        return Err(FeatureError::EmptySignal {
            context: " (entropy)".to_owned(),
        });
    }
    if !total_power.is_finite() || total_power <= 0.0 {
        return Err(FeatureError::ZeroTotalPower {
            context: " (entropy)".to_owned(),
        });
    }
    let mut entropy = 0.0_f64;
    let mut nonzero = 0_usize;
    for value in power {
        if !value.is_finite() || *value < 0.0 {
            return Err(FeatureError::InvalidPower {
                context: " (entropy)".to_owned(),
                message: "power must be finite and non-negative".to_owned(),
            });
        }
        if *value == 0.0 {
            continue;
        }
        nonzero += 1;
        let p = value / total_power;
        entropy -= p * p.ln();
    }
    if nonzero == 0 {
        return Ok(0.0);
    }
    if nonzero == 1 {
        return Ok(0.0);
    }
    let denom = (nonzero as f64).ln();
    if denom == 0.0 {
        return Ok(0.0);
    }
    Ok(entropy / denom)
}

/// Spectral flatness using a log-space geometric mean and epsilon floor.
///
/// Describes tonal versus broadband structure. Does not attach human-activity meaning.
pub fn spectral_flatness(power: &[f64], epsilon: f64) -> Result<f64, FeatureError> {
    if power.is_empty() {
        return Err(FeatureError::EmptySignal {
            context: " (flatness)".to_owned(),
        });
    }
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err(FeatureError::InvalidProfile {
            message: "flatness epsilon must be finite and positive".to_owned(),
        });
    }
    let mut log_sum = 0.0_f64;
    let mut arith = 0.0_f64;
    for value in power {
        if !value.is_finite() || *value < 0.0 {
            return Err(FeatureError::InvalidPower {
                context: " (flatness)".to_owned(),
                message: "power must be finite and non-negative".to_owned(),
            });
        }
        let floored = value.max(epsilon);
        log_sum += floored.ln();
        arith += floored;
    }
    let n = power.len() as f64;
    let geometric = (log_sum / n).exp();
    let arithmetic = arith / n;
    if arithmetic == 0.0 {
        return Ok(0.0);
    }
    let flatness = geometric / arithmetic;
    if !flatness.is_finite() || flatness < 0.0 {
        return Err(FeatureError::OutputValidation {
            message: "spectral flatness is non-finite or negative".to_owned(),
        });
    }
    Ok(flatness)
}

fn band_power_ratios(
    non_dc_power: &[f64],
    policy: FrequencyBandPolicy,
    total: f64,
) -> Result<(f64, f64, f64), FeatureError> {
    let FrequencyBandPolicy::RelativeNonDcThirds = policy;
    let n = non_dc_power.len();
    if n == 0 {
        return Ok((0.0, 0.0, 0.0));
    }
    let low_end = n.div_ceil(3);
    let mid_end = (2 * n).div_ceil(3);
    let low: f64 = non_dc_power[..low_end].iter().sum();
    let middle: f64 = non_dc_power[low_end..mid_end].iter().sum();
    let high: f64 = non_dc_power[mid_end..].iter().sum();
    if total == 0.0 {
        return Ok((0.0, 0.0, 0.0));
    }
    Ok((low / total, middle / total, high / total))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn known_spectrum_features() {
        // DC + three equal non-DC bins → flat, centroid mid, ratios ~1/3 each.
        let frequencies = [0.0, 1.0, 2.0, 3.0];
        let power = [10.0, 1.0, 1.0, 1.0];
        let features = extract_spectral_features(
            &frequencies,
            &power,
            1e-12,
            FrequencyBandPolicy::RelativeNonDcThirds,
        )
        .unwrap();
        assert!((features.total_non_dc_power - 3.0).abs() < EPS);
        assert!((features.spectral_centroid_hz - 2.0).abs() < EPS);
        assert!((features.low_frequency_power_ratio - 1.0 / 3.0).abs() < EPS);
        assert!((features.middle_frequency_power_ratio - 1.0 / 3.0).abs() < EPS);
        assert!((features.high_frequency_power_ratio - 1.0 / 3.0).abs() < EPS);
        let ratio_sum = features.low_frequency_power_ratio
            + features.middle_frequency_power_ratio
            + features.high_frequency_power_ratio;
        assert!((ratio_sum - 1.0).abs() < 1e-12);
        assert!(features.spectral_entropy > 0.99);
        assert!(features.spectral_flatness > 0.99);
    }

    #[test]
    fn one_bin_entropy_is_zero() {
        assert_eq!(normalized_spectral_entropy(&[4.0], 4.0).unwrap(), 0.0);
    }

    #[test]
    fn zero_power_policy() {
        let features = extract_spectral_features(
            &[0.0, 1.0, 2.0],
            &[0.0, 0.0, 0.0],
            1e-12,
            FrequencyBandPolicy::RelativeNonDcThirds,
        )
        .unwrap();
        assert_eq!(features.total_non_dc_power, 0.0);
        assert_eq!(features.spectral_entropy, 0.0);
        assert_eq!(features.spectral_flatness, 0.0);
    }

    #[test]
    fn negative_and_non_finite_rejected() {
        assert!(matches!(
            extract_spectral_features(
                &[0.0, 1.0],
                &[0.0, -1.0],
                1e-12,
                FrequencyBandPolicy::RelativeNonDcThirds
            ),
            Err(FeatureError::InvalidPower { .. })
        ));
        assert!(matches!(
            extract_spectral_features(
                &[0.0, 1.0],
                &[0.0, f64::NAN],
                1e-12,
                FrequencyBandPolicy::RelativeNonDcThirds
            ),
            Err(FeatureError::NonFiniteInput { .. })
        ));
    }

    #[test]
    fn dominant_is_highest_power_bin() {
        let features = extract_spectral_features(
            &[0.0, 1.0, 2.0, 3.0],
            &[0.0, 0.1, 5.0, 0.2],
            1e-12,
            FrequencyBandPolicy::RelativeNonDcThirds,
        )
        .unwrap();
        assert!((features.dominant_non_dc_frequency_hz - 2.0).abs() < EPS);
        assert!((features.dominant_non_dc_power - 5.0).abs() < EPS);
    }
}
