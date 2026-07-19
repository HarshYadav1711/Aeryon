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

export interface CalibrationSnapshot {
  enabled: boolean
  worker_state: string
  profile_id: string | null
  profile_version: number | null
  stages: string[]
  raw_frames_submitted: number
  frames_calibrated: number
  frames_failed: number
  latest_sequence: number | null
  latest_calibrated_timestamp: string | null
  last_duration_ns: number | null
  average_duration_ns: number | null
  last_warning?: string | null
  last_error?: string | null
  queue_depth: number
  health: string
  data_classification: string
}

export interface DspSnapshot {
  enabled: boolean
  profile_id: string | null
  profile_version: number | null
  worker_state: string
  health: string
  window_size_frames: number
  hop_size_frames: number
  calibrated_frames_received: number
  windows_emitted: number
  windows_rejected: number
  latest_first_sequence: number | null
  latest_last_sequence: number | null
  latest_window_timestamp: string | null
  effective_sample_rate_hz: number | null
  timestamp_jitter: number | null
  latest_dominant_non_dc_hz: number | null
  last_duration_ns: number | null
  average_duration_ns: number | null
  last_warning?: string | null
  last_error?: string | null
  configured_backend?: string | null
  active_backend?: string | null
  backend_display_name?: string | null
  backend_version?: string | null
  backend_abi_version?: number | null
  backend_available: boolean
  backend_init_status?: string | null
  last_backend_error?: string | null
  data_classification: string
}

export interface CalibratedMagnitudeGridLink {
  rx: number
  tx: number
  magnitudes: number[]
}

export interface SignalLatestResponse {
  available: boolean
  source_classification?: string | null
  sensor_id?: number | null
  sequence?: number | null
  capture_timestamp?: string | null
  rx?: number | null
  tx?: number | null
  subcarrier_indices?: number[] | null
  raw_amplitudes?: number[] | null
  calibrated_amplitudes?: number[] | null
  raw_wrapped_phases?: number[] | null
  calibrated_phases?: number[] | null
  raw_frame_id?: number | null
  calibration_profile_id?: string | null
  calibration_profile_version?: number | null
  amplitude_units?: string | null
  phase_units?: string | null
  amplitude_semantics?: string | null
  phase_semantics?: string | null
  data_classification?: string | null
  calibrated_magnitude_grid?: CalibratedMagnitudeGridLink[] | null
}

export interface DspLatestResponse {
  available: boolean
  rx?: number | null
  tx?: number | null
  sensor_id?: number | null
  window_id?: number | null
  first_sequence?: number | null
  last_sequence?: number | null
  first_capture_timestamp?: string | null
  last_capture_timestamp?: string | null
  processed_at?: string | null
  effective_sample_rate_hz?: number | null
  timestamp_jitter?: number | null
  motion_energy_time_secs?: number[] | null
  motion_energy_values?: number[] | null
  spectrum_frequencies_hz?: number[] | null
  spectrum_power?: number[] | null
  dominant_non_dc_hz?: number | null
  processing_duration_ns?: number | null
  warnings?: string[] | null
  dsp_profile_id?: string | null
  dsp_profile_version?: number | null
  motion_energy_semantics?: string | null
  spectrum_semantics?: string | null
  timeline_semantics?: string | null
  data_classification?: string | null
}

export interface FeatureProfileSummary {
  id: string | null
  version: number | null
}

export interface FeatureSchemaSummary {
  id: string | null
  version: number | null
  description?: string | null
  feature_count: number
}

export interface FeatureSnapshot {
  enabled: boolean
  profile: FeatureProfileSummary
  schema: FeatureSchemaSummary
  worker_state: string
  health: string
  dsp_results_received: number
  feature_vectors_produced: number
  feature_failures: number
  latest_feature_vector_id: number | null
  latest_first_sequence: number | null
  latest_last_sequence: number | null
  last_duration_ns: number | null
  average_duration_ns: number | null
  last_warning?: string | null
  last_error?: string | null
  data_classification: string
}

export interface FeatureValueEntry {
  id: string
  value: number
  unit: string
  description: string
}

export interface LinkFeaturesCompact {
  rx: number
  tx: number
  ordered_values: number[]
}

