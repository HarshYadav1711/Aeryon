//! Sampling-time analysis and Hann-windowed one-sided spectral estimation.
//!
//! # Timestamp jitter metric
//!
//! Intervals are derived from consecutive capture timestamps (nanoseconds).
//! Let `median` be the median positive interval. The jitter metric is:
//!
//! ```text
//! jitter = max_i |interval_i − median| / median
//! ```
//!
//! Frequencies use the capture-time timeline (`fs = 1 / median_interval`), never
//! browser arrival time or replay wall-clock delay.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

use crate::backend::{DspKernelBackend, RustKernelBackend};
use crate::errors::{DspError, DspFailureCode};
use crate::motion::LinkMotionEnergy;
use crate::window::CsiWindow;
use aeryon_calibration::AntennaLink;

/// Capture-time sampling statistics for one window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingAnalysis {
    /// Median frame interval in seconds.
    pub median_interval_secs: f64,
    /// Minimum frame interval in seconds.
    pub min_interval_secs: f64,
    /// Maximum frame interval in seconds.
    pub max_interval_secs: f64,
    /// Effective sample rate in hertz (`1 / median_interval`).
    pub effective_sample_rate_hz: f64,
    /// Relative jitter metric: `max |interval − median| / median`.
    pub timestamp_jitter: f64,
}

/// One-sided power spectrum for a single motion-energy series.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkPowerSpectrum {
    /// Antenna link identity.
    pub link: AntennaLink,
    /// Physical frequency bins in hertz (non-negative, DC first).
    pub frequencies_hz: Vec<f64>,
    /// Normalized one-sided power values aligned with `frequencies_hz`.
    pub power: Vec<f64>,
    /// Dominant non-DC frequency when a peak is meaningful.
    pub dominant_non_dc_hz: Option<f64>,
}

/// Spectral analysis outputs for every link plus optional aggregate.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralAnalysis {
    /// Capture-time sampling statistics.
    pub sampling: SamplingAnalysis,
    /// Per-link spectra.
    pub links: Vec<LinkPowerSpectrum>,
    /// Spectrum of the aggregate motion-energy series, when present.
    pub aggregate: Option<LinkPowerSpectrum>,
    /// Warnings that did not abort processing.
    pub warnings: Vec<String>,
}

/// Derives sampling statistics from window capture timestamps.
pub fn analyze_sampling(window: &CsiWindow) -> Result<SamplingAnalysis, DspError> {
    if window.frame_count() < 2 {
        return Err(DspError::Spectral {
            message: "sampling analysis requires at least two frames".to_owned(),
            code: DspFailureCode::InsufficientLength,
        });
    }

    let mut intervals = Vec::with_capacity(window.frame_count() - 1);
    for pair in window.frames().windows(2) {
        let previous = pair[0].capture_timestamp().as_nanos();
        let current = pair[1].capture_timestamp().as_nanos();
        if current < previous {
            return Err(DspError::Spectral {
                message: "capture timestamps are not monotonic".to_owned(),
                code: DspFailureCode::NonMonotonicTimestamp,
            });
        }
        let delta_ns = current - previous;
        if delta_ns == 0 {
            return Err(DspError::Spectral {
                message: "zero capture-time interval is invalid for spectral analysis".to_owned(),
                code: DspFailureCode::InvalidSampleRate,
            });
        }
        intervals.push(delta_ns as f64 / 1_000_000_000.0);
    }

    let median = median_f64(&intervals);
    if !median.is_finite() || median <= 0.0 {
        return Err(DspError::Spectral {
            message: "median capture interval is invalid".to_owned(),
            code: DspFailureCode::InvalidSampleRate,
        });
    }

    let min = intervals.iter().copied().fold(f64::INFINITY, f64::min);
    let max = intervals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let max_dev = intervals
        .iter()
        .map(|interval| (interval - median).abs())
        .fold(0.0_f64, f64::max);
    let jitter = max_dev / median;
    let sample_rate = 1.0 / median;

    if !jitter.is_finite() || !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Err(DspError::Spectral {
            message: "derived sample rate or jitter is non-finite".to_owned(),
            code: DspFailureCode::InvalidSampleRate,
        });
    }

    Ok(SamplingAnalysis {
        median_interval_secs: median,
        min_interval_secs: min,
        max_interval_secs: max,
        effective_sample_rate_hz: sample_rate,
        timestamp_jitter: jitter,
    })
}

/// Applies mean removal, Hann windowing, and one-sided FFT power estimation.
pub fn analyze_spectrum(
    window: &CsiWindow,
    motion: &[LinkMotionEnergy],
    aggregate: Option<&[f64]>,
    jitter_tolerance: f64,
) -> Result<SpectralAnalysis, DspError> {
    analyze_spectrum_with_backend(
        window,
        motion,
        aggregate,
        jitter_tolerance,
        &RustKernelBackend,
    )
}

