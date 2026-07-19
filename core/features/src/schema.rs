//! Versioned CSI channel feature schema.

use crate::errors::FeatureError;

/// Stable schema identity for baseline CSI channel descriptors.
pub const CSI_CHANNEL_FEATURES_V1_ID: &str = "csi-channel-features-v1";

/// Schema version for [`CSI_CHANNEL_FEATURES_V1_ID`].
pub const CSI_CHANNEL_FEATURES_V1_VERSION: u32 = 1;

/// Aggregation scope for a feature definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureAggregationScope {
    /// Value summarizes the whole window across links.
    Aggregate,
    /// Value is computed per RX–TX link.
    PerLink,
    /// Window-level temporal or quality metadata (not per-link).
    WindowMetadata,
}

impl FeatureAggregationScope {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Aggregate => "aggregate",
            Self::PerLink => "per_link",
            Self::WindowMetadata => "window_metadata",
        }
    }
}

/// Strongly typed feature identifiers for [`CSI_CHANNEL_FEATURES_V1_ID`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureId {
    /// Mean of the motion-energy series.
    MotionEnergyMean,
    /// Population standard deviation of motion energy.
    MotionEnergyStandardDeviation,
    /// Root-mean-square of motion energy.
    MotionEnergyRms,
    /// Minimum motion-energy sample.
    MotionEnergyMinimum,
    /// Maximum motion-energy sample.
    MotionEnergyMaximum,
    /// Median motion-energy sample.
    MotionEnergyMedian,
    /// 90th percentile of motion energy.
    MotionEnergyP90,
    /// 95th percentile of motion energy.
    MotionEnergyP95,
    /// Maximum − minimum of motion energy.
    MotionEnergyRange,
    /// Mean absolute consecutive delta of motion energy.
    MotionEnergyMeanAbsoluteDelta,
    /// Peak / mean ratio of motion energy.
    MotionEnergyPeakToMeanRatio,
    /// Sum of non-DC spectral power.
    TotalNonDcPower,
    /// Frequency of the strongest non-DC bin (Hz).
    DominantNonDcFrequencyHz,
    /// Power at the dominant non-DC frequency.
    DominantNonDcPower,
    /// Power-weighted spectral centroid (Hz).
    SpectralCentroidHz,
    /// Power-weighted spectral bandwidth (Hz).
    SpectralBandwidthHz,
    /// Normalized spectral entropy over non-DC bins.
    SpectralEntropy,
    /// Spectral flatness (geometric / arithmetic mean of power).
    SpectralFlatness,
    /// Fraction of non-DC power in the low third of the spectrum.
    LowFrequencyPowerRatio,
    /// Fraction of non-DC power in the middle third of the spectrum.
    MiddleFrequencyPowerRatio,
    /// Fraction of non-DC power in the high third of the spectrum.
    HighFrequencyPowerRatio,
    /// Capture-time effective sample rate (Hz).
    EffectiveSampleRateHz,
    /// Capture-time timestamp jitter metric.
    TimestampJitter,
    /// Number of frames in the source window.
    FrameCount,
    /// Number of antenna links present.
    LinkCount,
}

impl FeatureId {
    /// Stable snake_case wire identity.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MotionEnergyMean => "motion_energy_mean",
            Self::MotionEnergyStandardDeviation => "motion_energy_standard_deviation",
            Self::MotionEnergyRms => "motion_energy_rms",
            Self::MotionEnergyMinimum => "motion_energy_minimum",
            Self::MotionEnergyMaximum => "motion_energy_maximum",
            Self::MotionEnergyMedian => "motion_energy_median",
            Self::MotionEnergyP90 => "motion_energy_p90",
            Self::MotionEnergyP95 => "motion_energy_p95",
            Self::MotionEnergyRange => "motion_energy_range",
            Self::MotionEnergyMeanAbsoluteDelta => "motion_energy_mean_absolute_delta",
            Self::MotionEnergyPeakToMeanRatio => "motion_energy_peak_to_mean_ratio",
            Self::TotalNonDcPower => "total_non_dc_power",
            Self::DominantNonDcFrequencyHz => "dominant_non_dc_frequency_hz",
            Self::DominantNonDcPower => "dominant_non_dc_power",
            Self::SpectralCentroidHz => "spectral_centroid_hz",
            Self::SpectralBandwidthHz => "spectral_bandwidth_hz",
            Self::SpectralEntropy => "spectral_entropy",
            Self::SpectralFlatness => "spectral_flatness",
            Self::LowFrequencyPowerRatio => "low_frequency_power_ratio",
            Self::MiddleFrequencyPowerRatio => "middle_frequency_power_ratio",
            Self::HighFrequencyPowerRatio => "high_frequency_power_ratio",
            Self::EffectiveSampleRateHz => "effective_sample_rate_hz",
            Self::TimestampJitter => "timestamp_jitter",
            Self::FrameCount => "frame_count",
            Self::LinkCount => "link_count",
        }
    }

    /// Parses a snake_case feature identity.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|feature| feature.as_str() == name)
    }
}

