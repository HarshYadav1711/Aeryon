//! Explicit API DTOs. Internal runtime structures are not serialized directly.

use serde::{Deserialize, Serialize};

/// Standard error envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorResponse {
    /// Nested error body.
    pub error: ErrorBody,
}

/// Error detail payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorBody {
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable summary.
    pub message: String,
}

/// Synthetic sensor subset embedded in health responses.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntheticHealthSummary {
    /// Whether the synthetic source is enabled in configuration.
    pub enabled: bool,
    /// Plugin lifecycle label when registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,
    /// Plugin health label when registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
}

/// CSI replay subset embedded in health responses.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiReplayHealthSummary {
    /// Whether CSI replay is enabled in configuration.
    pub enabled: bool,
    /// Plugin lifecycle label when registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,
    /// Plugin health label when registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// Replay completion classification when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<String>,
}

/// `GET /health` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HealthResponse {
    /// Overall runtime lifecycle label.
    pub status: String,
    /// Whether the runtime is considered healthy for operators.
    pub healthy: bool,
    /// Process uptime in seconds.
    pub uptime_secs: f64,
    /// Response generation timestamp (RFC 3339).
    pub timestamp: String,
    /// Whether the typed event consumer task is running.
    pub event_consumer_running: bool,
    /// Synthetic sensor summary when the source is configured.
    pub synthetic_sensor: SyntheticHealthSummary,
    /// CSI replay summary when the source is configured.
    pub csi_replay: CsiReplayHealthSummary,
}

/// `GET /api/v1/runtime` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeSnapshot {
    /// Application name from configuration.
    pub application_name: String,
    /// Application version.
    pub application_version: String,
    /// Runtime lifecycle state.
    pub lifecycle_state: String,
    /// Process uptime in seconds.
    pub uptime_secs: f64,
    /// Wall-clock startup timestamp (RFC 3339).
    pub startup_timestamp: String,
    /// Number of registered plugins.
    pub registered_plugin_count: usize,
    /// Number of plugins currently running.
    pub active_plugin_count: usize,
    /// Frames received by the runtime event consumer.
    pub frames_received: u64,
    /// Last observed frame sequence, if any.
    pub last_frame_sequence: Option<u64>,
    /// Last observed frame timestamp (RFC 3339), if any.
    pub last_frame_timestamp: Option<String>,
    /// Synthetic sensor lifecycle when tracked.
    pub synthetic_sensor_lifecycle: Option<String>,
    /// Whether the synthetic source is enabled in configuration.
    pub synthetic_source_enabled: bool,
    /// CSI replay lifecycle when tracked.
    pub csi_replay_lifecycle: Option<String>,
    /// Whether CSI replay is enabled in configuration.
    pub csi_replay_enabled: bool,
    /// Active development source label (`synthetic`, `csi_replay`, or `none`).
    pub active_source: String,
}

/// `GET /api/v1/plugins` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginsResponse {
    /// Registered plugin summaries.
    pub plugins: Vec<PluginSummary>,
}

/// Public plugin summary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginSummary {
    /// Plugin identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Declared capabilities.
    pub capabilities: Vec<String>,
    /// Lifecycle state label.
    pub lifecycle_state: String,
    /// Health label.
    pub health: String,
}

/// `GET /api/v1/sensors/synthetic` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntheticSensorSnapshot {
    /// Whether the synthetic source is enabled in configuration.
    pub enabled: bool,
    /// Plugin lifecycle when registered.
    pub lifecycle_state: Option<String>,
    /// Configured frame interval in milliseconds.
    pub configured_interval_ms: u64,
    /// Configured samples per frame.
    pub samples_per_frame: usize,
    /// Configured sample rate in hertz.
    pub sample_rate_hz: f64,
    /// Configured primary and secondary frequencies.
    pub configured_frequencies_hz: ConfiguredFrequencies,
    /// Frames received by the runtime consumer.
    pub frames_received: u64,
    /// Last frame sequence, if any frame has arrived.
    pub last_sequence: Option<u64>,
    /// Last frame timestamp (RFC 3339), if any frame has arrived.
    pub last_frame_timestamp: Option<String>,
    /// Plugin health when registered.
    pub health: Option<String>,
}

