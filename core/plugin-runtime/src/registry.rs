//! Plugin registry for discovery and lookup.

use std::collections::BTreeMap;
use std::vec::Vec;

use crate::capability::Capability;
use crate::errors::RegistryError;
use crate::metadata::PluginMetadata;
use crate::plugin::{Plugin, PluginId};

/// Internal record stored by the registry.
struct PluginEntry {
    metadata: PluginMetadata,
    plugin: Box<dyn Plugin>,
}

/// Registry of installed plugins.
///
/// The registry owns plugin instances and exposes lookup APIs for the runtime
/// and orchestration layers.
pub struct PluginRegistry {
    plugins: BTreeMap<PluginId, PluginEntry>,
}

impl PluginRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
        }
    }

    /// Registers a plugin, rejecting duplicate identifiers.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), RegistryError> {
        let id = plugin.id().clone();
        if self.plugins.contains_key(&id) {
            return Err(RegistryError::DuplicatePluginId(id));
        }

        let metadata = PluginMetadata {
            id: id.clone(),
            name: plugin.name().to_owned(),
            version: plugin.version(),
            description: plugin.description().to_owned(),
            author: plugin.author().to_owned(),
            capabilities: plugin.capabilities().to_vec(),
        };

        self.plugins.insert(id, PluginEntry { metadata, plugin });
        Ok(())
    }

    /// Unregisters a plugin and returns the removed instance.
    pub fn unregister(&mut self, id: &PluginId) -> Result<Box<dyn Plugin>, RegistryError> {
        self.plugins
            .remove(id)
            .map(|entry| entry.plugin)
            .ok_or_else(|| RegistryError::PluginNotFound(id.clone()))
    }

    /// Returns a shared reference to a registered plugin.
    pub fn lookup(&self, id: &PluginId) -> Option<&dyn Plugin> {
        self.plugins.get(id).map(|entry| entry.plugin.as_ref())
    }

    /// Returns a mutable reference to a registered plugin.
    pub fn lookup_mut<'a>(&'a mut self, id: &PluginId) -> Option<&'a mut (dyn Plugin + 'a)> {
        match self.plugins.get_mut(id) {
            Some(entry) => Some(entry.plugin.as_mut()),
            None => None,
        }
    }

    /// Returns metadata for every registered plugin.
    pub fn list(&self) -> Vec<PluginMetadata> {
        self.plugins
            .values()
            .map(|entry| entry.metadata.clone())
            .collect()
    }

    /// Returns metadata for plugins that provide `capability`.
    pub fn query_by_capability(&self, capability: Capability) -> Vec<PluginMetadata> {
        self.plugins
            .values()
            .filter(|entry| entry.metadata.capabilities.contains(&capability))
            .map(|entry| entry.metadata.clone())
            .collect()
    }

    /// Returns `true` when a plugin with `id` is registered.
    pub fn contains(&self, id: &PluginId) -> bool {
        self.plugins.contains_key(id)
    }

    /// Returns the number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns `true` when no plugins are registered.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::metadata::Version;
    use crate::plugin::{HealthStatus, Plugin};

    struct TestPlugin {
        id: PluginId,
        capabilities: Vec<Capability>,
    }

    impl TestPlugin {
        fn new(id: &str, capabilities: Vec<Capability>) -> Self {
            Self {
                id: PluginId::new(id),
                capabilities,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn id(&self) -> &PluginId {
            &self.id
        }

        fn name(&self) -> &str {
            "test-plugin"
        }

        fn version(&self) -> Version {
            Version::new(0, 1, 0)
        }

        fn description(&self) -> &str {
            "test plugin"
        }

        fn author(&self) -> &str {
            "tests"
        }

        fn capabilities(&self) -> &[Capability] {
            &self.capabilities
        }

        fn initialize(&mut self) -> Result<(), crate::errors::PluginError> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), crate::errors::PluginError> {
            Ok(())
        }

        fn health(&self) -> HealthStatus {
            HealthStatus::Healthy
        }
    }

    #[test]
    fn register_and_lookup_plugin() {
        let mut registry = PluginRegistry::new();
        let id = PluginId::new("alpha");
        registry
            .register(Box::new(TestPlugin::new(
                "alpha",
                vec![Capability::Storage],
            )))
            .expect("register succeeds");
        assert!(registry.lookup(&id).is_some());
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(TestPlugin::new("dup", vec![Capability::Logging])))
            .expect("first register succeeds");
        let error = registry
            .register(Box::new(TestPlugin::new("dup", vec![Capability::Logging])))
            .expect_err("duplicate rejected");
        assert_eq!(
            error,
            RegistryError::DuplicatePluginId(PluginId::new("dup"))
        );
    }

    #[test]
    fn query_by_capability_filters_plugins() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(TestPlugin::new(
                "sensor-a",
                vec![Capability::Sensor],
            )))
            .expect("register sensor");
        registry
            .register(Box::new(TestPlugin::new(
                "storage-a",
                vec![Capability::Storage],
            )))
            .expect("register storage");

        let sensors = registry.query_by_capability(Capability::Sensor);
        assert_eq!(sensors.len(), 1);
        assert_eq!(sensors[0].id, PluginId::new("sensor-a"));
    }

    #[test]
    fn unregister_removes_plugin() {
        let mut registry = PluginRegistry::new();
        let id = PluginId::new("temp");
        registry
            .register(Box::new(TestPlugin::new("temp", vec![])))
            .expect("register succeeds");
        registry.unregister(&id).expect("unregister succeeds");
        assert!(registry.lookup(&id).is_none());
    }

    #[test]
    fn list_returns_all_metadata() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(TestPlugin::new("one", vec![])))
            .expect("register one");
        registry
            .register(Box::new(TestPlugin::new("two", vec![])))
            .expect("register two");
        assert_eq!(registry.list().len(), 2);
    }
}