/// Spectral analysis using the provided kernel backend for temporal preprocessing.
pub fn analyze_spectrum_with_backend(
    window: &CsiWindow,
    motion: &[LinkMotionEnergy],
    aggregate: Option<&[f64]>,
    jitter_tolerance: f64,
    backend: &dyn DspKernelBackend,
) -> Result<SpectralAnalysis, DspError> {
    let sampling = analyze_sampling(window)?;
    let mut warnings = Vec::new();
    if sampling.timestamp_jitter > jitter_tolerance {
        return Err(DspError::Spectral {
            message: format!(
                "timestamp jitter {:.4} exceeds tolerance {:.4}; refusing misleading spectrum",
                sampling.timestamp_jitter, jitter_tolerance
            ),
            code: DspFailureCode::ExcessiveJitter,
        });
    }

    let mut links = Vec::with_capacity(motion.len());
    for series in motion {
        links.push(spectrum_for_series(
            series.link,
            &series.values,
            sampling.effective_sample_rate_hz,
            backend,
        )?);
    }

    let aggregate = match aggregate {
        Some(values) => Some(spectrum_for_series(
            AntennaLink::new(u16::MAX, u16::MAX),
            values,
            sampling.effective_sample_rate_hz,
            backend,
        )?),
        None => None,
    };

    if links.iter().all(|link| link.dominant_non_dc_hz.is_none()) {
        warnings.push("no meaningful non-DC spectral peak detected".to_owned());
    }

    Ok(SpectralAnalysis {
        sampling,
        links,
        aggregate,
        warnings,
    })
}

fn spectrum_for_series(
    link: AntennaLink,
    signal: &[f64],
    sample_rate_hz: f64,
    backend: &dyn DspKernelBackend,
) -> Result<LinkPowerSpectrum, DspError> {
    if signal.len() < 4 {
        return Err(DspError::Spectral {
            message: "spectral analysis requires at least four motion-energy samples".to_owned(),
            code: DspFailureCode::InsufficientLength,
        });
    }
    if !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
        return Err(DspError::Spectral {
            message: "sample rate must be finite and positive".to_owned(),
            code: DspFailureCode::InvalidSampleRate,
        });
    }
    if signal.iter().any(|value| !value.is_finite()) {
        return Err(DspError::Spectral {
            message: "motion-energy signal contains non-finite values".to_owned(),
            code: DspFailureCode::NonFinite,
        });
    }

    let n = signal.len();
    let windowed = backend.center_and_apply_hann(signal)?;
    if windowed.len() != n {
        return Err(DspError::Spectral {
            message: "center/Hann output length mismatch".to_owned(),
            code: DspFailureCode::Spectral,
        });
    }

    let window = hann_window(n);
    let window_power: f64 = window.iter().map(|w| w * w).sum();
    if window_power <= 0.0 || !window_power.is_finite() {
        return Err(DspError::Spectral {
            message: "Hann window normalization failed".to_owned(),
            code: DspFailureCode::Spectral,
        });
    }

    let mut buffer: Vec<Complex<f64>> = windowed
        .iter()
        .map(|value| Complex::new(*value, 0.0))
        .collect();

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buffer);

    let onesided_len = n / 2 + 1;
    let mut frequencies_hz = Vec::with_capacity(onesided_len);
    let mut power = Vec::with_capacity(onesided_len);
    let scale = 1.0 / window_power;

    for (bin, sample) in buffer.iter().take(onesided_len).enumerate() {
        let freq = bin as f64 * sample_rate_hz / n as f64;
        let mut p = sample.norm_sqr() * scale;
        // One-sided spectrum: double interior bins (exclude DC and Nyquist).
        if bin > 0 && !(n % 2 == 0 && bin == onesided_len - 1) {
            p *= 2.0;
        }
        if !freq.is_finite() || !p.is_finite() {
            return Err(DspError::Spectral {
                message: "spectral output contains non-finite values".to_owned(),
                code: DspFailureCode::NonFinite,
            });
        }
        frequencies_hz.push(freq);
        power.push(p);
    }

    let dominant_non_dc_hz = dominant_non_dc(&frequencies_hz, &power);

    Ok(LinkPowerSpectrum {
        link,
        frequencies_hz,
        power,
        dominant_non_dc_hz,
    })
}

/// Symmetric Hann window of length `n` (`0.5 * (1 - cos(2πn/(N-1)))`).
pub fn hann_window(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1.0];
    }
    let denom = (n - 1) as f64;
    (0..n)
        .map(|index| {
            let phase = std::f64::consts::TAU * index as f64 / denom;
            0.5 * (1.0 - phase.cos())
        })
        .collect()
}

fn dominant_non_dc(frequencies_hz: &[f64], power: &[f64]) -> Option<f64> {
    if frequencies_hz.len() < 2 || power.len() != frequencies_hz.len() {
        return None;
    }
    let mut best_index = 1;
    let mut best_power = power[1];
    for (index, value) in power.iter().enumerate().skip(1) {
        if *value > best_power {
            best_power = *value;
            best_index = index;
        }
    }
    if !best_power.is_finite() || best_power <= 0.0 {
        return None;
    }
    // Require the peak to exceed mean non-DC power by a small margin so flat
    // spectra do not report a meaningless "dominant" bin.
    let mean_non_dc = power.iter().skip(1).sum::<f64>() / (power.len() - 1) as f64;
    if best_power < mean_non_dc * 1.05 {
        return None;
    }
    Some(frequencies_hz[best_index])
}