/// Configured synthetic signal frequencies.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfiguredFrequencies {
    /// Primary sine frequency in hertz.
    pub primary_hz: f64,
    /// Secondary sine frequency in hertz.
    pub secondary_hz: f64,
}

/// `GET /api/v1/sensors/csi-replay` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiReplaySnapshot {
    /// Whether CSI replay is enabled in configuration.
    pub enabled: bool,
    /// Plugin lifecycle when registered.
    pub lifecycle_state: Option<String>,
    /// Plugin health when registered.
    pub health: Option<String>,
    /// Source type marker (always `csi_replay` for this endpoint).
    pub source_type: &'static str,
    /// Data classification for operators.
    pub data_classification: &'static str,
    /// Repository-relative or display-safe fixture path.
    pub fixture_path: String,
    /// Whether loop playback is enabled.
    pub loop_playback: bool,
    /// Configured frame interval in milliseconds.
    pub frame_interval_ms: u64,
    /// Configured maximum frames (`0` means fixture/natural limit only).
    pub maximum_frames: u64,
    /// Frames read from the fixture.
    pub frames_read: u64,
    /// Frames accepted and published.
    pub frames_accepted: u64,
    /// Frames rejected due to validation failures.
    pub frames_rejected: u64,
    /// Latest accepted sequence, if any.
    pub latest_sequence: Option<u64>,
    /// Latest accepted frame timestamp (RFC 3339), if any.
    pub latest_frame_timestamp: Option<String>,
    /// Latest receive antenna count, if any frame arrived.
    pub receive_antennas: Option<u16>,
    /// Latest transmit antenna count, if any frame arrived.
    pub transmit_antennas: Option<u16>,
    /// Latest subcarrier count, if any frame arrived.
    pub subcarrier_count: Option<u16>,
    /// Latest center frequency in hertz, when present.
    pub center_frequency_hz: Option<f64>,
    /// Latest bandwidth in hertz, when present.
    pub bandwidth_hz: Option<f64>,
    /// Replay completion classification.
    pub completion: String,
    /// Last replay error message when appropriate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Versioned WebSocket event envelope.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApiEventEnvelope {
    /// Wire protocol version.
    pub version: u32,
    /// Event type discriminator.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Envelope timestamp (RFC 3339).
    pub timestamp: String,
    /// Event-specific payload.
    pub payload: serde_json::Value,
}

/// Lightweight frame event payload (no sample arrays).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SensorFramePayload {
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Frame identifier when present.
    pub frame_id: u64,
    /// Capture timestamp (RFC 3339).
    pub capture_timestamp: String,
    /// Configured samples per frame (synthetic) or subcarrier×link count (CSI).
    pub samples_per_frame: usize,
    /// Source type marker.
    pub source_type: &'static str,
}

/// Lightweight CSI frame metadata payload (no complex sample matrix).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiFramePayload {
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Frame identifier.
    pub frame_id: u64,
    /// Capture timestamp (RFC 3339).
    pub capture_timestamp: String,
    /// Replay/receive timestamp (RFC 3339).
    pub receive_timestamp: String,
    /// Receive antenna count.
    pub receive_antennas: u16,
    /// Transmit antenna count.
    pub transmit_antennas: u16,
    /// Subcarrier count.
    pub subcarrier_count: u16,
    /// Optional center frequency in hertz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_frequency_hz: Option<f64>,
    /// Optional bandwidth in hertz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_hz: Option<f64>,
    /// Source type marker.
    pub source_type: &'static str,
    /// Explicit development-fixture classification.
    pub data_classification: &'static str,
    /// Operator-facing honesty flag.
    pub live_hardware: bool,
}

/// Sensor lifecycle event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SensorLifecyclePayload {
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Optional failure classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
}

/// CSI replay lifecycle event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiReplayLifecyclePayload {
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Source type marker.
    pub source_type: &'static str,
    /// Explicit development-fixture classification.
    pub data_classification: &'static str,
    /// Optional failure classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    /// Frames accepted when completion is reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames_accepted: Option<u64>,
}