export interface FeatureLatest {
  available: boolean
  feature_vector_id?: number | null
  sensor_id?: number | null
  window_id?: number | null
  first_sequence?: number | null
  last_sequence?: number | null
  first_capture_timestamp?: string | null
  last_capture_timestamp?: string | null
  extracted_at?: string | null
  feature_schema_id?: string | null
  feature_schema_version?: number | null
  feature_profile_id?: string | null
  feature_profile_version?: number | null
  dsp_profile_id?: string | null
  dsp_profile_version?: number | null
  dsp_backend_id?: string | null
  dsp_backend_version?: string | null
  dsp_backend_abi_version?: number | null
  calibration_profile_id?: string | null
  calibration_profile_version?: number | null
  ordered_values?: number[] | null
  features?: FeatureValueEntry[] | null
  link_features?: LinkFeaturesCompact[] | null
  processing_duration_ns?: number | null
  warnings?: string[] | null
  semantics_label?: string | null
  data_classification?: string | null
}

export interface PerceptionProfileSummary {
  id: string | null
  version: number | null
}

export interface PerceptionSnapshot {
  enabled: boolean
  profile: PerceptionProfileSummary
  worker_state: string
  health: string
  feature_vectors_received: number
  observations_produced: number
  observation_failures: number
  latest_observation_id: number | null
  latest_observation_state: string | null
  latest_activity_score: number | null
  last_duration_ns: number | null
  average_duration_ns: number | null
  last_warning?: string | null
  last_error?: string | null
  data_classification: string
}

export interface ObservationFeatureEvidenceEntry {
  feature_id: string
  value: number
  normalized_contribution?: number | null
}

export interface ObservationEvidenceDto {
  features: ObservationFeatureEvidenceEntry[]
  activity_score: number
  stable_threshold: number
  high_change_threshold: number
  threshold_margin: number
  data_quality_warnings: string[]
}

export interface ObservationUncertaintyDto {
  threshold_margin: number
  normalized_threshold_margin: number
  timestamp_jitter: number
  warning_count: number
  supporting_frame_count: number
  valid_antenna_links: number
  reliability_score: number
  reliability_provenance: string
}

export interface ObservationProvenanceDto {
  threshold_profile_id: string
  threshold_profile_version: number
  feature_schema_id: string
  feature_schema_version: number
  feature_profile_id: string
  feature_profile_version: number
  dsp_profile_id: string
  dsp_profile_version: number
  dsp_backend_id: string
  dsp_backend_version: string
}

export interface ObservationLatest {
  available: boolean
  type?: 'channel_change' | null
  observation_id?: number | null
  sensor_id?: number | null
  feature_vector_id?: number | null
  window_id?: number | null
  first_sequence?: number | null
  last_sequence?: number | null
  first_capture_timestamp?: string | null
  last_capture_timestamp?: string | null
  created_at?: string | null
  state?: string | null
  activity_score?: number | null
  score_semantics?: string | null
  disclaimer?: string | null
  evidence?: ObservationEvidenceDto | null
  uncertainty?: ObservationUncertaintyDto | null
  provenance?: ObservationProvenanceDto | null
  warnings?: string[] | null
  data_classification?: string | null
}

export interface RecentEventsResponse {
  events: ApiEventEnvelope[]
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

export interface FeatureServiceStartedPayload {
  profile_id: string
  profile_version: number
  schema_id: string
  schema_version: number
  data_classification: string
}

export interface FeatureVectorProducedPayload {
  feature_vector_id: number
  sensor_id: number
  window_id: number
  first_sequence: number
  last_sequence: number
  schema_id: string
  schema_version: number
  profile_id: string
  profile_version: number
  feature_count: number
  link_count: number
  processing_duration_ns: number
  data_classification: string
}

export interface FeatureExtractionFailedPayload {
  code: string
  message: string
  window_id?: number | null
  sensor_id?: number | null
  first_sequence?: number | null
  last_sequence?: number | null
  data_classification: string
}

export interface FeatureServiceIdlePayload {
  completed: boolean
  data_classification: string
}

export interface FeatureServiceStoppedPayload {
  data_classification: string
}

export interface PerceptionServiceStartedPayload {
  profile_id: string
  profile_version: number
  data_classification: string
}

export interface ChannelChangeObservedPayload {
  observation_id: number
  sensor_id: number
  feature_vector_id: number
  first_sequence: number
  last_sequence: number
  state: string
  activity_score: number
  threshold_margin: number
  profile_id: string
  profile_version: number
  warning_count: number
  data_classification: string
}

export interface ObservationFailedPayload {
  code: string
  message: string
  feature_vector_id?: number | null
  sensor_id?: number | null
  first_sequence?: number | null
  last_sequence?: number | null
  data_classification: string
}

export interface PerceptionServiceIdlePayload {
  completed: boolean
  data_classification: string
}

export interface PerceptionServiceStoppedPayload {
  data_classification: string
}

export type ConnectionState =
  | 'loading'
  | 'connected'
  | 'disconnected'
  | 'reconnecting'
  | 'server_unavailable'
  | 'rest_error'
