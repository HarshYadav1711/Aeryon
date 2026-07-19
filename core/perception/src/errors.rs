//! Typed perception / observation errors.

use thiserror::Error;

/// Errors produced while creating channel-change observations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PerceptionError {
    /// Feature schema is incompatible with the observation profile.
    #[error("incompatible feature schema: {message}")]
    IncompatibleFeatureSchema {
        /// Operator-safe detail.
        message: String,
    },
    /// Required features are unavailable.
    #[error("missing features: {message}")]
    MissingFeatures {
        /// Operator-safe detail.
        message: String,
    },
    /// Observation profile failed validation.
    #[error("invalid perception profile: {message}")]
    InvalidProfile {
        /// Operator-safe detail.
        message: String,
    },
    /// Non-finite score or supporting values.
    #[error("non-finite observation input: {message}")]
    NonFinite {
        /// Operator-safe detail.
        message: String,
    },
    /// Output validation failure.
    #[error("observation output validation failed: {message}")]
    OutputValidation {
        /// Operator-safe detail.
        message: String,
    },
    /// Service-level failure.
    #[error("perception service failure: {message}")]
    ServiceFailure {
        /// Operator-safe detail.
        message: String,
    },
}
