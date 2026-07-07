//! Sensor backend plugin interface for the Aeryon perception platform.

/// Subsystem identifier.
pub const ID: &str = "plugins";

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
