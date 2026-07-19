//! Explicit API DTOs. Internal runtime structures are not serialized directly.

use serde::Serialize;

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
