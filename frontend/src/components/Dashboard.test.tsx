import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { Dashboard } from '../components/Dashboard'
import type {
  ApiEventEnvelope,
  CsiReplaySnapshot,
  RuntimeSnapshot,
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

function baseProps(
  overrides: Partial<Parameters<typeof Dashboard>[0]> = {},
): Parameters<typeof Dashboard>[0] {
  return {
    connection: 'connected',
    runtime,
    sensor,
    csiReplay: null,
    calibration: {
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
    },
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
  it('renders runtime snapshot values', () => {
    render(<Dashboard {...baseProps()} />)
    expect(screen.getByTestId('runtime-snapshot')).toHaveTextContent('running')
    expect(screen.getByTestId('runtime-snapshot')).toHaveTextContent('0.1.0')
    expect(screen.getByTestId('source-badge')).toHaveTextContent('Synthetic Development Source')
    expect(screen.getByText(/Deterministic Synthetic Sensor/i)).toBeInTheDocument()
  })

  it('renders no-frame connected state', () => {
    render(
      <Dashboard
        {...baseProps({
          framesReceived: 0,
          latestSequence: null,
          latestFrameTimestamp: null,
          events: [],
        })}
      />,
    )
    expect(screen.getByTestId('no-frame-state')).toHaveTextContent('no frame received yet')
  })

  it('renders disconnected state', () => {
    render(<Dashboard {...baseProps({ connection: 'disconnected' })} />)
    expect(screen.getByTestId('connection-state')).toHaveTextContent('Disconnected')
  })

  it('updates frame counter and timeline from websocket events', () => {
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
    expect(screen.getByTestId('event-timeline')).toHaveTextContent('sensor_frame')
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

  it('labels the source as synthetic', () => {
    render(<Dashboard {...baseProps()} />)
    expect(screen.getByText(/Not WiFi CSI/i)).toBeInTheDocument()
    expect(screen.getByTestId('source-badge')).toHaveTextContent(/Synthetic/i)
  })

  it('labels CSI replay development source honestly', () => {
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
          events: [
            {
              version: 1,
              type: 'csi_frame',
              timestamp: '2026-07-19T00:00:02.000Z',
              payload: { sequence: 3, source_type: 'csi_replay' },
            },
          ],
        })}
      />,
    )
    expect(screen.getByTestId('source-badge')).toHaveTextContent(
      'CSI Replay Development Source',
    )
    expect(screen.getByTestId('source-note')).toHaveTextContent(/Not live WiFi CSI/i)
    expect(screen.getByTestId('csi-replay-snapshot')).toHaveTextContent('2 × 1')
    expect(screen.getByTestId('csi-completion')).toHaveTextContent('active')
    expect(screen.getByTestId('event-timeline')).toHaveTextContent('csi_frame')
  })
})
