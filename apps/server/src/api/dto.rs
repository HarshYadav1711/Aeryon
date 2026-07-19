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
    /// Configured samples per frame.
    pub samples_per_frame: usize,
    /// Source type marker.
    pub source_type: &'static str,
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