impl FeatureId {
    /// Canonical ordered feature list for the v1 schema.
    pub const ALL: &[FeatureId] = &[
        Self::MotionEnergyMean,
        Self::MotionEnergyStandardDeviation,
        Self::MotionEnergyRms,
        Self::MotionEnergyMinimum,
        Self::MotionEnergyMaximum,
        Self::MotionEnergyMedian,
        Self::MotionEnergyP90,
        Self::MotionEnergyP95,
        Self::MotionEnergyRange,
        Self::MotionEnergyMeanAbsoluteDelta,
        Self::MotionEnergyPeakToMeanRatio,
        Self::TotalNonDcPower,
        Self::DominantNonDcFrequencyHz,
        Self::DominantNonDcPower,
        Self::SpectralCentroidHz,
        Self::SpectralBandwidthHz,
        Self::SpectralEntropy,
        Self::SpectralFlatness,
        Self::LowFrequencyPowerRatio,
        Self::MiddleFrequencyPowerRatio,
        Self::HighFrequencyPowerRatio,
        Self::EffectiveSampleRateHz,
        Self::TimestampJitter,
        Self::FrameCount,
        Self::LinkCount,
    ];
}

/// One ordered feature definition in a versioned schema.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureDefinition {
    /// Typed feature identity.
    pub id: FeatureId,
    /// Unit or semantic unit label.
    pub unit: &'static str,
    /// Concise semantic description (not a physical activity claim).
    pub description: &'static str,
    /// Aggregation scope.
    pub scope: FeatureAggregationScope,
    /// Expected numerical representation.
    pub data_type: &'static str,
    /// Whether DSP provenance is required to interpret the value.
    pub requires_dsp_provenance: bool,
}

/// Versioned feature schema describing ordered numerical descriptors.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureSchema {
    /// Schema identity.
    pub id: String,
    /// Schema version.
    pub version: u32,
    /// Human-readable description.
    pub description: String,
    /// Ordered feature definitions (canonical vector layout).
    pub features: Vec<FeatureDefinition>,
}

impl FeatureSchema {
    /// Number of numerical values in one aggregate vector.
    pub fn length(&self) -> usize {
        self.features.len()
    }

    /// Ordered feature names.
    pub fn ordered_names(&self) -> Vec<&'static str> {
        self.features
            .iter()
            .map(|feature| feature.id.as_str())
            .collect()
    }

    /// Looks up a feature definition by typed identifier.
    pub fn definition(&self, id: FeatureId) -> Option<&FeatureDefinition> {
        self.features.iter().find(|feature| feature.id == id)
    }

    /// Index of a typed feature in the ordered vector.
    pub fn index_of(&self, id: FeatureId) -> Option<usize> {
        self.features.iter().position(|feature| feature.id == id)
    }

    /// Validates schema identity and uniqueness.
    pub fn validate(&self) -> Result<(), FeatureError> {
        if self.id.trim().is_empty() {
            return Err(FeatureError::SchemaMismatch {
                message: "feature schema id must not be empty".to_owned(),
            });
        }
        if self.version == 0 {
            return Err(FeatureError::SchemaMismatch {
                message: "feature schema version must be >= 1".to_owned(),
            });
        }
        if self.features.is_empty() {
            return Err(FeatureError::SchemaMismatch {
                message: "feature schema must define at least one feature".to_owned(),
            });
        }
        let mut seen = Vec::with_capacity(self.features.len());
        for feature in &self.features {
            if seen.contains(&feature.id) {
                return Err(FeatureError::SchemaMismatch {
                    message: format!("duplicate feature `{}` in schema", feature.id.as_str()),
                });
            }
            seen.push(feature.id);
        }
        Ok(())
    }

    /// Validates compatibility with another schema identity/version.
    pub fn assert_compatible(&self, id: &str, version: u32) -> Result<(), FeatureError> {
        if self.id != id || self.version != version {
            return Err(FeatureError::SchemaMismatch {
                message: format!(
                    "expected schema `{}/{}`, got `{id}/{version}`",
                    self.id, self.version
                ),
            });
        }
        Ok(())
    }
}

/// Returns the built-in CSI channel feature schema v1.
pub fn csi_channel_features_v1() -> FeatureSchema {
    let features = FeatureId::ALL.iter().copied().map(definition_for).collect();
    FeatureSchema {
        id: CSI_CHANNEL_FEATURES_V1_ID.to_owned(),
        version: CSI_CHANNEL_FEATURES_V1_VERSION,
        description: "Deterministic CSI channel-change descriptors derived from DSP \
             motion-energy and non-DC spectra. Not human presence, occupancy, or activity labels."
            .to_owned(),
        features,
    }
}

