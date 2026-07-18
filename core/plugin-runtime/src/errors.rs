//! Typed errors for plugin registration, loading, and lifecycle management.

use core::fmt;

use crate::plugin::LifecycleState;
use crate::plugin::PluginId;

/// Errors produced by plugin factories during construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    /// The factory returned no plugin instance.
    EmptyFactory,
    /// The constructed plugin reported a mismatched identifier.
    IdMismatch {
        /// Expected plugin identifier.
        expected: PluginId,
        /// Identifier reported by the constructed plugin.
        actual: PluginId,
    },
}

/// Errors produced while loading plugins into the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// A factory failed to construct a plugin.
    Factory {
        /// Factory index in the load batch.
        index: usize,
        /// Underlying factory error.
        source: FactoryError,
    },
    /// Registration failed while loading a constructed plugin.
    Registration {
        /// Plugin identifier involved in the failure.
        plugin_id: PluginId,
        /// Underlying registry error.
        source: RegistryError,
    },
}

/// Errors produced by the plugin registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// A plugin with the same identifier is already registered.
    DuplicatePluginId(PluginId),
    /// The requested plugin was not found.
    PluginNotFound(PluginId),
}

/// Errors produced by lifecycle transitions in the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    /// The plugin is not registered in the runtime.
    PluginNotFound(PluginId),
    /// The requested lifecycle transition is not valid.
    InvalidTransition {
        /// Plugin identifier.
        plugin_id: PluginId,
        /// Current lifecycle state.
        from: LifecycleState,
        /// Requested lifecycle state.
        to: LifecycleState,
    },
    /// Plugin initialization failed.
    InitializationFailed(PluginId),
    /// Plugin shutdown failed.
    ShutdownFailed(PluginId),
}

/// Top-level plugin runtime error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    /// Registry operation failed.
    Registry(RegistryError),
    /// Plugin loading failed.
    Load(LoadError),
    /// Lifecycle operation failed.
    Lifecycle(LifecycleError),
}

impl PluginError {
    /// Creates a registry error.
    pub fn registry(error: RegistryError) -> Self {
        Self::Registry(error)
    }

    /// Creates a lifecycle error.
    pub fn lifecycle(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}

impl From<RegistryError> for PluginError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<LoadError> for PluginError {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<LifecycleError> for PluginError {
    fn from(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}

impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFactory => f.write_str("plugin factory returned no instance"),
            Self::IdMismatch { expected, actual } => {
                write!(f, "plugin id mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Factory { index, source } => {
                write!(f, "factory {index} failed: {source}")
            }
            Self::Registration { plugin_id, source } => {
                write!(f, "failed to register {plugin_id}: {source}")
            }
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePluginId(id) => write!(f, "duplicate plugin id: {id}"),
            Self::PluginNotFound(id) => write!(f, "plugin not found: {id}"),
        }
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PluginNotFound(id) => write!(f, "plugin not found: {id}"),
            Self::InvalidTransition {
                plugin_id,
                from,
                to,
            } => write!(f, "invalid transition for {plugin_id}: {from} -> {to}"),
            Self::InitializationFailed(id) => write!(f, "initialization failed: {id}"),
            Self::ShutdownFailed(id) => write!(f, "shutdown failed: {id}"),
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(f, "{error}"),
            Self::Load(error) => write!(f, "{error}"),
            Self::Lifecycle(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FactoryError {}
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Factory { source, .. } => Some(source),
            Self::Registration { source, .. } => Some(source),
        }
    }
}
impl std::error::Error for RegistryError {}
impl std::error::Error for LifecycleError {}
impl std::error::Error for PluginError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Registry(error) => Some(error),
            Self::Load(error) => Some(error),
            Self::Lifecycle(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::LifecycleState;

    #[test]
    fn display_formats_duplicate_plugin_id() {
        let error = RegistryError::DuplicatePluginId(PluginId::new("dup"));
        assert!(error.to_string().contains("dup"));
    }

    #[test]
    fn plugin_error_converts_from_registry_error() {
        let error: PluginError = RegistryError::PluginNotFound(PluginId::new("missing")).into();
        assert!(matches!(error, PluginError::Registry(_)));
    }

    #[test]
    fn lifecycle_error_describes_invalid_transition() {
        let error = LifecycleError::InvalidTransition {
            plugin_id: PluginId::new("demo"),
            from: LifecycleState::Registered,
            to: LifecycleState::Running,
        };
        assert!(error.to_string().contains("registered"));
        assert!(error.to_string().contains("running"));
    }
}