/// `GET /api/v1/calibration` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationSnapshot {
    /// Whether calibration is enabled in configuration.
    pub enabled: bool,
    /// Calibration worker state label.
    pub worker_state: String,
    /// Active profile identity when configured.
    pub profile_id: Option<String>,
    /// Active profile version when configured.
    pub profile_version: Option<u32>,
    /// Ordered enabled stage names.
    pub stages: Vec<String>,
    /// Raw frames submitted to the calibration worker.
    pub raw_frames_submitted: u64,
    /// Successfully calibrated frames.
    pub frames_calibrated: u64,
    /// Calibration failures.
    pub frames_failed: u64,
    /// Latest calibrated sequence, if any.
    pub latest_sequence: Option<u64>,
    /// Latest calibrated timestamp (RFC 3339), if any.
    pub latest_calibrated_timestamp: Option<String>,
    /// Last calibration duration in nanoseconds.
    pub last_duration_ns: Option<u64>,
    /// Average calibration duration in nanoseconds.
    pub average_duration_ns: Option<u64>,
    /// Last warning summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_warning: Option<String>,
    /// Last error summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Approximate queue depth when tracked.
    pub queue_depth: u64,
    /// Calibration health label derived from worker/failure state.
    pub health: String,
    /// Data source honesty label.
    pub data_classification: &'static str,
}

/// Calibration started event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationStartedPayload {
    /// Profile identity.
    pub profile_id: String,
    /// Profile version.
    pub profile_version: u32,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// Successful CSI frame calibration metadata payload (no sample matrices).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiFrameCalibratedPayload {
    /// Raw frame identifier.
    pub raw_frame_id: u64,
    /// Sensor identifier.
    pub sensor_id: u64,
    /// Sequence number.
    pub sequence: u64,
    /// Profile identity.
    pub profile_id: String,
    /// Profile version.
    pub profile_version: u32,
    /// Executed stage count.
    pub stage_count: u16,
    /// Calibration duration in nanoseconds.
    pub calibration_duration_ns: u64,
    /// Receive antenna count.
    pub receive_antennas: u16,
    /// Transmit antenna count.
    pub transmit_antennas: u16,
    /// Subcarrier count.
    pub subcarrier_count: u16,
    /// Source type marker.
    pub source_type: &'static str,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// Calibration failure event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationFailedPayload {
    /// Typed error code.
    pub code: String,
    /// Concise operator-safe message.
    pub message: String,
    /// Raw frame identifier when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_frame_id: Option<u64>,
    /// Sequence when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    /// Failed stage label when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_stage: Option<String>,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// Calibration service stopped payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationServiceStoppedPayload {
    /// Honesty label.
    pub data_classification: &'static str,
}

/// Optional RX/TX link selection query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct LinkQuery {
    /// Receive antenna index (defaults to `0`).
    pub rx: Option<u16>,
    /// Transmit antenna index (defaults to `0`).
    pub tx: Option<u16>,
}

impl LinkQuery {
    /// Resolved RX/TX pair with defaults `(0, 0)`.
    pub fn resolve(self) -> (u16, u16) {
        (self.rx.unwrap_or(0), self.tx.unwrap_or(0))
    }
}

/// `GET /api/v1/events/recent` query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct RecentEventsQuery {
    /// Maximum events to return (default 50, hard cap 100).
    pub limit: Option<usize>,
}

/// `GET /api/v1/dsp` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspSnapshot {
    /// Whether DSP is enabled in configuration.
    pub enabled: bool,
    /// Active profile identity when configured.
    pub profile_id: Option<String>,
    /// Active profile version when configured.
    pub profile_version: Option<u32>,
    /// DSP worker lifecycle label.
    pub worker_state: String,
    /// Operator-facing health derived from worker state.
    pub health: String,
    /// Configured temporal window size in frames.
    pub window_size_frames: usize,
    /// Configured hop size in frames.
    pub hop_size_frames: usize,
    /// Calibrated frames received by the DSP worker.
    pub calibrated_frames_received: u64,
    /// Successfully processed windows.
    pub windows_emitted: u64,
    /// Rejected or failed windows.
    pub windows_rejected: u64,
    /// Inclusive first sequence of the latest window, if any.
    pub latest_first_sequence: Option<u64>,
    /// Inclusive last sequence of the latest window, if any.
    pub latest_last_sequence: Option<u64>,
    /// Latest window processing timestamp (RFC 3339), if any.
    pub latest_window_timestamp: Option<String>,
    /// Effective sample rate derived from capture timestamps.
    pub effective_sample_rate_hz: Option<f64>,
    /// Latest timestamp jitter metric.
    pub timestamp_jitter: Option<f64>,
    /// Latest dominant non-DC frequency in hertz.
    pub latest_dominant_non_dc_hz: Option<f64>,
    /// Last processing duration in nanoseconds.
    pub last_duration_ns: Option<u64>,
    /// Average processing duration in nanoseconds.
    pub average_duration_ns: Option<u64>,
    /// Last warning summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_warning: Option<String>,
    /// Last error summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Configured kernel backend identifier (`rust` or `cpp`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_backend: Option<String>,
    /// Active kernel backend identifier when initialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_backend: Option<String>,
    /// Human-readable backend display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_display_name: Option<String>,
    /// Backend implementation version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_version: Option<String>,
    /// Native ABI version when the C++ backend is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_abi_version: Option<u32>,
    /// Whether the configured backend is compiled and available.
    pub backend_available: bool,
    /// Backend initialization status label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_init_status: Option<String>,
    /// Last backend-specific error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backend_error: Option<String>,
    /// Data source honesty label.
    pub data_classification: &'static str,
}

