//! Immutable feature vector produced from one DSP window.

use aeryon_calibration::AntennaLink;
use aeryon_domain::{SensorId, Timestamp};

use crate::errors::FeatureError;
use crate::schema::FeatureId;

/// Processing status for one feature extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureVectorStatus {
    /// Extraction completed successfully.
    Success,
    /// Extraction failed before a usable vector could be published.
    Failed,
}

impl FeatureVectorStatus {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

/// Per-link ordered feature values aligned with the schema.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkFeatureValues {
    /// Antenna link identity.
    pub link: AntennaLink,
    /// Ordered numerical values matching the feature schema.
    pub values: Vec<f64>,
}

/// Immutable feature vector for one DSP window.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureVector {
    /// Feature-vector identity (monotone within a process).
    pub feature_vector_id: u64,
    /// Sensor identity.
    pub sensor_id: SensorId,
    /// Source DSP window identity.
    pub window_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// First capture timestamp.
    pub first_capture_timestamp: Timestamp,
    /// Last capture timestamp.
    pub last_capture_timestamp: Timestamp,
    /// Feature schema identity.
    pub feature_schema_id: String,
    /// Feature schema version.
    pub feature_schema_version: u32,
    /// Feature profile identity.
    pub feature_profile_id: String,
    /// Feature profile version.
    pub feature_profile_version: u32,
    /// DSP profile identity.
    pub dsp_profile_id: String,
    /// DSP profile version.
    pub dsp_profile_version: u32,
    /// DSP backend identity (`rust` or `cpp`).
    pub dsp_backend_id: String,
    /// DSP backend implementation version.
    pub dsp_backend_version: String,
    /// Native ABI version when applicable.
    pub dsp_backend_abi_version: Option<u32>,
    /// Calibration profile identity.
    pub calibration_profile_id: String,
    /// Calibration profile version.
    pub calibration_profile_version: u32,
    /// Ordered aggregate numerical values (schema layout).
    values: Vec<f64>,
    /// Per-link feature collections (schema layout each).
    pub link_features: Vec<LinkFeatureValues>,
    /// Extraction completion timestamp.
    pub extracted_at: Timestamp,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
    /// Result status.
    pub status: FeatureVectorStatus,
}

impl FeatureVector {
    /// Constructs and validates an immutable feature vector.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        feature_vector_id: u64,
        sensor_id: SensorId,
        window_id: u64,
        first_sequence: u64,
        last_sequence: u64,
        first_capture_timestamp: Timestamp,
        last_capture_timestamp: Timestamp,
        feature_schema_id: String,
        feature_schema_version: u32,
        feature_profile_id: String,
        feature_profile_version: u32,
        dsp_profile_id: String,
        dsp_profile_version: u32,
        dsp_backend_id: String,
        dsp_backend_version: String,
        dsp_backend_abi_version: Option<u32>,
        calibration_profile_id: String,
        calibration_profile_version: u32,
        values: Vec<f64>,
        link_features: Vec<LinkFeatureValues>,
        extracted_at: Timestamp,
        processing_duration_ns: u64,
        warnings: Vec<String>,
        expected_length: usize,
    ) -> Result<Self, FeatureError> {
        if values.len() != expected_length {
            return Err(FeatureError::OutputValidation {
                message: format!(
                    "aggregate feature length {} does not match schema length {expected_length}",
                    values.len()
                ),
            });
        }
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(FeatureError::OutputValidation {
                    message: format!("aggregate feature index {index} is non-finite"),
                });
            }
        }
        for link in &link_features {
            if link.values.len() != expected_length {
                return Err(FeatureError::OutputValidation {
                    message: format!(
                        "link rx{}-tx{} feature length {} does not match schema length {expected_length}",
                        link.link.rx,
                        link.link.tx,
                        link.values.len()
                    ),
                });
            }
            for (index, value) in link.values.iter().enumerate() {
                if !value.is_finite() {
                    return Err(FeatureError::OutputValidation {
                        message: format!(
                            "link rx{}-tx{} feature index {index} is non-finite",
                            link.link.rx, link.link.tx
                        ),
                    });
                }
            }
        }
        if feature_schema_id.trim().is_empty()
            || feature_profile_id.trim().is_empty()
            || dsp_profile_id.trim().is_empty()
            || calibration_profile_id.trim().is_empty()
            || dsp_backend_id.trim().is_empty()
        {
            return Err(FeatureError::OutputValidation {
                message: "feature vector provenance fields must not be empty".to_owned(),
            });
        }

        Ok(Self {
            feature_vector_id,
            sensor_id,
            window_id,
            first_sequence,
            last_sequence,
            first_capture_timestamp,
            last_capture_timestamp,
            feature_schema_id,
            feature_schema_version,
            feature_profile_id,
            feature_profile_version,
            dsp_profile_id,
            dsp_profile_version,
            dsp_backend_id,
            dsp_backend_version,
            dsp_backend_abi_version,
            calibration_profile_id,
            calibration_profile_version,
            values,
            link_features,
            extracted_at,
            processing_duration_ns,
            warnings,
            status: FeatureVectorStatus::Success,
        })
    }

    /// Ordered aggregate values (immutable view).
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Number of aggregate features.
    pub fn feature_count(&self) -> usize {
        self.values.len()
    }

    /// Number of per-link collections.
    pub fn link_count(&self) -> usize {
        self.link_features.len()
    }

    /// Looks up an aggregate feature by schema index.
    pub fn value_at(&self, index: usize) -> Option<f64> {
        self.values.get(index).copied()
    }

    /// Looks up an aggregate feature by typed identifier using a precomputed index.
    pub fn value_by_id(
        &self,
        id: FeatureId,
        index_of: impl Fn(FeatureId) -> Option<usize>,
    ) -> Option<f64> {
        index_of(id).and_then(|index| self.value_at(index))
    }
}
