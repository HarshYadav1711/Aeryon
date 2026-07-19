//! Deterministic CSI fixture replay sensor plugin for Aeryon.
//!
//! Replays versioned development fixtures as canonical [`aeryon_csi::CsiFrame`]
//! values. This is an offline development and testing source, not live RF sensing.

#![deny(missing_docs)]

pub mod config;
pub mod plugin;
pub mod stats;

pub use config::{CsiReplayConfig, CsiReplayConfigError};
pub use plugin::{CsiReplayPlugin, PLUGIN_ID, SENSOR_ID, SOURCE_ID};
pub use stats::{CsiReplayCompletion, CsiReplayStats};
