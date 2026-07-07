//! Plugin identity and lifecycle contracts.

use core::fmt;

use crate::capability::Capability;
use crate::errors::PluginError;
use crate::metadata::Version;

/// Stable identifier for a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginId(String);

impl PluginId {
    /// Creates a plugin identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lifecycle state of a plugin managed by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleState {
    /// Plugin is registered but not initialized.
    Registered,
    /// Plugin completed initialization successfully.
    Initialized,
    /// Plugin is active and available for work.
    Running,
    /// Plugin was shut down cleanly.
    Stopped,
    /// Plugin entered a failure state.
    Failed,
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Registered => "registered",
            Self::Initialized => "initialized",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        };
        f.write_str(label)
    }
}

/// Runtime health reported by a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Plugin is operating normally.
    Healthy,
    /// Plugin is operational with reduced functionality.
    Degraded,
    /// Plugin is not fit for service.
    Unhealthy,
}

/// Contract implemented by every Aeryon plugin.
///
/// Plugins expose identity, capabilities, and lifecycle hooks so the runtime can
/// manage them uniformly regardless of sensing modality or implementation language.
pub trait Plugin: Send + Sync {
    /// Returns the stable plugin identifier.
    fn id(&self) -> &PluginId;

    /// Returns the human-readable plugin name.
    fn name(&self) -> &str;

    /// Returns the plugin version.
    fn version(&self) -> Version;

    /// Returns a short description of plugin behavior.
    fn description(&self) -> &str;

    /// Returns the plugin author or maintainer.
    fn author(&self) -> &str;

    /// Returns the capabilities implemented by the plugin.
    fn capabilities(&self) -> &[Capability];

    /// Prepares the plugin for operation.
    fn initialize(&mut self) -> Result<(), PluginError>;

    /// Releases resources held by the plugin.
    fn shutdown(&mut self) -> Result<(), PluginError>;

    /// Returns the current health of the plugin.
    fn health(&self) -> HealthStatus;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;

    struct BenchPlugin {
        id: PluginId,
        initialized: bool,
        running: bool,
    }

    impl BenchPlugin {
        fn new(id: &str) -> Self {
            Self {
                id: PluginId::new(id),
                initialized: false,
                running: false,
            }
        }
    }

    impl Plugin for BenchPlugin {
        fn id(&self) -> &PluginId {
            &self.id
        }

        fn name(&self) -> &str {
            "bench-plugin"
        }

        fn version(&self) -> Version {
            Version::new(0, 1, 0)
        }

        fn description(&self) -> &str {
            "Benchmark plugin"
        }

        fn author(&self) -> &str {
            "Aeryon Contributors"
        }

        fn capabilities(&self) -> &[Capability] {
            &[Capability::Logging]
        }

        fn initialize(&mut self) -> Result<(), PluginError> {
            self.initialized = true;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), PluginError> {
            self.running = false;
            self.initialized = false;
            Ok(())
        }

        fn health(&self) -> HealthStatus {
            if self.running {
                HealthStatus::Healthy
            } else if self.initialized {
                HealthStatus::Degraded
            } else {
                HealthStatus::Unhealthy
            }
        }
    }

    #[test]
    fn plugin_id_orders_lexicographically() {
        assert!(PluginId::new("a") < PluginId::new("b"));
    }

    #[test]
    fn plugin_trait_exposes_identity_and_capabilities() {
        let plugin = BenchPlugin::new("bench.logging");
        assert_eq!(plugin.id().as_str(), "bench.logging");
        assert_eq!(plugin.capabilities(), &[Capability::Logging]);
    }

    #[test]
    fn lifecycle_state_display_is_stable() {
        assert_eq!(LifecycleState::Running.to_string(), "running");
    }
}
