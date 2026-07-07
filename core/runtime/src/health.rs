//! Runtime health states.

use core::fmt;

/// Health of the application runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeHealth {
    /// Runtime is starting up.
    Starting,
    /// Runtime is active.
    Running,
    /// Runtime is shutting down.
    Stopping,
    /// Runtime has stopped cleanly.
    Stopped,
    /// Runtime failed to start or shut down cleanly.
    Failed,
}

impl fmt::Display for RuntimeHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_states_have_stable_labels() {
        assert_eq!(RuntimeHealth::Running.to_string(), "running");
        assert_eq!(RuntimeHealth::Failed.to_string(), "failed");
    }
}
