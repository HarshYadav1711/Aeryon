//! Plugin metadata shared by the registry and runtime.

use core::fmt;

use crate::capability::Capability;
use crate::plugin::PluginId;

/// Semantic version of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version component.
    pub major: u32,
    /// Minor version component.
    pub minor: u32,
    /// Patch version component.
    pub patch: u32,
}

impl Version {
    /// Creates a semantic version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Descriptive metadata for a registered plugin.
///
/// The registry snapshots this information at registration time so callers can
/// inspect plugins without mutating live instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    /// Stable plugin identifier.
    pub id: PluginId,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version.
    pub version: Version,
    /// Short description of plugin behavior.
    pub description: String,
    /// Plugin author or maintainer.
    pub author: String,
    /// Capabilities provided by the plugin.
    pub capabilities: Vec<Capability>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;

    #[test]
    fn version_display_formats_semver() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn metadata_stores_plugin_identity() {
        let metadata = PluginMetadata {
            id: PluginId::new("test.plugin"),
            name: "Test Plugin".into(),
            version: Version::new(0, 1, 0),
            description: "A test plugin".into(),
            author: "Aeryon Contributors".into(),
            capabilities: vec![Capability::Logging],
        };
        assert_eq!(metadata.id.as_str(), "test.plugin");
        assert!(metadata.capabilities.contains(&Capability::Logging));
    }
}
