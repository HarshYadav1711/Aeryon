//! Feature extraction from [`DspWindowResult`] into [`FeatureVector`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use aeryon_domain::Timestamp;
use aeryon_dsp::DspWindowResult;

use crate::aggregate::AggregationPolicy;
use crate::errors::FeatureError;
use crate::profile::FeatureProfile;
use crate::report::FeatureExtractionReport;
use crate::schema::{FeatureId, FeatureSchema};
use crate::spectral::extract_spectral_features;
use crate::statistics::{
    max, mean, mean_absolute_delta, median, min, peak_to_mean_ratio, percentile, population_std,
    require_finite_output, rms,
};
use crate::vector::{FeatureVector, FeatureVectorStatus, LinkFeatureValues};

/// Monotone feature-vector identity allocator.
static NEXT_FEATURE_VECTOR_ID: AtomicU64 = AtomicU64::new(1);

/// Extracts an immutable feature vector from one DSP window result.
pub fn extract_features(
    result: &DspWindowResult,
    profile: &FeatureProfile,
) -> Result<(FeatureVector, FeatureExtractionReport), FeatureError> {
    let started_instant = Instant::now();
    let started_at = now();
    profile.validate()?;
    profile.assert_dsp_compatible(&result.dsp_profile_id, result.dsp_profile_version)?;
    let schema = profile.schema()?;

    let context = FeatureError::context(
        Some(result.sensor_id),
        Some(result.window_id),
        None,
        None,
        Some(&profile.id),
    );

    let motion = result
        .motion_energy
        .signal
        .aggregate
        .as_deref()
        .ok_or_else(|| FeatureError::MissingMotionEnergy {
            context: context.clone(),
        })?;
    let spectrum =
        result
            .spectra
            .aggregate
            .as_ref()
            .ok_or_else(|| FeatureError::MissingSpectrum {
                context: context.clone(),
            })?;

    if result.motion_energy.signal.links.len() != result.spectra.links.len() {
        return Err(FeatureError::MismatchedLinkData {
            message: format!(
                "motion links {} vs spectrum links {}{context}",
                result.motion_energy.signal.links.len(),
                result.spectra.links.len()
            ),
        });
    }

    let mut warnings = result.warnings.clone();
    warnings.extend(result.spectra.warnings.iter().cloned());

    // Prefer DSP aggregate series for the canonical aggregate vector.
    let aggregate_values = compute_ordered_values(
        motion,
        &spectrum.frequencies_hz,
        &spectrum.power,
        result.sampling.effective_sample_rate_hz,
        result.sampling.timestamp_jitter,
        result.frame_count,
        result.antenna_links.len(),
        profile,
        &schema,
    )?;

    let mut link_features = Vec::with_capacity(result.motion_energy.signal.links.len());
    for motion_link in &result.motion_energy.signal.links {
        let spectrum_link = result
            .spectra
            .links
            .iter()
            .find(|spectrum| spectrum.link == motion_link.link)
            .ok_or_else(|| FeatureError::MismatchedLinkData {
                message: format!(
                    "missing spectrum for rx{}-tx{}{context}",
                    motion_link.link.rx, motion_link.link.tx
                ),
            })?;
        let values = compute_ordered_values(
            &motion_link.values,
            &spectrum_link.frequencies_hz,
            &spectrum_link.power,
            result.sampling.effective_sample_rate_hz,
            result.sampling.timestamp_jitter,
            result.frame_count,
            result.antenna_links.len(),
            profile,
            &schema,
        )?;
        link_features.push(LinkFeatureValues {
            link: motion_link.link,
            values,
        });
    }

    // When DSP aggregate is present we already used it. Aggregation policy still
    // documents fallback semantics for dominant-frequency selection across links.
    let _ = AggregationPolicy::PreferDspAggregateThenMean;

    let duration_ns = u64::try_from(started_instant.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let extracted_at = now();
    let feature_vector_id = NEXT_FEATURE_VECTOR_ID.fetch_add(1, Ordering::Relaxed);

    let vector = FeatureVector::try_new(
        feature_vector_id,
        result.sensor_id,
        result.window_id,
        result.first_sequence,
        result.last_sequence,
        result.first_capture_timestamp,
        result.last_capture_timestamp,
        schema.id.clone(),
        schema.version,
        profile.id.clone(),
        profile.version,
        result.dsp_profile_id.clone(),
        result.dsp_profile_version,
        result.backend_id.clone(),
        result.backend_version.clone(),
        result.backend_abi_version,
        result.calibration_profile_id.clone(),
        result.calibration_profile_version,
        aggregate_values,
        link_features,
        extracted_at,
        duration_ns,
        warnings.clone(),
        schema.length(),
    )?;

    let report = FeatureExtractionReport {
        window_id: result.window_id,
        profile_id: profile.id.clone(),
        profile_version: profile.version,
        schema_id: schema.id.clone(),
        schema_version: schema.version,
        features_requested: schema.length(),
        features_produced: vector.feature_count(),
        link_count: vector.link_count(),
        started_at,
        completed_at: extracted_at,
        processing_duration_ns: duration_ns,
        warnings,
        status: FeatureVectorStatus::Success,
    };

    Ok((vector, report))
}

#[allow(clippy::too_many_arguments)]
fn compute_ordered_values(
    motion: &[f64],
    frequencies_hz: &[f64],
    power: &[f64],
    sample_rate_hz: f64,
    timestamp_jitter: f64,
    frame_count: usize,
    link_count: usize,
    profile: &FeatureProfile,
    schema: &FeatureSchema,
) -> Result<Vec<f64>, FeatureError> {
    let spectral = extract_spectral_features(
        frequencies_hz,
        power,
        profile.flatness_epsilon,
        profile.frequency_band_policy,
    )?;

    let motion_mean = mean(motion)?;
    let motion_std = population_std(motion)?;
    let motion_rms = rms(motion)?;
    let motion_min = min(motion)?;
    let motion_max = max(motion)?;
    let motion_median = median(motion)?;
    let motion_p90 = percentile(motion, 90.0)?;
    let motion_p95 = percentile(motion, 95.0)?;
    let motion_range = motion_max - motion_min;
    let motion_mad = mean_absolute_delta(motion)?;
    let motion_peak_ratio = peak_to_mean_ratio(motion)?;

    let mut values = Vec::with_capacity(schema.length());
    for definition in &schema.features {
        if !profile.enabled_features.contains(&definition.id) {
            return Err(FeatureError::InvalidProfile {
                message: format!(
                    "schema feature `{}` is not enabled in the profile",
                    definition.id.as_str()
                ),
            });
        }
        let value = match definition.id {
            FeatureId::MotionEnergyMean => motion_mean,
            FeatureId::MotionEnergyStandardDeviation => motion_std,
            FeatureId::MotionEnergyRms => motion_rms,
            FeatureId::MotionEnergyMinimum => motion_min,
            FeatureId::MotionEnergyMaximum => motion_max,
            FeatureId::MotionEnergyMedian => motion_median,
            FeatureId::MotionEnergyP90 => motion_p90,
            FeatureId::MotionEnergyP95 => motion_p95,
            FeatureId::MotionEnergyRange => motion_range,
            FeatureId::MotionEnergyMeanAbsoluteDelta => motion_mad,
            FeatureId::MotionEnergyPeakToMeanRatio => motion_peak_ratio,
            FeatureId::TotalNonDcPower => spectral.total_non_dc_power,
            FeatureId::DominantNonDcFrequencyHz => spectral.dominant_non_dc_frequency_hz,
            FeatureId::DominantNonDcPower => spectral.dominant_non_dc_power,
            FeatureId::SpectralCentroidHz => spectral.spectral_centroid_hz,
            FeatureId::SpectralBandwidthHz => spectral.spectral_bandwidth_hz,
            FeatureId::SpectralEntropy => spectral.spectral_entropy,
            FeatureId::SpectralFlatness => spectral.spectral_flatness,
            FeatureId::LowFrequencyPowerRatio => spectral.low_frequency_power_ratio,
            FeatureId::MiddleFrequencyPowerRatio => spectral.middle_frequency_power_ratio,
            FeatureId::HighFrequencyPowerRatio => spectral.high_frequency_power_ratio,
            FeatureId::EffectiveSampleRateHz => sample_rate_hz,
            FeatureId::TimestampJitter => timestamp_jitter,
            FeatureId::FrameCount => frame_count as f64,
            FeatureId::LinkCount => link_count as f64,
        };
        values.push(require_finite_output(value, definition.id.as_str())?);
    }
    Ok(values)
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}
