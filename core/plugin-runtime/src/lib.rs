//! Plugin runtime infrastructure for the Aeryon perception platform.
//!
//! This crate provides plugin contracts, capability declarations, registration,
//! loading, and lifecycle management. It contains infrastructure only; concrete
//! sensor, DSP, and inference plugins are implemented in separate crates.

#![deny(missing_docs)]

pub mod capability;
pub mod errors;
pub mod loader;
pub mod metadata;
pub mod plugin;
pub mod registry;
pub mod runtime;

pub use capability::Capability;
pub use errors::{FactoryError, LifecycleError, LoadError, PluginError, RegistryError};
pub use loader::{PluginFactory, PluginLoader};
pub use metadata::{PluginMetadata, Version};
pub use plugin::{HealthStatus, LifecycleState, Plugin, PluginId};
pub use registry::PluginRegistry;
pub use runtime::PluginRuntime;
