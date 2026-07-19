/** TypeScript contracts matching the Aeryon local-development API DTOs. */

export interface ErrorBody {
  code: string
  message: string
}

export interface ErrorResponse {
  error: ErrorBody
}

export interface SyntheticHealthSummary {
  enabled: boolean
  lifecycle_state?: string | null
  health?: string | null
}

export interface CsiReplayHealthSummary {
  enabled: boolean
  lifecycle_state?: string | null
  health?: string | null
  completion?: string | null
}

export interface HealthResponse {
  status: string
  healthy: boolean
  uptime_secs: number
  timestamp: string
  event_consumer_running: boolean
  synthetic_sensor: SyntheticHealthSummary
  csi_replay: CsiReplayHealthSummary
}

export interface RuntimeSnapshot {
  application_name: string
  application_version: string
  lifecycle_state: string
  uptime_secs: number
  startup_timestamp: string
  registered_plugin_count: number
  active_plugin_count: number
  frames_received: number
  last_frame_sequence: number | null
  last_frame_timestamp: string | null
  synthetic_sensor_lifecycle: string | null
  synthetic_source_enabled: boolean
  csi_replay_lifecycle: string | null
  csi_replay_enabled: boolean
  active_source: string
}

export interface PluginSummary {
  id: string
  name: string
  version: string
  capabilities: string[]
  lifecycle_state: string
  health: string
}

export interface PluginsResponse {
  plugins: PluginSummary[]
}

export interface ConfiguredFrequencies {
  primary_hz: number
  secondary_hz: number
}

export interface SyntheticSensorSnapshot {
  enabled: boolean
  lifecycle_state: string | null
  configured_interval_ms: number
  samples_per_frame: number
  sample_rate_hz: number
  configured_frequencies_hz: ConfiguredFrequencies
  frames_received: number
  last_sequence: number | null
  last_frame_timestamp: string | null
  health: string | null
}

export interface CsiReplaySnapshot {
  enabled: boolean
  lifecycle_state: string | null
  health: string | null
  source_type: string
  data_classification: string
  fixture_path: string
  loop_playback: boolean
  frame_interval_ms: number
  maximum_frames: number
  frames_read: number
  frames_accepted: number
  frames_rejected: number
  latest_sequence: number | null
  latest_frame_timestamp: string | null
  receive_antennas: number | null
  transmit_antennas: number | null
  subcarrier_count: number | null
  center_frequency_hz: number | null
  bandwidth_hz: number | null
  completion: string
  last_error?: string | null
}

export interface ApiEventEnvelope {
  version: number
  type: string
  timestamp: string
  payload: Record<string, unknown>
}

export interface SensorFramePayload {
  sensor_id: number
  sequence: number
  frame_id: number
  capture_timestamp: string
  samples_per_frame: number
  source_type: string
}

export type ConnectionState =
  | 'loading'
  | 'connected'
  | 'disconnected'
  | 'reconnecting'
  | 'server_unavailable'
  | 'rest_error'
