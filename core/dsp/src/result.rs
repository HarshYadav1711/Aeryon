//! Immutable DSP window result and related identities.

use aeryon_calibration::AntennaLink;
use aeryon_domain::{SensorId, Timestamp};

use crate::motion::MotionEnergySignal;
use crate::spectral::{SamplingAnalysis, SpectralAnalysis};

/// Relative time axis for motion-energy transitions (seconds from window start).
#[derive(Debug, Clone, PartialEq)]
pub struct MotionEnergySeries {
    /// Per-link and aggregate motion-energy proxy values.
    pub signal: MotionEnergySignal,
    /// Relative time of each transition in seconds from the first capture timestamp.
    pub time_axis_secs: Vec<f64>,
}

/// Processing status for one DSP window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DspResultStatus {
    /// Processing completed successfully.
    Success,
    /// Processing failed before a usable result could be published.
    Failed,
}

impl DspResultStatus {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

/// Immutable DSP result for one temporal CSI window.
#[derive(Debug, Clone, PartialEq)]
pub struct DspWindowResult {
    /// Assembler-assigned window identity.
    pub window_id: u64,
    /// Sensor identity.
    pub sensor_id: SensorId,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// First capture timestamp.
    pub first_capture_timestamp: Timestamp,
    /// Last capture timestamp.
    pub last_capture_timestamp: Timestamp,
    /// Frame count in the source window.
    pub frame_count: usize,
    /// Capture-time sampling analysis.
    pub sampling: SamplingAnalysis,
    /// Antenna links present in the result.
    pub antenna_links: Vec<AntennaLink>,
    /// Motion-energy proxy series.
    pub motion_energy: MotionEnergySeries,
    /// One-sided power spectra.
    pub spectra: SpectralAnalysis,
    /// DSP profile identity.
    pub dsp_profile_id: String,
    /// DSP profile version.
    pub dsp_profile_version: u32,
    /// Calibration profile identity shared by the source window.
    pub calibration_profile_id: String,
    /// Calibration profile version shared by the source window.
    pub calibration_profile_version: u32,
    /// Active kernel backend identifier (`rust` or `cpp`).
    pub backend_id: String,
    /// Backend implementation version.
    pub backend_version: String,
    /// Native ABI version when the C++ backend produced this result.
    pub backend_abi_version: Option<u32>,
    /// Processing completion timestamp.
    pub processed_at: Timestamp,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
    /// Result status.
    pub status: DspResultStatus,
}

impl DspWindowResult {
    /// Dominant non-DC frequency from the aggregate spectrum when available,
    /// otherwise the first link with a dominant peak.
    pub fn dominant_non_dc_hz(&self) -> Option<f64> {
        if let Some(aggregate) = &self.spectra.aggregate {
            if aggregate.dominant_non_dc_hz.is_some() {
                return aggregate.dominant_non_dc_hz;
            }
        }
        self.spectra
            .links
            .iter()
            .find_map(|link| link.dominant_non_dc_hz)
    }

    /// Returns spectrum for `(rx, tx)`, if present.
    pub fn spectrum_for_link(
        &self,
        rx: u16,
        tx: u16,
    ) -> Option<&crate::spectral::LinkPowerSpectrum> {
        self.spectra
            .links
            .iter()
            .find(|link| link.link.rx == rx && link.link.tx == tx)
    }

    /// Returns motion-energy values for `(rx, tx)`, if present.
    pub fn motion_for_link(&self, rx: u16, tx: u16) -> Option<&[f64]> {
        self.motion_energy
            .signal
            .links
            .iter()
            .find(|link| link.link.rx == rx && link.link.tx == tx)
            .map(|link| link.values.as_slice())
    }
}