/// One RX–TX magnitude row for heatmap rendering.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibratedMagnitudeGridLink {
    /// Receive antenna index.
    pub rx: u16,
    /// Transmit antenna index.
    pub tx: u16,
    /// Per-subcarrier magnitude values for the link.
    pub magnitudes: Vec<f32>,
}

/// `GET /api/v1/signal/latest` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalLatestResponse {
    /// Whether a calibrated (and raw) frame snapshot is available.
    pub available: bool,
    /// Source classification label when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_classification: Option<&'static str>,
    /// Sensor identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor_id: Option<u64>,
    /// Frame sequence when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    /// Capture timestamp (RFC 3339) when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_timestamp: Option<String>,
    /// Selected receive antenna index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx: Option<u16>,
    /// Selected transmit antenna index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx: Option<u16>,
    /// Subcarrier indices for the selected frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcarrier_indices: Option<Vec<i16>>,
    /// Raw amplitudes for the selected link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_amplitudes: Option<Vec<f32>>,
    /// Calibrated amplitudes for the selected link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibrated_amplitudes: Option<Vec<f32>>,
    /// Raw wrapped phases (radians) for the selected link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_wrapped_phases: Option<Vec<f32>>,
    /// Calibrated phases (radians) for the selected link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibrated_phases: Option<Vec<f32>>,
    /// Raw frame identity retained by the calibrated frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_frame_id: Option<u64>,
    /// Calibration profile identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_profile_id: Option<String>,
    /// Calibration profile version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_profile_version: Option<u32>,
    /// Amplitude unit label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amplitude_units: Option<&'static str>,
    /// Phase unit label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_units: Option<&'static str>,
    /// Concise amplitude semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amplitude_semantics: Option<&'static str>,
    /// Concise phase semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_semantics: Option<&'static str>,
    /// Data honesty label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_classification: Option<&'static str>,
    /// Per-link calibrated magnitudes for heatmap rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibrated_magnitude_grid: Option<Vec<CalibratedMagnitudeGridLink>>,
}

impl SignalLatestResponse {
    /// Honest empty response before the first calibrated frame arrives.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            source_classification: None,
            sensor_id: None,
            sequence: None,
            capture_timestamp: None,
            rx: None,
            tx: None,
            subcarrier_indices: None,
            raw_amplitudes: None,
            calibrated_amplitudes: None,
            raw_wrapped_phases: None,
            calibrated_phases: None,
            raw_frame_id: None,
            calibration_profile_id: None,
            calibration_profile_version: None,
            amplitude_units: None,
            phase_units: None,
            amplitude_semantics: None,
            phase_semantics: None,
            data_classification: None,
            calibrated_magnitude_grid: None,
        }
    }
}

