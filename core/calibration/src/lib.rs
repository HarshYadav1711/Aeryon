//! Sensor correction and normalization.

/// Subsystem identifier.
pub const ID: &str = "calibration";

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
