//! Canonical WiFi CSI types and the Aeryon CSI Fixture Format v1.
//!
//! This crate is modality-specific. Sensor-agnostic identifiers and timestamps
//! come from [`aeryon_domain`]; WiFi CSI concepts stay here so they can be shared
//! by acquisition, replay, recording, calibration, DSP, and tests.
//!
//! # Sample memory order
//!
//! Contiguous sample storage uses canonical order:
//!
//! `[receive_antenna][transmit_antenna][subcarrier]`
//!
//! Subcarriers are contiguous for each transmit–receive antenna link.
//! Index formula:
//!
//! ```text
//! index = ((rx * n_tx) + tx) * n_subcarriers + subcarrier_position
//! ```

#![deny(missing_docs)]

pub mod error;
pub mod fixture;
pub mod frame;

pub use error::{CsiFrameError, FixtureError};
pub use fixture::{
    CanonicalSampleLayout, FIXTURE_SCHEMA, FIXTURE_VERSION, FixtureFrameRecord, FixtureHeader,
    FixtureReader, SAMPLE_LAYOUT_RX_TX_SUBCARRIER,
};
pub use frame::{
    CENTER_FREQUENCY_HZ_MIN, CHANNEL_BANDWIDTH_HZ_MIN, ComplexSample, CsiFrame, CsiRadioMetadata,
    CsiSourceKind,
};
pub use num_complex::Complex32;