/// `GET /api/v1/dsp/latest` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspLatestResponse {
    /// Whether a DSP window result is available.
    pub available: bool,
    /// Selected receive antenna index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx: Option<u16>,
    /// Selected transmit antenna index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx: Option<u16>,
    /// Sensor identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor_id: Option<u64>,
    /// Window identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u64>,
    /// Inclusive first sequence in the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_sequence: Option<u64>,
    /// Inclusive last sequence in the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sequence: Option<u64>,
    /// First capture timestamp (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_capture_timestamp: Option<String>,
    /// Last capture timestamp (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_capture_timestamp: Option<String>,
    /// Processing completion timestamp (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,
    /// Effective sample rate from capture timestamps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_sample_rate_hz: Option<f64>,
    /// Timestamp jitter metric for the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_jitter: Option<f64>,
    /// Motion-energy relative time axis in seconds from window start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion_energy_time_secs: Option<Vec<f64>>,
    /// Motion-energy proxy values for the selected link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion_energy_values: Option<Vec<f64>>,
    /// One-sided spectrum frequency bins in hertz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectrum_frequencies_hz: Option<Vec<f64>>,
    /// One-sided spectrum power values aligned with frequency bins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectrum_power: Option<Vec<f64>>,
    /// Dominant non-DC frequency for the selected link or aggregate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_non_dc_hz: Option<f64>,
    /// Processing duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_duration_ns: Option<u64>,
    /// Non-fatal warnings from processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
    /// DSP profile identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsp_profile_id: Option<String>,
    /// DSP profile version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsp_profile_version: Option<u32>,
    /// Motion-energy semantic note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion_energy_semantics: Option<&'static str>,
    /// Spectral semantic note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectrum_semantics: Option<&'static str>,
    /// Sample-rate / timeline semantic note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline_semantics: Option<&'static str>,
    /// Data honesty label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_classification: Option<&'static str>,
}

impl DspLatestResponse {
    /// Honest empty response before the first completed DSP window.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            rx: None,
            tx: None,
            sensor_id: None,
            window_id: None,
            first_sequence: None,
            last_sequence: None,
            first_capture_timestamp: None,
            last_capture_timestamp: None,
            processed_at: None,
            effective_sample_rate_hz: None,
            timestamp_jitter: None,
            motion_energy_time_secs: None,
            motion_energy_values: None,
            spectrum_frequencies_hz: None,
            spectrum_power: None,
            dominant_non_dc_hz: None,
            processing_duration_ns: None,
            warnings: None,
            dsp_profile_id: None,
            dsp_profile_version: None,
            motion_energy_semantics: None,
            spectrum_semantics: None,
            timeline_semantics: None,
            data_classification: None,
        }
    }
}

/// `GET /api/v1/events/recent` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecentEventsResponse {
    /// Chronological event envelopes (oldest first).
    pub events: Vec<ApiEventEnvelope>,
}

/// DSP service started event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspServiceStartedPayload {
    /// Active DSP profile identity.
    pub profile_id: String,
    /// Active DSP profile version.
    pub profile_version: u32,
    /// Temporal window size in frames.
    pub window_size_frames: u32,
    /// Hop size in frames.
    pub hop_size_frames: u32,
    /// Selected kernel backend identifier.
    pub backend_id: String,
    /// Backend implementation version.
    pub backend_version: String,
    /// Native ABI version when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_abi_version: Option<u32>,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// CSI window assembled metadata payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CsiWindowAssembledPayload {
    /// Window identity.
    pub window_id: u64,
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Frame count in the window.
    pub frame_count: u32,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// DSP window processed metadata payload (no spectra arrays).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspWindowProcessedPayload {
    /// Window identity.
    pub window_id: u64,
    /// Source sensor identifier.
    pub sensor_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Frame count in the window.
    pub frame_count: u32,
    /// Active DSP profile identity.
    pub profile_id: String,
    /// Active DSP profile version.
    pub profile_version: u32,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Effective sample rate derived from capture timestamps.
    pub effective_sample_rate_hz: f64,
    /// Timestamp jitter metric for the window.
    pub timestamp_jitter: f64,
    /// Dominant non-DC frequency when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_non_dc_hz: Option<f64>,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// DSP processing failed event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspProcessingFailedPayload {
    /// Typed error code.
    pub code: String,
    /// Concise operator-safe message.
    pub message: String,
    /// Window identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u64>,
    /// Sensor identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor_id: Option<u64>,
    /// Inclusive first sequence when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_sequence: Option<u64>,
    /// Inclusive last sequence when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sequence: Option<u64>,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// DSP service idle / completed event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspServiceIdlePayload {
    /// Whether finite input completed cleanly.
    pub completed: bool,
    /// Honesty label.
    pub data_classification: &'static str,
}

/// DSP service stopped event payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DspServiceStoppedPayload {
    /// Honesty label.
    pub data_classification: &'static str,
}
