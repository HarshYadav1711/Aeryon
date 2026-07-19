//! Deterministic channel-change perception for Aeryon.
//!
//! # Honesty
//!
//! - The first observation describes measured WiFi channel change only.
//! - It does not identify human presence, occupancy, activity, animals, or objects.
//! - The channel-change score is a documented heuristic, not a probability.

#![deny(missing_docs)]

pub mod channel_change;
pub mod errors;
pub mod evidence;
pub mod observation;
pub mod profile;
pub mod service;
pub mod stats;

pub use channel_change::observe_channel_change;
pub use errors::PerceptionError;
pub use evidence::{FeatureEvidence, ObservationEvidence};
pub use observation::{
    ChannelChangeObservation, ChannelChangeState, ObservationUncertainty, RELIABILITY_PROVENANCE,
};
pub use profile::{
    CHANNEL_CHANGE_V1_ID, CHANNEL_CHANGE_V1_VERSION, ChannelChangeProfile, ChannelChangeV1Config,
    PerceptionConfig,
};
pub use service::{FeatureVectorRx, ObservationSink, PerceptionService};
pub use stats::{PerceptionStats, PerceptionWorkerState};

/// Subsystem identifier.
pub const ID: &str = "perception";

/// Returns the subsystem name.
pub fn name() -> &'static str {
    ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_matches_id() {
        assert_eq!(name(), ID);
    }
}
