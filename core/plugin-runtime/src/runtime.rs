//! Plugin runtime lifecycle management.

use std::collections::BTreeMap;

use crate::errors::{LifecycleError, PluginError};
use crate::plugin::{HealthStatus, LifecycleState, PluginId};
use crate::registry::PluginRegistry;

/// Manages plugin lifecycle on top of the registry.
///
/// The runtime coordinates initialization, activation, shutdown, and health
/// inspection while preserving registry ownership of plugin instances.
pub struct PluginRuntime {
    registry: PluginRegistry,
    states: BTreeMap<PluginId, LifecycleState>,
}

impl PluginRuntime {
    /// Creates an empty runtime.
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            states: BTreeMap::new(),
        }
    }

    /// Returns a shared reference to the underlying registry.
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Returns a mutable reference to the underlying registry.
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }

    /// Registers a plugin and marks it as `Registered`.
    pub fn register(&mut self, plugin: Box<dyn crate::plugin::Plugin>) -> Result<(), PluginError> {
        let id = plugin.id().clone();
        self.registry.register(plugin)?;
        self.states.insert(id, LifecycleState::Registered);
        Ok(())
    }

    /// Unregisters a plugin after ensuring it is not running.
    pub fn unregister(&mut self, id: &PluginId) -> Result<(), PluginError> {
        let state = self
            .state_of(id)
            .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;

        if state == LifecycleState::Running {
            return Err(LifecycleError::InvalidTransition {
                plugin_id: id.clone(),
                from: state,
                to: LifecycleState::Registered,
            }
            .into());
        }

        if state == LifecycleState::Initialized {
            self.stop(id)?;
        }

        self.registry.unregister(id)?;
        self.states.remove(id);
        Ok(())
    }

    /// Initializes and starts a plugin.
    pub fn start(&mut self, id: &PluginId) -> Result<(), PluginError> {
        let state = self
            .state_of(id)
            .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;

        match state {
            LifecycleState::Registered | LifecycleState::Stopped => {
                let plugin = self
                    .registry
                    .lookup_mut(id)
                    .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;

                if plugin.initialize().is_err() {
                    self.states.insert(id.clone(), LifecycleState::Failed);
                    return Err(LifecycleError::InitializationFailed(id.clone()).into());
                }

                self.states.insert(id.clone(), LifecycleState::Running);
                Ok(())
            }
            LifecycleState::Running => Ok(()),
            LifecycleState::Initialized => {
                self.states.insert(id.clone(), LifecycleState::Running);
                Ok(())
            }
            LifecycleState::Failed => Err(LifecycleError::InvalidTransition {
                plugin_id: id.clone(),
                from: state,
                to: LifecycleState::Running,
            }
            .into()),
        }
    }

    /// Shuts down a running plugin.
    pub fn stop(&mut self, id: &PluginId) -> Result<(), PluginError> {
        let state = self
            .state_of(id)
            .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;

        match state {
            LifecycleState::Running | LifecycleState::Initialized => {
                let plugin = self
                    .registry
                    .lookup_mut(id)
                    .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;

                if plugin.shutdown().is_err() {
                    self.states.insert(id.clone(), LifecycleState::Failed);
                    return Err(LifecycleError::ShutdownFailed(id.clone()).into());
                }

                self.states.insert(id.clone(), LifecycleState::Stopped);
                Ok(())
            }
            LifecycleState::Stopped | LifecycleState::Registered => Ok(()),
            LifecycleState::Failed => Err(LifecycleError::InvalidTransition {
                plugin_id: id.clone(),
                from: state,
                to: LifecycleState::Stopped,
            }
            .into()),
        }
    }

    /// Returns the lifecycle state of a plugin.
    pub fn lifecycle_state(&self, id: &PluginId) -> Option<LifecycleState> {
        self.state_of(id)
    }

    /// Returns the health reported by a plugin.
    pub fn health(&self, id: &PluginId) -> Result<HealthStatus, PluginError> {
        let plugin = self
            .registry
            .lookup(id)
            .ok_or_else(|| LifecycleError::PluginNotFound(id.clone()))?;
        Ok(plugin.health())
    }

    /// Returns lifecycle states for all managed plugins.
    pub fn lifecycle_snapshot(&self) -> Vec<(PluginId, LifecycleState)> {
        self.states
            .iter()
            .map(|(id, state)| (id.clone(), *state))
            .collect()
    }

    fn state_of(&self, id: &PluginId) -> Option<LifecycleState> {
        self.states.get(id).copied()
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl From<PluginRegistry> for PluginRuntime {
    fn from(registry: PluginRegistry) -> Self {
        let states = registry
            .list()
            .into_iter()
            .map(|metadata| (metadata.id, LifecycleState::Registered))
            .collect();

        Self { registry, states }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::errors::RegistryError;
    use crate::metadata::Version;
    use crate::plugin::{HealthStatus, Plugin};

    struct LifecyclePlugin {
        id: PluginId,
        initialized: bool,
        fail_init: bool,
    }

    impl LifecyclePlugin {
        fn new(id: &str) -> Self {
            Self {
                id: PluginId::new(id),
                initialized: false,
                fail_init: false,
            }
        }
    }

    impl Plugin for LifecyclePlugin {
        fn id(&self) -> &PluginId {
            &self.id
        }

        fn name(&self) -> &str {
            "lifecycle-plugin"
        }

        fn version(&self) -> Version {
            Version::new(0, 1, 0)
        }

        fn description(&self) -> &str {
            "lifecycle test plugin"
        }

        fn author(&self) -> &str {
            "tests"
        }

        fn capabilities(&self) -> &[Capability] {
            &[Capability::Configuration]
        }

        fn initialize(&mut self) -> Result<(), PluginError> {
            if self.fail_init {
                return Err(PluginError::lifecycle(
                    LifecycleError::InitializationFailed(self.id.clone()),
                ));
            }
            self.initialized = true;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), PluginError> {
            self.initialized = false;
            Ok(())
        }

        fn health(&self) -> HealthStatus {
            if self.initialized {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            }
        }
    }

    #[test]
    fn registration_starts_in_registered_state() {
        let mut runtime = PluginRuntime::new();
        runtime
            .register(Box::new(LifecyclePlugin::new("life")))
            .expect("register succeeds");
        assert_eq!(
            runtime.lifecycle_state(&PluginId::new("life")),
            Some(LifecycleState::Registered)
        );
    }

    #[test]
    fn start_transitions_to_running() {
        let mut runtime = PluginRuntime::new();
        let id = PluginId::new("life");
        runtime
            .register(Box::new(LifecyclePlugin::new("life")))
            .expect("register succeeds");
        runtime.start(&id).expect("start succeeds");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Running));
        assert_eq!(runtime.health(&id).expect("health"), HealthStatus::Healthy);
    }

    #[test]
    fn stop_transitions_to_stopped() {
        let mut runtime = PluginRuntime::new();
        let id = PluginId::new("life");
        runtime
            .register(Box::new(LifecyclePlugin::new("life")))
            .expect("register succeeds");
        runtime.start(&id).expect("start succeeds");
        runtime.stop(&id).expect("stop succeeds");
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Stopped));
    }

    #[test]
    fn failed_initialization_marks_plugin_failed() {
        let mut plugin = LifecyclePlugin::new("broken");
        plugin.fail_init = true;
        let mut runtime = PluginRuntime::new();
        let id = PluginId::new("broken");
        runtime
            .register(Box::new(plugin))
            .expect("register succeeds");
        let error = runtime.start(&id).expect_err("start fails");
        assert!(matches!(
            error,
            PluginError::Lifecycle(LifecycleError::InitializationFailed(_))
        ));
        assert_eq!(runtime.lifecycle_state(&id), Some(LifecycleState::Failed));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut runtime = PluginRuntime::new();
        runtime
            .register(Box::new(LifecyclePlugin::new("dup")))
            .expect("first register succeeds");
        let error = runtime
            .register(Box::new(LifecyclePlugin::new("dup")))
            .expect_err("duplicate rejected");
        assert!(matches!(
            error,
            PluginError::Registry(RegistryError::DuplicatePluginId(_))
        ));
    }

    #[test]
    fn lifecycle_snapshot_lists_all_plugins() {
        let mut runtime = PluginRuntime::new();
        runtime
            .register(Box::new(LifecyclePlugin::new("one")))
            .expect("register one");
        runtime
            .register(Box::new(LifecyclePlugin::new("two")))
            .expect("register two");
        assert_eq!(runtime.lifecycle_snapshot().len(), 2);
    }
}
