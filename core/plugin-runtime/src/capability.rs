//! Strongly typed plugin capability declarations.

/// Functional capability offered by a plugin.
///
/// Capabilities are enumerated explicitly so the registry can route work to
/// plugins without string-based capability negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Ingests raw sensor frames.
    Sensor,
    /// Applies sensor correction and normalization.
    Calibration,
    /// Performs signal processing.
    Dsp,
    /// Extracts structured features from processed signals.
    FeatureExtraction,
    /// Executes models or rule engines.
    Inference,
    /// Renders perception output for operators.
    Visualization,
    /// Persists platform artifacts.
    Storage,
    /// Exports artifacts to external systems.
    Exporter,
    /// Imports artifacts from external systems.
    Importer,
    /// Supplies configuration providers.
    Configuration,
    /// Emits structured logs or diagnostics.
    Logging,
}

impl Capability {
    /// Returns every defined capability.
    pub fn all() -> &'static [Capability] {
        &[
            Self::Sensor,
            Self::Calibration,
            Self::Dsp,
            Self::FeatureExtraction,
            Self::Inference,
            Self::Visualization,
            Self::Storage,
            Self::Exporter,
            Self::Importer,
            Self::Configuration,
            Self::Logging,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_are_copy_and_comparable() {
        assert_eq!(Capability::Sensor, Capability::Sensor);
        assert_ne!(Capability::Sensor, Capability::Storage);
    }

    #[test]
    fn all_capabilities_lists_every_variant() {
        assert_eq!(Capability::all().len(), 11);
        assert!(Capability::all().contains(&Capability::Visualization));
    }
}
