//! Pure window processing pipeline used by the runtime DSP service.

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use aeryon_domain::Timestamp;

use crate::backend::DspKernelBackend;
use crate::errors::DspError;
use crate::motion::compute_motion_energy_with_backend;
use crate::profile::DspProfile;
use crate::result::{DspResultStatus, DspWindowResult, MotionEnergySeries};
use crate::spectral::analyze_spectrum_with_backend;
use crate::window::CsiWindow;

/// Processes one validated [`CsiWindow`] into an immutable [`DspWindowResult`].
pub fn process_window(
    window: &CsiWindow,
    profile: &DspProfile,
    backend: &dyn DspKernelBackend,
) -> Result<DspWindowResult, DspError> {
    let started = Instant::now();
    let identity = backend.identity();
    let motion = compute_motion_energy_with_backend(window, backend)?;
    let spectra = analyze_spectrum_with_backend(
        window,
        &motion.links,
        motion.aggregate.as_deref(),
        profile.timestamp_jitter_tolerance,
        backend,
    )?;

    let first_capture = window.first_capture_timestamp().as_nanos();
    let mut time_axis_secs = Vec::with_capacity(window.frame_count().saturating_sub(1));
    for frame in window.frames().iter().skip(1) {
        let delta_ns = frame
            .capture_timestamp()
            .as_nanos()
            .saturating_sub(first_capture);
        time_axis_secs.push(delta_ns as f64 / 1_000_000_000.0);
    }

    let warnings = spectra.warnings.clone();
    for value in motion
        .links
        .iter()
        .flat_map(|link| link.values.iter())
        .chain(motion.aggregate.iter().flatten())
    {
        if !value.is_finite() {
            return Err(DspError::OutputValidation {
                message: "motion-energy output contains non-finite values".to_owned(),
            });
        }
    }
    for spectrum in spectra.links.iter().chain(spectra.aggregate.iter()) {
        if spectrum.power.iter().any(|value| !value.is_finite())
            || spectrum
                .frequencies_hz
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(DspError::OutputValidation {
                message: "spectral output contains non-finite values".to_owned(),
            });
        }
    }

    let antenna_links = motion.links.iter().map(|link| link.link).collect();
    let duration = started.elapsed();
    let duration_ns = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);

    Ok(DspWindowResult {
        window_id: window.window_id(),
        sensor_id: window.sensor_id(),
        first_sequence: window.first_sequence(),
        last_sequence: window.last_sequence(),
        first_capture_timestamp: window.first_capture_timestamp(),
        last_capture_timestamp: window.last_capture_timestamp(),
        frame_count: window.frame_count(),
        sampling: spectra.sampling,
        antenna_links,
        motion_energy: MotionEnergySeries {
            signal: motion,
            time_axis_secs,
        },
        spectra,
        dsp_profile_id: profile.id.clone(),
        dsp_profile_version: profile.version,
        backend_id: identity.kind.as_str().to_owned(),
        backend_version: identity.implementation_version.clone(),
        backend_abi_version: identity.abi_version,
        processed_at: now(),
        processing_duration_ns: duration_ns,
        warnings,
        status: DspResultStatus::Success,
    })
}

/// Processes a window using a shared backend handle.
pub fn process_window_arc(
    window: &CsiWindow,
    profile: &DspProfile,
    backend: &Arc<dyn DspKernelBackend>,
) -> Result<DspWindowResult, DspError> {
    process_window(window, profile, backend.as_ref())
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}
