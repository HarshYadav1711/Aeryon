import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import {
  Dashboard,
  buildPipelineStages,
  formatBasename,
  formatEventType,
  formatStageName,
  presentChannelChangeState,
} from '../components/Dashboard'
import type {
  ApiEventEnvelope,
  CalibrationSnapshot,
  CsiReplaySnapshot,
  DspLatestResponse,
  DspSnapshot,
  FeatureLatest,
  FeatureSnapshot,
  ObservationLatest,
  PerceptionSnapshot,
  RuntimeSnapshot,
  SignalLatestResponse,
  SyntheticSensorSnapshot,
} from '../api/types'
import { MAX_EVENTS } from '../hooks/useDashboard'

const runtime: RuntimeSnapshot = {
  application_name: 'aeryon',
  application_version: '0.1.0',
  lifecycle_state: 'running',
  uptime_secs: 12.5,
  startup_timestamp: '2026-07-19T00:00:00.000Z',
  registered_plugin_count: 1,
  active_plugin_count: 1,
  frames_received: 3,
  last_frame_sequence: 2,
  last_frame_timestamp: '2026-07-19T00:00:01.000Z',
  synthetic_sensor_lifecycle: 'running',
  synthetic_source_enabled: true,
  csi_replay_lifecycle: null,
  csi_replay_enabled: false,
  active_source: 'synthetic',
}

const sensor: SyntheticSensorSnapshot = {
  enabled: true,
  lifecycle_state: 'running',
  configured_interval_ms: 100,
  samples_per_frame: 64,
  sample_rate_hz: 1000,
  configured_frequencies_hz: {
    primary_hz: 10,
    secondary_hz: 37,
  },
  frames_received: 3,
  last_sequence: 2,
  last_frame_timestamp: '2026-07-19T00:00:01.000Z',
  health: 'healthy',
}

const csiReplay: CsiReplaySnapshot = {
  enabled: true,
  lifecycle_state: 'running',
  health: 'healthy',
  source_type: 'csi_replay',
  data_classification: 'deterministic_development_fixture',
  fixture_path: 'datasets/fixtures/csi/synthetic_dev_v1.ndjson',
  loop_playback: false,
  frame_interval_ms: 100,
  maximum_frames: 0,
  frames_read: 4,
  frames_accepted: 4,
  frames_rejected: 0,
  latest_sequence: 3,
  latest_frame_timestamp: '2026-07-19T00:00:01.000Z',
  receive_antennas: 2,
  transmit_antennas: 1,
  subcarrier_count: 16,
  center_frequency_hz: 5_180_000_000,
  bandwidth_hz: 20_000_000,
  completion: 'active',
}

const calibration: CalibrationSnapshot = {
  enabled: true,
  worker_state: 'idle',
  profile_id: 'baseline-csi-v1',
  profile_version: 1,
  stages: ['phase_unwrap', 'linear_phase_detrend', 'rms_amplitude_normalize'],
  raw_frames_submitted: 0,
  frames_calibrated: 0,
  frames_failed: 0,
  latest_sequence: null,
  latest_calibrated_timestamp: null,
  last_duration_ns: null,
  average_duration_ns: null,
  queue_depth: 0,
  health: 'idle',
  data_classification: 'csi_replay_development_source',
}

const dspDisabled: DspSnapshot = {
  enabled: false,
  profile_id: null,
  profile_version: null,
  worker_state: 'disabled',
  health: 'disabled',
  window_size_frames: 32,
  hop_size_frames: 16,
  calibrated_frames_received: 0,
  windows_emitted: 0,
  windows_rejected: 0,
  latest_first_sequence: null,
  latest_last_sequence: null,
  latest_window_timestamp: null,
  effective_sample_rate_hz: null,
  timestamp_jitter: null,
  latest_dominant_non_dc_hz: null,
  last_duration_ns: null,
  average_duration_ns: null,
  configured_backend: 'rust',
  active_backend: null,
  backend_display_name: 'Rust reference backend',
  backend_version: '1.0.0',
  backend_abi_version: null,
  backend_available: true,
  backend_init_status: 'pending',
  last_backend_error: null,
  data_classification: 'csi_replay_development_source',
}

const dspActive: DspSnapshot = {
  ...dspDisabled,
  enabled: true,
  profile_id: 'baseline-dsp-v1',
  profile_version: 1,
  worker_state: 'running',
  health: 'running',
  calibrated_frames_received: 8,
  windows_emitted: 2,
  latest_first_sequence: 0,
  latest_last_sequence: 7,
  latest_dominant_non_dc_hz: 3.5,
  effective_sample_rate_hz: 100,
}

