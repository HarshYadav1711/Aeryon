//! Deterministic synthetic signal generation.

use crate::config::SyntheticSensorConfig;

/// Generates deterministic dual-sine samples for a frame.
///
/// `sample[n] = sin(2π f1 t) + amplitude × sin(2π f2 t)` where
/// `t = (sequence × samples_per_frame + n) / sample_rate`.
pub fn generate_samples(config: &SyntheticSensorConfig, sequence: u64) -> Vec<f64> {
    let start = sequence.saturating_mul(config.samples_per_frame as u64) as f64;
    let mut samples = Vec::with_capacity(config.samples_per_frame);

    for offset in 0..config.samples_per_frame {
        let n = start + offset as f64;
        let t = n / config.sample_rate_hz;
        let primary = (std::f64::consts::TAU * config.primary_frequency_hz * t).sin();
        let secondary = config.secondary_amplitude
            * (std::f64::consts::TAU * config.secondary_frequency_hz * t).sin();
        samples.push(primary + secondary);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_config_produces_identical_frames() {
        let config = SyntheticSensorConfig::default();
        let first = generate_samples(&config, 0);
        let second = generate_samples(&config, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn frame_size_matches_configuration() {
        let config = SyntheticSensorConfig {
            samples_per_frame: 32,
            ..SyntheticSensorConfig::default()
        };
        assert_eq!(generate_samples(&config, 3).len(), 32);
    }

    #[test]
    fn successive_sequences_differ() {
        let config = SyntheticSensorConfig::default();
        let first = generate_samples(&config, 0);
        let second = generate_samples(&config, 1);
        assert_ne!(first, second);
    }
}
