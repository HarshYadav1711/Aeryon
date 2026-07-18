//! Deterministic synthetic sensor plugin for Aeryon.
//!
//! This crate provides integration-test infrastructure that emits reproducible
//! numerical frames. It is not a real environmental perception sensor and does
//! not perform DSP, calibration, or inference.

#![deny(missing_docs)]

pub mod config;
pub mod frame;
pub mod plugin;
pub mod signal;

pub use config::{SyntheticConfigError, SyntheticSensorConfig};
pub use frame::{SOURCE_ID, SyntheticFrame};
pub use plugin::{PLUGIN_ID, SENSOR_ID, SyntheticSensorPlugin};
pub use signal::generate_samples;