fn median_f64(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{LinkMotionEnergy, compute_motion_energy};
    use aeryon_calibration::{CalibrationPipeline, baseline_csi_v1};
    use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};
    use std::sync::Arc;

    fn calibrated_constant(
        sequence: u64,
        capture_nanos: u64,
    ) -> Arc<aeryon_calibration::CalibratedCsiFrame> {
        let samples = vec![ComplexSample::new(1.0, 0.0); 4];
        let metadata = FrameMetadata {
            frame_id: FrameId::new(sequence + 1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(capture_nanos),
            sequence,
            mission_id: None,
            metadata: Metadata::new(),
        };
        let raw = CsiFrame::try_new(
            metadata,
            Timestamp::from_nanos(capture_nanos),
            None,
            None,
            1,
            1,
            vec![0, 1, 2, 3],
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("raw");
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        Arc::new(pipeline.calibrate(Arc::new(raw)).expect("calibrated"))
    }

    #[test]
    fn constant_signal_after_mean_removal_has_near_zero_ac_power() {
        let signal = vec![3.0; 16];
        let spectrum =
            spectrum_for_series(AntennaLink::new(0, 0), &signal, 10.0, &RustKernelBackend)
                .expect("spectrum");
        assert_eq!(spectrum.frequencies_hz.len(), 9);
        let ac: f64 = spectrum.power.iter().skip(1).sum();
        assert!(ac < 1e-9);
    }

    #[test]
    fn single_frequency_sine_recovers_dominant_bin() {
        let fs = 32.0;
        let n = 32;
        let target_hz = 4.0;
        let signal: Vec<f64> = (0..n)
            .map(|index| (std::f64::consts::TAU * target_hz * index as f64 / fs).sin())
            .collect();
        let spectrum = spectrum_for_series(AntennaLink::new(0, 0), &signal, fs, &RustKernelBackend)
            .expect("spectrum");
        let dominant = spectrum.dominant_non_dc_hz.expect("dominant");
        let bin_hz = fs / n as f64;
        assert!((dominant - target_hz).abs() <= bin_hz + 1e-9);
        assert!(spectrum.power.iter().all(|p| p.is_finite()));
    }

    #[test]
    fn hann_window_endpoints_are_zero() {
        let window = hann_window(8);
        assert!((window[0]).abs() < 1e-12);
        assert!((window[7]).abs() < 1e-12);
        assert!(window[3] > 0.9);
    }

    #[test]
    fn rejects_insufficient_length_and_invalid_rate() {
        let err = spectrum_for_series(
            AntennaLink::new(0, 0),
            &[1.0, 2.0],
            10.0,
            &RustKernelBackend,
        )
        .expect_err("short");
        assert_eq!(err.code(), DspFailureCode::InsufficientLength);
        let err = spectrum_for_series(AntennaLink::new(0, 0), &[1.0; 8], 0.0, &RustKernelBackend)
            .expect_err("rate");
        assert_eq!(err.code(), DspFailureCode::InvalidSampleRate);
    }

    #[test]
    fn irregular_timestamps_fail_jitter_gate() {
        let frames: Vec<_> = (0..8)
            .map(|sequence| {
                let nanos = if sequence == 4 {
                    sequence * 100_000_000 + 80_000_000
                } else {
                    sequence * 100_000_000
                };
                calibrated_constant(sequence, nanos)
            })
            .collect();
        let window = crate::window::CsiWindow::try_new(1, frames).expect("window");
        let motion = compute_motion_energy(&window).expect("motion");
        let err = analyze_spectrum(&window, &motion.links, motion.aggregate.as_deref(), 0.05)
            .expect_err("jitter");
        assert_eq!(err.code(), DspFailureCode::ExcessiveJitter);
    }

    #[test]
    fn sampling_uses_median_interval() {
        let frames: Vec<_> = (0..5)
            .map(|sequence| calibrated_constant(sequence, sequence * 100_000_000))
            .collect();
        let window = crate::window::CsiWindow::try_new(1, frames).expect("window");
        let sampling = analyze_sampling(&window).expect("sampling");
        assert!((sampling.median_interval_secs - 0.1).abs() < 1e-12);
        assert!((sampling.effective_sample_rate_hz - 10.0).abs() < 1e-9);
        assert!(sampling.timestamp_jitter < 1e-12);
    }

    #[test]
    fn link_motion_energy_type_is_used() {
        let series = LinkMotionEnergy {
            link: AntennaLink::new(0, 0),
            values: vec![0.0; 8],
        };
        assert_eq!(series.values.len(), 8);
    }
}