fn definition_for(id: FeatureId) -> FeatureDefinition {
    match id {
        FeatureId::MotionEnergyMean => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Mean of the motion-energy channel-change proxy series.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyStandardDeviation => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Population standard deviation of the motion-energy series.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyRms => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Root-mean-square of the motion-energy series.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyMinimum => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Minimum motion-energy sample in the window.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyMaximum => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Maximum motion-energy sample in the window.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyMedian => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Median motion-energy sample in the window.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyP90 => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Linear-interpolated 90th percentile of motion energy.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyP95 => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Linear-interpolated 95th percentile of motion energy.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyRange => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Motion-energy maximum minus minimum.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyMeanAbsoluteDelta => FeatureDefinition {
            id,
            unit: "normalized_complex_difference",
            description: "Mean absolute consecutive difference of motion energy.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MotionEnergyPeakToMeanRatio => FeatureDefinition {
            id,
            unit: "ratio",
            description: "Motion-energy maximum divided by mean (zero mean → 0).",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::TotalNonDcPower => FeatureDefinition {
            id,
            unit: "normalized_power",
            description: "Sum of one-sided periodogram power excluding DC.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::DominantNonDcFrequencyHz => FeatureDefinition {
            id,
            unit: "hertz",
            description: "Frequency of the strongest non-DC spectral bin.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::DominantNonDcPower => FeatureDefinition {
            id,
            unit: "normalized_power",
            description: "Power at the dominant non-DC frequency bin.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::SpectralCentroidHz => FeatureDefinition {
            id,
            unit: "hertz",
            description: "Power-weighted centroid of non-DC spectral bins.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::SpectralBandwidthHz => FeatureDefinition {
            id,
            unit: "hertz",
            description: "Power-weighted second-moment bandwidth around the centroid.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::SpectralEntropy => FeatureDefinition {
            id,
            unit: "normalized_entropy",
            description: "Normalized spectral entropy over non-DC power (tonal vs broadband).",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::SpectralFlatness => FeatureDefinition {
            id,
            unit: "ratio",
            description: "Spectral flatness of non-DC power (geometric/arithmetic mean).",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::LowFrequencyPowerRatio => FeatureDefinition {
            id,
            unit: "ratio",
            description: "Non-DC power fraction in the first third of available frequencies.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::MiddleFrequencyPowerRatio => FeatureDefinition {
            id,
            unit: "ratio",
            description: "Non-DC power fraction in the middle third of available frequencies.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::HighFrequencyPowerRatio => FeatureDefinition {
            id,
            unit: "ratio",
            description: "Non-DC power fraction in the final third of available frequencies.",
            scope: FeatureAggregationScope::Aggregate,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::EffectiveSampleRateHz => FeatureDefinition {
            id,
            unit: "hertz",
            description: "Capture-time effective sample rate (1 / median interval).",
            scope: FeatureAggregationScope::WindowMetadata,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::TimestampJitter => FeatureDefinition {
            id,
            unit: "relative",
            description: "Capture-time relative timestamp jitter metric from DSP sampling analysis.",
            scope: FeatureAggregationScope::WindowMetadata,
            data_type: "f64",
            requires_dsp_provenance: true,
        },
        FeatureId::FrameCount => FeatureDefinition {
            id,
            unit: "count",
            description: "Number of CSI frames in the source DSP window.",
            scope: FeatureAggregationScope::WindowMetadata,
            data_type: "f64",
            requires_dsp_provenance: false,
        },
        FeatureId::LinkCount => FeatureDefinition {
            id,
            unit: "count",
            description: "Number of RX–TX antenna links contributing features.",
            scope: FeatureAggregationScope::WindowMetadata,
            data_type: "f64",
            requires_dsp_provenance: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_order_and_length_are_stable() {
        let schema = csi_channel_features_v1();
        schema.validate().expect("valid");
        assert_eq!(schema.id, CSI_CHANNEL_FEATURES_V1_ID);
        assert_eq!(schema.version, CSI_CHANNEL_FEATURES_V1_VERSION);
        assert_eq!(schema.length(), FeatureId::ALL.len());
        assert_eq!(schema.ordered_names()[0], "motion_energy_mean");
        assert_eq!(schema.ordered_names().last().copied(), Some("link_count"));
        assert_eq!(schema.index_of(FeatureId::MotionEnergyRms), Some(2));
    }

    #[test]
    fn typed_lookup_round_trips() {
        for id in FeatureId::ALL {
            assert_eq!(FeatureId::parse(id.as_str()), Some(*id));
        }
        assert_eq!(FeatureId::parse("not_a_feature"), None);
    }
}