const featuresDisabled: FeatureSnapshot = {
  enabled: false,
  profile: { id: null, version: null },
  schema: { id: null, version: null, feature_count: 0 },
  worker_state: 'disabled',
  health: 'disabled',
  dsp_results_received: 0,
  feature_vectors_produced: 0,
  feature_failures: 0,
  latest_feature_vector_id: null,
  latest_first_sequence: null,
  latest_last_sequence: null,
  last_duration_ns: null,
  average_duration_ns: null,
  data_classification: 'csi_replay_development_source',
}

const featuresActive: FeatureSnapshot = {
  ...featuresDisabled,
  enabled: true,
  profile: { id: 'baseline-features-v1', version: 1 },
  schema: {
    id: 'csi-channel-features-v1',
    version: 1,
    feature_count: 25,
  },
  worker_state: 'running',
  health: 'running',
  feature_vectors_produced: 2,
  latest_feature_vector_id: 1,
  latest_first_sequence: 0,
  latest_last_sequence: 7,
}

const perceptionDisabled: PerceptionSnapshot = {
  enabled: false,
  profile: { id: null, version: null },
  worker_state: 'disabled',
  health: 'disabled',
  feature_vectors_received: 0,
  observations_produced: 0,
  observation_failures: 0,
  latest_observation_id: null,
  latest_observation_state: null,
  latest_activity_score: null,
  last_duration_ns: null,
  average_duration_ns: null,
  data_classification: 'csi_replay_development_source',
}

const perceptionActive: PerceptionSnapshot = {
  ...perceptionDisabled,
  enabled: true,
  profile: { id: 'channel-change-v1', version: 1 },
  worker_state: 'running',
  health: 'running',
  feature_vectors_received: 2,
  observations_produced: 2,
  latest_observation_id: 1,
  latest_observation_state: 'changing',
  latest_activity_score: 0.41,
}

const featuresLatest: FeatureLatest = {
  available: true,
  feature_vector_id: 1,
  feature_schema_id: 'csi-channel-features-v1',
  feature_profile_id: 'baseline-features-v1',
  dsp_profile_id: 'baseline-dsp-v1',
  dsp_backend_id: 'rust',
  dsp_backend_version: '1.0.0',
  calibration_profile_id: 'baseline-csi-v1',
  calibration_profile_version: 1,
  semantics_label:
    'Deterministic CSI channel descriptors; not presence, occupancy, or activity labels.',
  features: [
    {
      id: 'motion_energy_mean',
      value: 0.12,
      unit: 'normalized_complex_difference',
      description: 'Mean motion-energy proxy',
    },
    {
      id: 'motion_energy_rms',
      value: 0.18,
      unit: 'normalized_complex_difference',
      description: 'RMS motion-energy proxy',
    },
    {
      id: 'low_frequency_power_ratio',
      value: 0.4,
      unit: 'ratio',
      description: 'Low band ratio',
    },
    {
      id: 'middle_frequency_power_ratio',
      value: 0.35,
      unit: 'ratio',
      description: 'Middle band ratio',
    },
    {
      id: 'high_frequency_power_ratio',
      value: 0.25,
      unit: 'ratio',
      description: 'High band ratio',
    },
  ],
}

const observationLatest: ObservationLatest = {
  available: true,
  type: 'channel_change',
  observation_id: 1,
  state: 'changing',
  activity_score: 0.41,
  score_semantics: 'heuristic_channel_change_intensity_v1',
  disclaimer:
    'Signal-derived channel-change estimate. Not human-presence or activity recognition.',
  uncertainty: {
    threshold_margin: 0.08,
    normalized_threshold_margin: 0.24,
    timestamp_jitter: 0.01,
    warning_count: 0,
    supporting_frame_count: 8,
    valid_antenna_links: 2,
    reliability_score: 0.72,
    reliability_provenance: 'heuristic-threshold-reliability-v1',
  },
  evidence: {
    features: [
      {
        feature_id: 'motion_energy_rms',
        value: 0.18,
        normalized_contribution: 0.5,
      },
    ],
    activity_score: 0.41,
    stable_threshold: 0.22,
    high_change_threshold: 0.55,
    threshold_margin: 0.08,
    data_quality_warnings: [],
  },
}

