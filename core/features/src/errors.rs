//! Typed feature-extraction errors.

use aeryon_domain::SensorId;
use thiserror::Error;

/// Errors produced while extracting or validating feature vectors.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FeatureError {
    /// Selected DSP profile is incompatible with the feature profile.
    #[error("incompatible DSP profile: {message}")]
    IncompatibleDspProfile {
        /// Operator-safe detail.
        message: String,
    },
    /// Motion-energy series required by the extractor is missing.
    #[error("missing motion-energy series{context}")]
    MissingMotionEnergy {
        /// Optional contextual suffix (sensor/window/link).
        context: String,
    },
    /// Power spectrum required by the extractor is missing.
    #[error("missing spectrum{context}")]
    MissingSpectrum {
        /// Optional contextual suffix.
        context: String,
    },
    /// Per-link geometry or counts do not align.
    #[error("mismatched link data: {message}")]
    MismatchedLinkData {
        /// Operator-safe detail.
        message: String,
    },
    /// Empty motion-energy or spectral input.
    #[error("empty signal{context}")]
    EmptySignal {
        /// Optional contextual suffix.
        context: String,
    },
    /// Non-finite intermediate or input values.
    #[error("non-finite input{context}: {message}")]
    NonFiniteInput {
        /// Optional contextual suffix.
        context: String,
        /// Operator-safe detail.
        message: String,
    },
    /// Negative or otherwise invalid spectral power.
    #[error("invalid power values{context}: {message}")]
    InvalidPower {
        /// Optional contextual suffix.
        context: String,
        /// Operator-safe detail.
        message: String,
    },
    /// Zero total power where the selected calculation cannot proceed.
    #[error("zero total power{context}")]
    ZeroTotalPower {
        /// Optional contextual suffix.
        context: String,
    },
    /// Feature profile failed validation.
    #[error("invalid feature profile: {message}")]
    InvalidProfile {
        /// Operator-safe detail.
        message: String,
    },
    /// Feature schema mismatch or validation failure.
    #[error("feature schema mismatch: {message}")]
    SchemaMismatch {
        /// Operator-safe detail.
        message: String,
    },
    /// Produced feature vector failed validation.
    #[error("feature output validation failed: {message}")]
    OutputValidation {
        /// Operator-safe detail.
        message: String,
    },
    /// Feature service / configuration failure.
    #[error("feature service failure: {message}")]
    ServiceFailure {
        /// Operator-safe detail.
        message: String,
    },
}

impl FeatureError {
    /// Builds a contextual suffix for error messages.
    pub fn context(
        sensor_id: Option<SensorId>,
        window_id: Option<u64>,
        link: Option<(u16, u16)>,
        feature: Option<&str>,
        profile: Option<&str>,
    ) -> String {
        let mut parts = Vec::new();
        if let Some(sensor) = sensor_id {
            parts.push(format!("sensor={}", sensor.value()));
        }
        if let Some(window) = window_id {
            parts.push(format!("window={window}"));
        }
        if let Some((rx, tx)) = link {
            parts.push(format!("link=rx{rx}-tx{tx}"));
        }
        if let Some(feature) = feature {
            parts.push(format!("feature={feature}"));
        }
        if let Some(profile) = profile {
            parts.push(format!("profile={profile}"));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", parts.join(", "))
        }
    }
}
