//! Typed errors for CSI frame validation and fixture parsing.

use core::fmt;
use std::io;
use std::num::ParseIntError;

use thiserror::Error;

/// Errors produced when constructing or indexing a [`crate::CsiFrame`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiFrameError {
    /// Receive antenna count must be greater than zero.
    ZeroReceiveAntennas,
    /// Transmit antenna count must be greater than zero.
    ZeroTransmitAntennas,
    /// At least one subcarrier index is required.
    EmptySubcarriers,
    /// Subcarrier indices must be unique.
    DuplicateSubcarrierIndex {
        /// Duplicate index value.
        index: i16,
    },
    /// Subcarrier indices must already be in strict ascending order.
    SubcarriersNotAscending {
        /// Position of the ordering violation.
        position: usize,
    },
    /// Sample vector length does not match antenna × subcarrier dimensions.
    SampleCountMismatch {
        /// Expected number of complex samples.
        expected: usize,
        /// Actual number of complex samples.
        actual: usize,
    },
    /// A complex sample was non-finite.
    NonFiniteSample {
        /// Sample index in contiguous storage.
        index: usize,
    },
    /// Center frequency must be finite and above the platform minimum when set.
    InvalidCenterFrequency,
    /// Bandwidth must be finite and positive when set.
    InvalidBandwidth,
    /// Optional RSSI must be finite when set.
    InvalidRssi,
    /// Optional noise floor must be finite when set.
    InvalidNoiseFloor,
    /// Optional AGC value must be finite when set.
    InvalidAgc,
    /// Indexing request was outside validated frame dimensions.
    OutOfBounds {
        /// Human-readable component that was out of bounds.
        component: &'static str,
        /// Requested index.
        requested: usize,
        /// Exclusive upper bound.
        bound: usize,
    },
}

impl fmt::Display for CsiFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroReceiveAntennas => f.write_str("receive_antennas must be > 0"),
            Self::ZeroTransmitAntennas => f.write_str("transmit_antennas must be > 0"),
            Self::EmptySubcarriers => f.write_str("subcarrier_indices must not be empty"),
            Self::DuplicateSubcarrierIndex { index } => {
                write!(f, "duplicate subcarrier index {index}")
            }
            Self::SubcarriersNotAscending { position } => {
                write!(
                    f,
                    "subcarrier_indices must be strictly ascending at position {position}"
                )
            }
            Self::SampleCountMismatch { expected, actual } => {
                write!(
                    f,
                    "sample count mismatch: expected {expected}, got {actual}"
                )
            }
            Self::NonFiniteSample { index } => {
                write!(f, "non-finite complex sample at index {index}")
            }
            Self::InvalidCenterFrequency => {
                f.write_str("center_frequency_hz must be finite and within valid range")
            }
            Self::InvalidBandwidth => f.write_str("bandwidth_hz must be finite and > 0"),
            Self::InvalidRssi => f.write_str("rssi_dbm must be finite when present"),
            Self::InvalidNoiseFloor => f.write_str("noise_floor_dbm must be finite when present"),
            Self::InvalidAgc => f.write_str("agc must be finite when present"),
            Self::OutOfBounds {
                component,
                requested,
                bound,
            } => write!(
                f,
                "{component} index {requested} is out of bounds (bound {bound})"
            ),
        }
    }
}

impl std::error::Error for CsiFrameError {}

/// Errors produced while reading an Aeryon CSI Fixture Format stream.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// Underlying I/O failure.
    #[error("fixture I/O error at line {line}: {source}")]
    Io {
        /// 1-based line number being processed when the error occurred.
        line: usize,
        /// Source I/O error.
        #[source]
        source: io::Error,
    },
    /// JSON parse failure for a record.
    #[error("fixture JSON error at line {line}: {source}")]
    Json {
        /// 1-based line number of the malformed record.
        line: usize,
        /// Source serde JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// The fixture stream ended before a header was observed.
    #[error("fixture is missing a header record")]
    MissingHeader,
    /// First record was not a header.
    #[error("fixture first record must be a header (line {line})")]
    ExpectedHeader {
        /// 1-based line number of the unexpected record.
        line: usize,
    },
    /// Schema identifier is not supported.
    #[error("unsupported fixture schema `{schema}` at line {line}")]
    UnsupportedSchema {
        /// 1-based line number.
        line: usize,
        /// Observed schema string.
        schema: String,
    },
    /// Schema version is not supported.
    #[error("unsupported fixture version {version} at line {line}")]
    UnsupportedVersion {
        /// 1-based line number.
        line: usize,
        /// Observed version.
        version: u32,
    },
    /// Sample layout is unknown or unsupported.
    #[error("unsupported sample layout `{layout}` at line {line}")]
    UnsupportedLayout {
        /// 1-based line number.
        line: usize,
        /// Observed layout string.
        layout: String,
    },
    /// Header sensor identifier could not be parsed.
    #[error("invalid sensor_id in fixture header at line {line}: {source}")]
    InvalidSensorId {
        /// 1-based line number.
        line: usize,
        /// Parse failure.
        #[source]
        source: ParseIntError,
    },
    /// A frame record failed structural validation before CSI conversion.
    #[error("malformed fixture frame at line {line}: {reason}")]
    MalformedFrame {
        /// 1-based line number.
        line: usize,
        /// Short reason.
        reason: String,
    },
    /// Canonical CSI frame validation failed for a fixture record.
    #[error("invalid CSI frame at line {line}: {source}")]
    InvalidFrame {
        /// 1-based line number.
        line: usize,
        /// Frame validation error.
        #[source]
        source: CsiFrameError,
    },
    /// Frame sequences must be strictly increasing across the fixture.
    #[error("non-monotonic sequence at line {line}: previous={previous}, current={current}")]
    NonMonotonicSequence {
        /// 1-based line number.
        line: usize,
        /// Previous accepted sequence.
        previous: u64,
        /// Current sequence value.
        current: u64,
    },
    /// Encountered a record type that is not permitted after the header.
    #[error("unexpected record_type `{record_type}` at line {line}")]
    UnexpectedRecordType {
        /// 1-based line number.
        line: usize,
        /// Observed record type.
        record_type: String,
    },
}

impl FixtureError {
    /// Returns the 1-based line associated with this error, when available.
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::Io { line, .. }
            | Self::Json { line, .. }
            | Self::ExpectedHeader { line }
            | Self::UnsupportedSchema { line, .. }
            | Self::UnsupportedVersion { line, .. }
            | Self::UnsupportedLayout { line, .. }
            | Self::InvalidSensorId { line, .. }
            | Self::MalformedFrame { line, .. }
            | Self::InvalidFrame { line, .. }
            | Self::NonMonotonicSequence { line, .. }
            | Self::UnexpectedRecordType { line, .. } => Some(*line),
            Self::MissingHeader => None,
        }
    }
}
