//! Deterministic channel-change observation from feature vectors.
//!
//! # Score formula (`channel-change-v1`)
//!
//! ```text
//! rms_n = clamp(motion_energy_rms / motion_energy_rms_scale, 0, 1)
//! p95_n = clamp(motion_energy_p95 / motion_energy_p95_scale, 0, 1)
//! score = 0.5 * rms_n + 0.5 * p95_n
//! ```
//!
//! The score is a bounded heuristic intensity of measured channel variation.
//! It is **not** a probability, ML confidence, or presence likelihood.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeryon_domain::Timestamp;
use aeryon_features::{FeatureId, FeatureVector, csi_channel_features_v1};

use crate::errors::PerceptionError;
use crate::evidence::{FeatureEvidence, ObservationEvidence};
use crate::observation::{
    ChannelChangeObservation, ChannelChangeState, ObservationUncertainty, RELIABILITY_PROVENANCE,
};
use crate::profile::ChannelChangeProfile;

static NEXT_OBSERVATION_ID: AtomicU64 = AtomicU64::new(1);

/// Creates a [`ChannelChangeObservation`] from one feature vector.
pub fn observe_channel_change(
    vector: &FeatureVector,
    profile: &ChannelChangeProfile,
) -> Result<ChannelChangeObservation, PerceptionError> {
    profile.validate()?;
    if vector.feature_schema_id != profile.feature_schema_id
        || vector.feature_schema_version != profile.feature_schema_version
    {
        return Err(PerceptionError::IncompatibleFeatureSchema {
            message: format!(
                "feature vector schema `{}/{}` incompatible with observation profile `{}/{}`",
                vector.feature_schema_id,
                vector.feature_schema_version,
                profile.id,
                profile.version
            ),
        });
    }

    let schema = csi_channel_features_v1();
    let lookup = |id: FeatureId| -> Result<f64, PerceptionError> {
        let index = schema
            .index_of(id)
            .ok_or_else(|| PerceptionError::MissingFeatures {
                message: format!("feature `{}` missing from schema", id.as_str()),
            })?;
        vector
            .value_at(index)
            .ok_or_else(|| PerceptionError::MissingFeatures {
                message: format!("feature `{}` missing from vector", id.as_str()),
            })
    };

    let rms = lookup(FeatureId::MotionEnergyRms)?;
    let p95 = lookup(FeatureId::MotionEnergyP95)?;
    let jitter = lookup(FeatureId::TimestampJitter)?;
    let frame_count = lookup(FeatureId::FrameCount)?;
    let link_count = lookup(FeatureId::LinkCount)?;

    for (name, value) in [
        ("motion_energy_rms", rms),
        ("motion_energy_p95", p95),
        ("timestamp_jitter", jitter),
    ] {
        if !value.is_finite() {
            return Err(PerceptionError::NonFinite {
                message: format!("{name} is non-finite"),
            });
        }
    }

    let mut warnings = vector.warnings.clone();
    let mut indeterminate_reasons = Vec::new();

    if jitter > profile.maximum_timestamp_jitter {
        indeterminate_reasons.push(format!(
            "timestamp_jitter {jitter} exceeds maximum {}",
            profile.maximum_timestamp_jitter
        ));
    }
    if !vector.warnings.is_empty() {
        indeterminate_reasons.push(format!(
            "feature extraction reported {} warning(s)",
            vector.warnings.len()
        ));
    }

    let rms_n = clamp01(rms / profile.motion_energy_rms_scale);
    let p95_n = clamp01(p95 / profile.motion_energy_p95_scale);
    let score = 0.5 * rms_n + 0.5 * p95_n;
    if !score.is_finite() {
        return Err(PerceptionError::NonFinite {
            message: "activity score is non-finite".to_owned(),
        });
    }

    let margin = threshold_margin(
        score,
        profile.stable_threshold,
        profile.high_change_threshold,
    );
    if margin < profile.minimum_margin {
        indeterminate_reasons.push(format!(
            "threshold margin {margin} below minimum {}",
            profile.minimum_margin
        ));
    }

    let state = if !indeterminate_reasons.is_empty() {
        warnings.extend(indeterminate_reasons.iter().cloned());
        ChannelChangeState::Indeterminate
    } else if score < profile.stable_threshold {
        ChannelChangeState::Stable
    } else if score < profile.high_change_threshold {
        ChannelChangeState::Changing
    } else {
        ChannelChangeState::HighlyChanging
    };

    let evidence = ObservationEvidence {
        features: vec![
            FeatureEvidence {
                feature_id: FeatureId::MotionEnergyRms,
                value: rms,
                normalized_contribution: Some(0.5 * rms_n),
            },
            FeatureEvidence {
                feature_id: FeatureId::MotionEnergyP95,
                value: p95,
                normalized_contribution: Some(0.5 * p95_n),
            },
        ],
        activity_score: score,
        stable_threshold: profile.stable_threshold,
        high_change_threshold: profile.high_change_threshold,
        threshold_margin: margin,
        data_quality_warnings: vector.warnings.clone(),
    };

    let reliability =
        heuristic_reliability(margin, &warnings, frame_count, link_count, jitter, profile);
    let uncertainty = ObservationUncertainty {
        threshold_margin: margin,
        normalized_threshold_margin: normalize_margin(margin, profile),
        timestamp_jitter: jitter,
        warning_count: warnings.len() as u32,
        supporting_frame_count: frame_count.max(0.0) as u32,
        valid_antenna_links: link_count.max(0.0) as u32,
        reliability_score: reliability,
        reliability_provenance: RELIABILITY_PROVENANCE.to_owned(),
    };

    let observation = ChannelChangeObservation {
        observation_id: NEXT_OBSERVATION_ID.fetch_add(1, Ordering::Relaxed),
        sensor_id: vector.sensor_id,
        feature_vector_id: vector.feature_vector_id,
        window_id: vector.window_id,
        first_sequence: vector.first_sequence,
        last_sequence: vector.last_sequence,
        first_capture_timestamp: vector.first_capture_timestamp,
        last_capture_timestamp: vector.last_capture_timestamp,
        state,
        activity_score: score,
        score_semantics: "heuristic_channel_change_intensity_v1".to_owned(),
        threshold_profile_id: profile.id.clone(),
        threshold_profile_version: profile.version,
        evidence,
        uncertainty,
        feature_schema_id: vector.feature_schema_id.clone(),
        feature_schema_version: vector.feature_schema_version,
        feature_profile_id: vector.feature_profile_id.clone(),
        feature_profile_version: vector.feature_profile_version,
        dsp_profile_id: vector.dsp_profile_id.clone(),
        dsp_profile_version: vector.dsp_profile_version,
        dsp_backend_id: vector.dsp_backend_id.clone(),
        dsp_backend_version: vector.dsp_backend_version.clone(),
        created_at: now(),
        warnings,
    };

    if !observation.activity_score.is_finite() || !(0.0..=1.0).contains(&observation.activity_score)
    {
        return Err(PerceptionError::OutputValidation {
            message: "activity score must be finite and within [0, 1]".to_owned(),
        });
    }

    Ok(observation)
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn threshold_margin(score: f64, stable: f64, high: f64) -> f64 {
    if score < stable {
        stable - score
    } else if score < high {
        (score - stable).min(high - score)
    } else {
        score - high
    }
}

fn normalize_margin(margin: f64, profile: &ChannelChangeProfile) -> f64 {
    let span = (profile.high_change_threshold - profile.stable_threshold).max(f64::EPSILON);
    clamp01(margin / span)
}

fn heuristic_reliability(
    margin: f64,
    warnings: &[String],
    frame_count: f64,
    link_count: f64,
    jitter: f64,
    profile: &ChannelChangeProfile,
) -> f64 {
    // Conservative heuristic reliability — not a probability.
    let margin_term = normalize_margin(margin, profile);
    let warning_penalty = (warnings.len() as f64 * 0.1).min(0.5);
    let jitter_penalty = if profile.maximum_timestamp_jitter > 0.0 {
        clamp01(jitter / profile.maximum_timestamp_jitter) * 0.25
    } else {
        0.0
    };
    let support = clamp01((frame_count / 16.0).min(1.0) * 0.5 + (link_count / 4.0).min(1.0) * 0.5);
    clamp01(0.35 + 0.45 * margin_term + 0.20 * support - warning_penalty - jitter_penalty).min(0.85)
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
    use aeryon_domain::{SensorId, Timestamp};
    use aeryon_features::{FeatureVector, baseline_features_v1};

    fn vector_with(rms: f64, p95: f64, jitter: f64, warnings: Vec<String>) -> FeatureVector {
        let profile = baseline_features_v1();
        let schema = profile.schema().unwrap();
        let mut values = vec![0.0; schema.length()];
        values[schema.index_of(FeatureId::MotionEnergyRms).unwrap()] = rms;
        values[schema.index_of(FeatureId::MotionEnergyP95).unwrap()] = p95;
        values[schema.index_of(FeatureId::TimestampJitter).unwrap()] = jitter;
        values[schema.index_of(FeatureId::FrameCount).unwrap()] = 16.0;
        values[schema.index_of(FeatureId::LinkCount).unwrap()] = 2.0;
        let expected_length = schema.length();
        FeatureVector::try_new(
            1,
            SensorId::new(2),
            9,
            0,
            15,
            Timestamp::from_nanos(0),
            Timestamp::from_nanos(1_500_000_000),
            schema.id.clone(),
            schema.version,
            profile.id,
            profile.version,
            "baseline-dsp-v1".into(),
            1,
            "rust".into(),
            "1".into(),
            None,
            "baseline-csi-v1".into(),
            1,
            values,
            Vec::new(),
            Timestamp::from_nanos(2),
            100,
            warnings,
            expected_length,
        )
        .unwrap()
    }

    #[test]
    fn states_and_boundaries() {
        let profile = crate::profile::ChannelChangeV1Config::default().to_profile();
        let stable =
            observe_channel_change(&vector_with(0.01, 0.01, 0.0, Vec::new()), &profile).unwrap();
        assert_eq!(stable.state, ChannelChangeState::Stable);
        assert!((0.0..=1.0).contains(&stable.activity_score));

        let at_stable = observe_channel_change(
            &vector_with(
                profile.stable_threshold * profile.motion_energy_rms_scale,
                profile.stable_threshold * profile.motion_energy_p95_scale,
                0.0,
                Vec::new(),
            ),
            &profile,
        )
        .unwrap();
        // score == stable_threshold → Changing (stable is exclusive upper bound for Stable)
        assert_eq!(at_stable.state, ChannelChangeState::Changing);

        let high = observe_channel_change(
            &vector_with(
                profile.high_change_threshold * profile.motion_energy_rms_scale,
                profile.high_change_threshold * profile.motion_energy_p95_scale,
                0.0,
                Vec::new(),
            ),
            &profile,
        )
        .unwrap();
        assert_eq!(high.state, ChannelChangeState::HighlyChanging);
    }

    #[test]
    fn indeterminate_on_jitter_and_warnings() {
        let profile = crate::profile::ChannelChangeV1Config::default().to_profile();
        let obs = observe_channel_change(
            &vector_with(0.1, 0.1, profile.maximum_timestamp_jitter + 0.1, Vec::new()),
            &profile,
        )
        .unwrap();
        assert_eq!(obs.state, ChannelChangeState::Indeterminate);

        let warned = observe_channel_change(
            &vector_with(0.1, 0.1, 0.0, vec!["quality".into()]),
            &profile,
        )
        .unwrap();
        assert_eq!(warned.state, ChannelChangeState::Indeterminate);
    }

    #[test]
    fn score_is_monotonic() {
        let profile = crate::profile::ChannelChangeV1Config::default().to_profile();
        let low = observe_channel_change(&vector_with(0.05, 0.05, 0.0, Vec::new()), &profile)
            .unwrap()
            .activity_score;
        let high = observe_channel_change(&vector_with(0.4, 0.5, 0.0, Vec::new()), &profile)
            .unwrap()
            .activity_score;
        assert!(high > low);
    }

    #[test]
    fn no_probability_semantics_in_provenance() {
        let profile = crate::profile::ChannelChangeV1Config::default().to_profile();
        let obs =
            observe_channel_change(&vector_with(0.1, 0.1, 0.0, Vec::new()), &profile).unwrap();
        assert!(obs.score_semantics.contains("heuristic"));
        assert!(!obs.score_semantics.contains("probability"));
        assert_eq!(
            obs.uncertainty.reliability_provenance,
            RELIABILITY_PROVENANCE
        );
    }
}