const signalLatest: SignalLatestResponse = {
  available: true,
  source_classification: 'deterministic_development_fixture',
  sensor_id: 2,
  sequence: 7,
  capture_timestamp: '2026-07-19T00:00:01.000Z',
  rx: 0,
  tx: 0,
  subcarrier_indices: [0, 1, 2, 3],
  raw_amplitudes: [1, 2, 3, 4],
  calibrated_amplitudes: [0.9, 1.8, 2.7, 3.6],
  raw_wrapped_phases: [0.1, -0.2, 0.3, -0.4],
  calibrated_phases: [0.05, -0.1, 0.15, -0.2],
  amplitude_units: 'linear_magnitude',
  phase_units: 'radians',
  data_classification: 'deterministic_development_fixture',
  calibrated_magnitude_grid: [
    { rx: 0, tx: 0, magnitudes: [1, 2, 3, 4] },
    { rx: 1, tx: 0, magnitudes: [0.5, 1.5, 2.5, 3.5] },
  ],
}

const dspLatest: DspLatestResponse = {
  available: true,
  rx: 0,
  tx: 0,
  sensor_id: 2,
  window_id: 1,
  first_sequence: 0,
  last_sequence: 7,
  motion_energy_time_secs: [0, 0.01, 0.02, 0.03],
  motion_energy_values: [0.1, 0.2, 0.15, 0.25],
  spectrum_frequencies_hz: [0, 1, 2, 3, 4],
  spectrum_power: [10, 4, 8, 2, 1],
  dominant_non_dc_hz: 2,
  motion_energy_semantics: 'channel_change_proxy',
  spectrum_semantics: 'one_sided_power',
  data_classification: 'deterministic_development_fixture',
}

function baseProps(
  overrides: Partial<Parameters<typeof Dashboard>[0]> = {},
): Parameters<typeof Dashboard>[0] {
  return {
    connection: 'connected',
    runtime,
    sensor,
    csiReplay: null,
    calibration,
    dsp: dspDisabled,
    features: featuresDisabled,
    featuresLatest: { available: false },
    perception: perceptionDisabled,
    observationLatest: { available: false },
    signalLatest: { available: false },
    dspLatest: { available: false },
    events: [],
    framesReceived: 3,
    latestSequence: 2,
    latestFrameTimestamp: '2026-07-19T00:00:01.000Z',
    framesPerSecond: 10,
    restError: null,
    ...overrides,
  }
}

