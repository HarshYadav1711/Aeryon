//! Plugin loading infrastructure.

use std::vec::Vec;

use crate::errors::{FactoryError, LoadError, RegistryError};
use crate::plugin::{Plugin, PluginId};
use crate::registry::PluginRegistry;

/// Factory that constructs plugin instances.
///
/// Loaders invoke factories to create plugins before registration. Concrete
/// factories are provided by subsystem crates and built-in plugin bundles.
pub trait PluginFactory: Send + Sync {
    /// Returns the identifier of plugins produced by this factory.
    fn plugin_id(&self) -> &PluginId;

    /// Constructs a plugin instance.
    fn create(&self) -> Result<Box<dyn Plugin>, FactoryError>;
}

/// Loads plugins from registered factories into a registry.
pub struct PluginLoader {
    factories: Vec<Box<dyn PluginFactory>>,
}

impl PluginLoader {
    /// Creates an empty loader.
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Registers a plugin factory.
    pub fn register_factory(&mut self, factory: Box<dyn PluginFactory>) {
        self.factories.push(factory);
    }

    /// Returns the number of registered factories.
    pub fn factory_count(&self) -> usize {
        self.factories.len()
    }

    /// Constructs plugins from all factories and registers them.
    pub fn load_into(&self, registry: &mut PluginRegistry) -> Result<usize, LoadError> {
        let mut loaded = 0;

        for (index, factory) in self.factories.iter().enumerate() {
            let expected_id = factory.plugin_id().clone();
            let plugin = factory
                .create()
                .map_err(|source| LoadError::Factory { index, source })?;

            if plugin.id() != &expected_id {
                return Err(LoadError::Factory {
                    index,
                    source: FactoryError::IdMismatch {
                        expected: expected_id,
                        actual: plugin.id().clone(),
                    },
                });
            }

            registry.register(plugin).map_err(|source| {
                let plugin_id = match &source {
                    RegistryError::DuplicatePluginId(id) => id.clone(),
                    RegistryError::PluginNotFound(id) => id.clone(),
                };
                LoadError::Registration { plugin_id, source }
            })?;

            loaded += 1;
        }

        Ok(loaded)
    }
}

impl Default for PluginLoader {
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

    struct FactoryPlugin {
        id: PluginId,
    }

    impl Plugin for FactoryPlugin {
        fn id(&self) -> &PluginId {
            &self.id
        }

        fn name(&self) -> &str {
            "factory-plugin"
        }

        fn version(&self) -> Version {
            Version::new(0, 0, 1)
        }

        fn description(&self) -> &str {
            "loaded by factory"
        }

        fn author(&self) -> &str {
            "loader-tests"
        }

        fn capabilities(&self) -> &[Capability] {
            &[Capability::Importer]
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

    struct StaticFactory {
        id: PluginId,
    }

    impl PluginFactory for StaticFactory {
        fn plugin_id(&self) -> &PluginId {
            &self.id
        }

        fn create(&self) -> Result<Box<dyn Plugin>, FactoryError> {
            Ok(Box::new(FactoryPlugin {
                id: self.id.clone(),
            }))
        }
    }

    #[test]
    fn loader_registers_plugins_from_factories() {
        let mut loader = PluginLoader::new();
        loader.register_factory(Box::new(StaticFactory {
            id: PluginId::new("factory.one"),
        }));

        let mut registry = PluginRegistry::new();
        let loaded = loader.load_into(&mut registry).expect("load succeeds");
        assert_eq!(loaded, 1);
        assert!(registry.contains(&PluginId::new("factory.one")));
    }

    #[test]
    fn loader_reports_factory_id_mismatch() {
        struct MismatchFactory {
            expected: PluginId,
        }

        impl PluginFactory for MismatchFactory {
            fn plugin_id(&self) -> &PluginId {
                &self.expected
            }

            fn create(&self) -> Result<Box<dyn Plugin>, FactoryError> {
                Ok(Box::new(FactoryPlugin {
                    id: PluginId::new("actual"),
                }))
            }
        }

        let mut loader = PluginLoader::new();
        loader.register_factory(Box::new(MismatchFactory {
            expected: PluginId::new("expected"),
        }));
        let mut registry = PluginRegistry::new();
        let error = loader.load_into(&mut registry).expect_err("mismatch fails");
        assert!(matches!(error, LoadError::Factory { .. }));
    }
}