describe('Dashboard', () => {
  it('renders the compact status strip', () => {
    render(<Dashboard {...baseProps()} />)
    const strip = screen.getByTestId('status-strip')
    expect(strip).toBeInTheDocument()
    expect(within(strip).getByTestId('connection-state')).toHaveTextContent('Connected')
    expect(within(strip).getByTestId('active-source')).toHaveTextContent('synthetic')
    expect(within(strip).getByTestId('latest-sequence')).toHaveTextContent('2')
    expect(within(strip).getByTestId('frames-per-second')).toHaveTextContent('10.0')
  })

  it('shows honest idle FPS as an em dash', () => {
    render(<Dashboard {...baseProps({ framesPerSecond: null })} />)
    expect(screen.getByTestId('frames-per-second')).toHaveTextContent('—')
  })

  it('renders pipeline stages with correct states for active CSI replay', () => {
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            synthetic_source_enabled: false,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
            csi_replay_lifecycle: 'running',
          },
          csiReplay,
          calibration: { ...calibration, worker_state: 'running', health: 'healthy' },
          dsp: dspActive,
          features: featuresActive,
          perception: perceptionActive,
        })}
      />,
    )
    expect(screen.getByTestId('pipeline-state-csi_replay')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-validation')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-calibration')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-windowing')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-spectral_dsp')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-feature_extraction')).toHaveTextContent('Active')
    expect(screen.getByTestId('pipeline-state-channel_change_observation')).toHaveTextContent(
      'Active',
    )
    expect(screen.getByTestId('pipeline-stage-occupancy_ml')).toHaveAttribute(
      'data-state',
      'not_implemented',
    )
  })

  it('treats completed replay as Completed, not Failed', () => {
    const completedReplay: CsiReplaySnapshot = {
      ...csiReplay,
      lifecycle_state: 'stopped',
      completion: 'completed',
      frames_accepted: 16,
    }
    const completedCalibration: CalibrationSnapshot = {
      ...calibration,
      worker_state: 'stopped',
      health: 'idle',
      frames_calibrated: 16,
    }
    const completedDsp: DspSnapshot = {
      ...dspActive,
      worker_state: 'completed',
      health: 'completed',
      windows_emitted: 3,
    }
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            synthetic_source_enabled: false,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
            csi_replay_lifecycle: 'stopped',
          },
          csiReplay: completedReplay,
          calibration: completedCalibration,
          dsp: completedDsp,
          framesPerSecond: null,
        })}
      />,
    )
    expect(screen.getByTestId('replay-state')).toHaveTextContent('Completed')
    expect(screen.getByTestId('csi-completion')).toHaveTextContent('Completed')
    expect(screen.getByTestId('calibration-state')).toHaveTextContent('Completed')
    expect(screen.getByTestId('pipeline-state-csi_replay')).toHaveTextContent('Completed')
    expect(screen.getByTestId('pipeline-state-csi_replay')).not.toHaveTextContent('Failed')
    expect(screen.getByTestId('pipeline-stage-csi_replay')).toHaveAttribute(
      'data-state',
      'completed',
    )
    expect(screen.getByTestId('pipeline-stage-calibration')).toHaveAttribute(
      'data-state',
      'completed',
    )
    expect(screen.getByTestId('frames-per-second')).toHaveTextContent('—')
  })

  it('distinguishes Completed vs Stopped replay presentation', () => {
    const { rerender } = render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay: { ...csiReplay, completion: 'completed' },
        })}
      />,
    )
    expect(screen.getByTestId('replay-state')).toHaveTextContent('Completed')
    rerender(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay: { ...csiReplay, completion: 'stopped' },
        })}
      />,
    )
    expect(screen.getByTestId('replay-state')).toHaveTextContent('Stopped')
  })

  it('passes signal DTO values into charts', () => {
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay,
          dsp: dspActive,
          signalLatest,
          dspLatest,
        })}
      />,
    )
    expect(screen.getByTestId('amplitude-chart-svg')).toBeInTheDocument()
    expect(screen.getByTestId('amplitude-chart-series-raw')).toBeInTheDocument()
    expect(screen.getByTestId('amplitude-chart-series-calibrated')).toBeInTheDocument()
    expect(screen.getByTestId('phase-chart-series-raw')).toBeInTheDocument()
    expect(screen.getByTestId('motion-chart-series-motion_energy')).toBeInTheDocument()
    expect(screen.getByTestId('spectrum-chart-series-power')).toBeInTheDocument()
    expect(screen.getByTestId('spectrum-chart-annotation')).toHaveTextContent(/2\.000 Hz/)
    expect(screen.getByTestId('phase-chart-annotation')).toHaveTextContent(/affine detrend/i)
    expect(screen.getByTestId('motion-chart-annotation')).toHaveTextContent(/not human-motion/i)
  })

  it('renders chart empty states when latest snapshots are unavailable', () => {
    render(
      <Dashboard
        {...baseProps({
          signalLatest: { available: false },
          dspLatest: { available: false },
          dsp: { ...dspActive, windows_emitted: 0 },
        })}
      />,
    )
    expect(screen.getByTestId('amplitude-chart-empty')).toBeInTheDocument()
    expect(screen.getByTestId('phase-chart-empty')).toBeInTheDocument()
    expect(screen.getByTestId('amplitude-heatmap-empty')).toBeInTheDocument()
    expect(screen.getByTestId('motion-chart-empty')).toBeInTheDocument()
    expect(screen.getByTestId('spectrum-chart-empty')).toBeInTheDocument()
  })

  it('renders heatmap with expected dimensions from magnitude grid', () => {
    render(
      <Dashboard
        {...baseProps({
          signalLatest,
          dsp: dspActive,
        })}
      />,
    )
    const grid = screen.getByTestId('amplitude-heatmap-grid')
    expect(grid).toHaveAttribute('data-rows', '2')
    expect(grid).toHaveAttribute('data-cols', '4')
    expect(screen.getByTestId('amplitude-heatmap-cell-0-0')).toBeInTheDocument()
    expect(screen.getByTestId('amplitude-heatmap-cell-1-3')).toBeInTheDocument()
  })

  it('shows DSP disabled state on spectral charts', () => {
    render(
      <Dashboard
        {...baseProps({
          dsp: dspDisabled,
          signalLatest,
          dspLatest: { available: false },
        })}
      />,
    )
    expect(screen.getByTestId('dsp-disabled-banner')).toBeInTheDocument()
    expect(screen.getByTestId('dsp-state')).toHaveTextContent('Disabled')
    expect(screen.getByTestId('motion-chart-error')).toHaveTextContent(/DSP disabled/i)
    expect(screen.getByTestId('spectrum-chart-error')).toHaveTextContent(/DSP disabled/i)
    expect(screen.getByTestId('pipeline-stage-spectral_dsp')).toHaveAttribute(
      'data-state',
      'disabled',
    )
  })

  it('shows recent REST events in the timeline before later WS events', () => {
    const restThenWs: ApiEventEnvelope[] = [
      {
        version: 1,
        type: 'dsp_window_processed',
        timestamp: '2026-07-19T00:00:03.000Z',
        payload: { window_id: 1 },
      },
      {
        version: 1,
        type: 'csi_frame_calibrated',
        timestamp: '2026-07-19T00:00:02.000Z',
        payload: { sequence: 2 },
      },
      {
        version: 1,
        type: 'csi_frame',
        timestamp: '2026-07-19T00:00:01.000Z',
        payload: { sequence: 1 },
      },
    ]
    render(<Dashboard {...baseProps({ events: restThenWs })} />)
    const items = screen.getByTestId('event-timeline').querySelectorAll('li')
    expect(items).toHaveLength(3)
    expect(items[0]).toHaveTextContent('DSP window processed')
    expect(items[1]).toHaveTextContent('CSI frame calibrated')
    expect(items[2]).toHaveTextContent('CSI frame')
  })

  it('keeps event timeline bounded', () => {
    const events = Array.from({ length: MAX_EVENTS }, (_, index) => ({
      version: 1,
      type: 'sensor_frame',
      timestamp: `2026-07-19T00:00:${String(index).padStart(2, '0')}.000Z`,
      payload: { sequence: index, source_type: 'synthetic' },
    }))
    render(<Dashboard {...baseProps({ events })} />)
    expect(screen.getByTestId('event-timeline').querySelectorAll('li')).toHaveLength(MAX_EVENTS)
  })

  it('shortens long fixture paths without character breaking', () => {
    const longPath =
      'D:/Fun/Icarus/Aeryon/datasets/fixtures/csi/very_long_deterministic_fixture_name_for_layout_stress_test_v1.ndjson'
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            synthetic_source_enabled: false,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay: { ...csiReplay, fixture_path: longPath },
        })}
      />,
    )
    const fixture = screen.getByTestId('csi-fixture-path')
    expect(fixture).toHaveTextContent(
      'very_long_deterministic_fixture_name_for_layout_stress_test_v1.ndjson',
    )
    expect(fixture).not.toHaveTextContent('D:/Fun/Icarus')
    expect(fixture).toHaveAttribute('title', longPath)
    expect(fixture.className).toMatch(/path-basename/)
    expect(fixture.className).toMatch(/ellipsis/)
  })

  it('labels the source as deterministic fixture data', () => {
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay,
        })}
      />,
    )
    expect(screen.getByTestId('source-badge')).toHaveTextContent(
      'CSI Replay Development Source',
    )
    expect(screen.getByTestId('source-note')).toHaveTextContent(/Deterministic fixture data/i)
    expect(screen.getByTestId('source-note')).toHaveTextContent(/Not live WiFi RF sensing/i)
  })

  it('maps calibration stage identifiers to display names', () => {
    render(
      <Dashboard
        {...baseProps({
          runtime: {
            ...runtime,
            csi_replay_enabled: true,
            active_source: 'csi_replay',
          },
          csiReplay,
        })}
      />,
    )
    const stages = screen.getByTestId('calibration-stages')
    expect(stages).toHaveTextContent('Phase unwrap')
    expect(stages).toHaveTextContent('Linear phase detrend')
    expect(stages).toHaveTextContent('RMS amplitude normalization')
    expect(stages).not.toHaveTextContent('phase_unwrap')
  })

  it('exposes stage display name helpers', () => {
    expect(formatStageName('phase_unwrap')).toBe('Phase unwrap')
    expect(formatStageName('linear_phase_detrend')).toBe('Linear phase detrend')
    expect(formatStageName('rms_amplitude_normalize')).toBe('RMS amplitude normalization')
    expect(formatBasename('a/b/c.ndjson')).toBe('c.ndjson')
  })

  it('builds completed pipeline when finite workers finish after replay', () => {
    const stages = buildPipelineStages(
      { ...csiReplay, completion: 'completed', lifecycle_state: 'stopped', frames_accepted: 8 },
      { ...calibration, worker_state: 'stopped', frames_calibrated: 8 },
      { ...dspActive, worker_state: 'completed', windows_emitted: 2 },
      { ...featuresActive, worker_state: 'completed', feature_vectors_produced: 2 },
      { ...perceptionActive, worker_state: 'completed', observations_produced: 2 },
    )
    expect(stages.find((s) => s.id === 'csi_replay')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'validation')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'calibration')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'spectral_dsp')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'feature_extraction')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'channel_change_observation')?.state).toBe('completed')
    expect(stages.find((s) => s.id === 'occupancy_ml')?.state).toBe('not_implemented')
  })

  it('renders disconnected state', () => {
    render(<Dashboard {...baseProps({ connection: 'disconnected' })} />)
    expect(screen.getByTestId('connection-state')).toHaveTextContent('Disconnected')
  })

  it('updates frame counter and timeline from websocket-shaped events', () => {
    const events: ApiEventEnvelope[] = [
      {
        version: 1,
        type: 'sensor_frame',
        timestamp: '2026-07-19T00:00:02.000Z',
        payload: { sequence: 9, source_type: 'synthetic' },
      },
    ]
    render(
      <Dashboard
        {...baseProps({
          events,
          framesReceived: 9,
          latestSequence: 9,
        })}
      />,
    )
    expect(screen.getByTestId('frames-received')).toHaveTextContent('9')
    expect(screen.getByTestId('latest-sequence')).toHaveTextContent('9')
    expect(screen.getByTestId('event-timeline')).toHaveTextContent('Sensor frame')
  })

  it('renders feature vector panel and frequency band chart when available', () => {
    render(
      <Dashboard
        {...baseProps({
          features: featuresActive,
          featuresLatest,
          dsp: dspActive,
        })}
      />,
    )
    expect(screen.getByTestId('features-semantics')).toHaveTextContent(/not presence/i)
    expect(screen.getByTestId('features-selected')).toHaveTextContent('motion energy rms')
    expect(screen.getByTestId('frequency-band-chart-series-power_ratio')).toBeInTheDocument()
    expect(screen.getByTestId('features-table')).toBeInTheDocument()
    expect(screen.getByTestId('features-provenance')).toHaveTextContent('baseline-dsp-v1')
  })

  it('renders observation state, disclaimer, and no occupancy labels', () => {
    render(
      <Dashboard
        {...baseProps({
          perception: perceptionActive,
          observationLatest,
        })}
      />,
    )
    expect(screen.getByTestId('observation-disclaimer')).toHaveTextContent(
      'Signal-derived channel-change estimate. Not human-presence or activity recognition.',
    )
    expect(screen.getByTestId('observation-channel-state')).toHaveTextContent('Changing')
    expect(screen.getByTestId('observation-score-semantics')).toHaveTextContent(/heuristic/i)
    expect(screen.getByTestId('observation-latest')).not.toHaveTextContent(/occupancy/i)
    expect(screen.getByTestId('observation-latest')).not.toHaveTextContent(/human presence/i)
    expect(screen.getByTestId('observation-latest')).not.toHaveTextContent(/confidence/i)
  })

  it('shows disabled banners when features and observation are disabled', () => {
    render(
      <Dashboard
        {...baseProps({
          features: featuresDisabled,
          perception: perceptionDisabled,
          featuresLatest: { available: false },
          observationLatest: { available: false },
        })}
      />,
    )
    expect(screen.getByTestId('features-disabled-banner')).toBeInTheDocument()
    expect(screen.getByTestId('observation-disabled-banner')).toBeInTheDocument()
    expect(screen.getByTestId('features-latest-empty')).toBeInTheDocument()
    expect(screen.getByTestId('observation-latest-empty')).toBeInTheDocument()
    expect(screen.getByTestId('frequency-band-chart-error')).toHaveTextContent(
      /Feature extraction disabled/i,
    )
  })

  it('maps channel-change states and event labels for display', () => {
    expect(presentChannelChangeState('stable')).toBe('Stable')
    expect(presentChannelChangeState('highly_changing')).toBe('Highly changing')
    expect(formatEventType('feature_vector_produced')).toBe('Feature vector produced')
    expect(formatEventType('channel_change_observed')).toBe('Channel change observed')
  })
})
