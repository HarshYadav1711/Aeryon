import type {
  ApiEventEnvelope,
  CalibrationSnapshot,
  ConnectionState,
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
import { Heatmap } from './charts/Heatmap'
import { LineChart } from './charts/LineChart'

export type DashboardProps = {
  connection: ConnectionState
  runtime: RuntimeSnapshot | null
  sensor: SyntheticSensorSnapshot | null
  csiReplay: CsiReplaySnapshot | null
  calibration: CalibrationSnapshot | null
  dsp: DspSnapshot | null
  features: FeatureSnapshot | null
  featuresLatest: FeatureLatest | null
  perception: PerceptionSnapshot | null
  observationLatest: ObservationLatest | null
  signalLatest: SignalLatestResponse | null
  dspLatest: DspLatestResponse | null
  events: ApiEventEnvelope[]
  framesReceived: number
  latestSequence: number | null
  latestFrameTimestamp: string | null
  framesPerSecond: number | null
  restError: string | null
}

export type PipelineStageState =
  | 'active'
  | 'completed'
  | 'idle'
  | 'disabled'
  | 'degraded'
  | 'failed'
  | 'not_implemented'

export type PipelineStage = {
  id: string
  label: string
  state: PipelineStageState
}

const STAGE_DISPLAY: Record<string, string> = {
  phase_unwrap: 'Phase unwrap',
  linear_phase_detrend: 'Linear phase detrend',
  rms_amplitude_normalize: 'RMS amplitude normalization',
}

const EVENT_TYPE_LABELS: Record<string, string> = {
  sensor_frame: 'Sensor frame',
  csi_frame: 'CSI frame',
  csi_frame_calibrated: 'CSI frame calibrated',
  calibration_started: 'Calibration started',
  calibration_failed: 'Calibration failed',
  calibration_service_stopped: 'Calibration stopped',
  dsp_service_started: 'DSP service started',
  csi_window_assembled: 'CSI window assembled',
  dsp_window_processed: 'DSP window processed',
  dsp_processing_failed: 'DSP processing failed',
  dsp_service_idle: 'DSP service idle',
  dsp_service_completed: 'DSP service completed',
  dsp_service_stopped: 'DSP service stopped',
  feature_service_started: 'Feature service started',
  feature_vector_produced: 'Feature vector produced',
  feature_extraction_failed: 'Feature extraction failed',
  feature_service_idle: 'Feature service idle',
  feature_service_completed: 'Feature service completed',
  feature_service_stopped: 'Feature service stopped',
  perception_service_started: 'Perception service started',
  channel_change_observed: 'Channel change observed',
  observation_failed: 'Observation failed',
  perception_service_idle: 'Perception service idle',
  perception_service_completed: 'Perception service completed',
  perception_service_stopped: 'Perception service stopped',
}

const SELECTED_FEATURE_IDS = [
  'motion_energy_rms',
  'motion_energy_p95',
  'low_frequency_power_ratio',
  'middle_frequency_power_ratio',
  'high_frequency_power_ratio',
] as const

const OBSERVATION_DISCLAIMER =
  'Signal-derived channel-change estimate. Not human-presence or activity recognition.'

export function formatStageName(stage: string): string {
  return STAGE_DISPLAY[stage] ?? stage.replaceAll('_', ' ')
}

export function formatEventType(eventType: string): string {
  return EVENT_TYPE_LABELS[eventType] ?? eventType.replaceAll('_', ' ')
}

export function formatBasename(path: string): string {
  const normalized = path.replaceAll('\\', '/')
  const parts = normalized.split('/')
  return parts[parts.length - 1] || path
}

export function formatTimestamp(value: string | null | undefined): string {
  if (!value) {
    return '—'
  }
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value.replace('T', ' ').replace(/\.\d{3}Z$/, 'Z')
  }
  return date.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function formatUptime(seconds: number | undefined): string {
  if (seconds === undefined) {
    return '—'
  }
  const total = Math.floor(seconds)
  const hrs = Math.floor(total / 3600)
  const mins = Math.floor((total % 3600) / 60)
  const secs = total % 60
  if (hrs > 0) {
    return `${hrs}h ${mins}m ${secs}s`
  }
  if (mins > 0) {
    return `${mins}m ${secs}s`
  }
  return `${secs}s`
}

function connectionLabel(connection: ConnectionState): string {
  switch (connection) {
    case 'loading':
      return 'Loading'
    case 'connected':
      return 'Connected'
    case 'disconnected':
      return 'Disconnected'
    case 'reconnecting':
      return 'Reconnecting'
    case 'server_unavailable':
      return 'Server unavailable'
    case 'rest_error':
      return 'REST request failed'
  }
}

function sourceBadge(runtime: RuntimeSnapshot | null): string {
  if (runtime?.active_source === 'csi_replay' || runtime?.csi_replay_enabled) {
    return 'CSI Replay Development Source'
  }
  return 'Synthetic Development Source'
}

function presentReplayState(csiReplay: CsiReplaySnapshot | null): string {
  if (!csiReplay) {
    return '—'
  }
  if (!csiReplay.enabled) {
    return 'Disabled'
  }
  switch (csiReplay.completion) {
    case 'completed':
      return 'Completed'
    case 'failed':
      return 'Failed'
    case 'active':
      return 'Active'
    case 'stopped':
      return 'Stopped'
    case 'idle':
      return 'Idle'
    default:
      return csiReplay.completion
  }
}

function presentCalibrationState(
  calibration: CalibrationSnapshot | null,
  replayCompleted: boolean,
): string {
  if (!calibration) {
    return '—'
  }
  if (!calibration.enabled) {
    return 'Disabled'
  }
  const worker = calibration.worker_state
  if (worker === 'stopped' && replayCompleted) {
    return 'Completed'
  }
  if (worker === 'running') {
    return 'Active'
  }
  if (worker === 'completed') {
    return 'Completed'
  }
  if (worker === 'failed') {
    return 'Failed'
  }
  if (worker === 'idle') {
    return 'Idle'
  }
  if (worker === 'disabled') {
    return 'Disabled'
  }
  if (worker === 'stopped') {
    return 'Stopped'
  }
  return worker
}

function presentDspState(dsp: DspSnapshot | null): string {
  if (!dsp) {
    return '—'
  }
  if (!dsp.enabled) {
    return 'Disabled'
  }
  switch (dsp.worker_state) {
    case 'running':
      return 'Active'
    case 'completed':
      return 'Completed'
    case 'idle':
      return 'Idle'
    case 'disabled':
      return 'Disabled'
    case 'failed':
      return 'Failed'
    case 'stopped':
      return 'Stopped'
    default:
      return dsp.health === 'degraded' ? 'Degraded' : dsp.worker_state
  }
}

function presentFeatureState(features: FeatureSnapshot | null): string {
  if (!features) {
    return '—'
  }
  if (!features.enabled) {
    return 'Disabled'
  }
  switch (features.worker_state) {
    case 'running':
      return 'Active'
    case 'completed':
      return 'Completed'
    case 'idle':
      return 'Idle'
    case 'disabled':
      return 'Disabled'
    case 'failed':
      return 'Failed'
    case 'stopped':
      return 'Stopped'
    default:
      return features.health === 'degraded' ? 'Degraded' : features.worker_state
  }
}

function presentObservationState(perception: PerceptionSnapshot | null): string {
  if (!perception) {
    return '—'
  }
  if (!perception.enabled) {
    return 'Disabled'
  }
  switch (perception.worker_state) {
    case 'running':
      return 'Active'
    case 'completed':
      return 'Completed'
    case 'idle':
      return 'Idle'
    case 'disabled':
      return 'Disabled'
    case 'failed':
      return 'Failed'
    case 'stopped':
      return 'Stopped'
    default:
      return perception.health === 'degraded' ? 'Degraded' : perception.worker_state
  }
}

export function presentChannelChangeState(state: string | null | undefined): string {
  switch (state) {
    case 'stable':
      return 'Stable'
    case 'changing':
      return 'Changing'
    case 'highly_changing':
      return 'Highly changing'
    case 'indeterminate':
      return 'Indeterminate'
    default:
      return state ?? '—'
  }
}

function featureValueById(
  featuresLatest: FeatureLatest | null,
  featureId: string,
): number | null {
  const entry = featuresLatest?.features?.find((feature) => feature.id === featureId)
  return entry && Number.isFinite(entry.value) ? entry.value : null
}

function mapWorkerStage(
  enabled: boolean,
  worker: string | null | undefined,
  health: string | null | undefined,
  replayCompleted: boolean,
): PipelineStageState {
  if (!enabled) {
    return 'disabled'
  }
  if (health === 'degraded') {
    return 'degraded'
  }
  switch (worker) {
    case 'running':
      return 'active'
    case 'completed':
      return 'completed'
    case 'failed':
      return 'failed'
    case 'disabled':
      return 'disabled'
    case 'idle':
      return 'idle'
    case 'stopped':
      return replayCompleted ? 'completed' : 'idle'
    default:
      return 'idle'
  }
}

export function buildPipelineStages(
  csiReplay: CsiReplaySnapshot | null,
  calibration: CalibrationSnapshot | null,
  dsp: DspSnapshot | null,
  features: FeatureSnapshot | null,
  perception: PerceptionSnapshot | null,
): PipelineStage[] {
  const replayCompleted = csiReplay?.completion === 'completed'

  let replayState: PipelineStageState = 'idle'
  if (!csiReplay || !csiReplay.enabled) {
    replayState = csiReplay?.enabled === false ? 'disabled' : 'idle'
  } else if (csiReplay.completion === 'completed') {
    replayState = 'completed'
  } else if (csiReplay.completion === 'failed') {
    replayState = 'failed'
  } else if (csiReplay.completion === 'active') {
    replayState = 'active'
  } else if (csiReplay.health === 'degraded') {
    replayState = 'degraded'
  } else if (csiReplay.completion === 'stopped') {
    replayState = 'idle'
  } else {
    replayState = 'idle'
  }

  let validationState: PipelineStageState = 'idle'
  if (!csiReplay || !csiReplay.enabled) {
    validationState = 'disabled'
  } else if (csiReplay.completion === 'failed' || (csiReplay.frames_rejected > 0 && csiReplay.health === 'unhealthy')) {
    validationState = 'failed'
  } else if (csiReplay.health === 'degraded') {
    validationState = 'degraded'
  } else if (csiReplay.completion === 'completed' && csiReplay.frames_accepted > 0) {
    validationState = 'completed'
  } else if (csiReplay.completion === 'active' && csiReplay.frames_accepted > 0) {
    validationState = 'active'
  } else if (csiReplay.frames_accepted > 0) {
    validationState = replayCompleted ? 'completed' : 'active'
  }

  const calibrationState = mapWorkerStage(
    Boolean(calibration?.enabled),
    calibration?.worker_state,
    calibration?.health,
    Boolean(replayCompleted),
  )

  let windowingState: PipelineStageState = 'idle'
  if (!dsp || !dsp.enabled) {
    windowingState = 'disabled'
  } else if (dsp.health === 'degraded') {
    windowingState = 'degraded'
  } else if (dsp.worker_state === 'failed') {
    windowingState = 'failed'
  } else if (dsp.worker_state === 'running') {
    windowingState = 'active'
  } else if (
    dsp.worker_state === 'completed' ||
    (replayCompleted && dsp.windows_emitted > 0)
  ) {
    windowingState = 'completed'
  } else if (dsp.windows_emitted > 0) {
    windowingState = 'active'
  } else if (dsp.worker_state === 'idle' || dsp.worker_state === 'stopped') {
    windowingState = replayCompleted && dsp.windows_emitted > 0 ? 'completed' : 'idle'
  }

  const spectralState = mapWorkerStage(
    Boolean(dsp?.enabled),
    dsp?.worker_state,
    dsp?.health,
    Boolean(replayCompleted) && (dsp?.windows_emitted ?? 0) > 0,
  )

  const featureState = mapWorkerStage(
    Boolean(features?.enabled),
    features?.worker_state,
    features?.health,
    Boolean(replayCompleted) && (features?.feature_vectors_produced ?? 0) > 0,
  )

  const observationState = mapWorkerStage(
    Boolean(perception?.enabled),
    perception?.worker_state,
    perception?.health,
    Boolean(replayCompleted) && (perception?.observations_produced ?? 0) > 0,
  )

  return [
    { id: 'csi_replay', label: 'CSI Replay', state: replayState },
    { id: 'validation', label: 'Validation', state: validationState },
    { id: 'calibration', label: 'Calibration', state: calibrationState },
    { id: 'windowing', label: 'Windowing', state: windowingState },
    { id: 'spectral_dsp', label: 'Spectral DSP', state: spectralState },
    { id: 'feature_extraction', label: 'Feature Extraction', state: featureState },
    {
      id: 'channel_change_observation',
      label: 'Channel-Change Observation',
      state: observationState,
    },
    {
      id: 'occupancy_ml',
      label: 'Occupancy/Activity ML',
      state: 'not_implemented',
    },
  ]
}

function stageLabel(state: PipelineStageState): string {
  switch (state) {
    case 'active':
      return 'Active'
    case 'completed':
      return 'Completed'
    case 'idle':
      return 'Idle'
    case 'disabled':
      return 'Disabled'
    case 'degraded':
      return 'Degraded'
    case 'failed':
      return 'Failed'
    case 'not_implemented':
      return 'Not implemented'
  }
}

function indexAxis(length: number, indices?: number[] | null): number[] {
  if (indices && indices.length === length) {
    return indices.map((v) => Number(v))
  }
  return Array.from({ length }, (_, i) => i)
}

function heatmapFromGrid(
  grid: SignalLatestResponse['calibrated_magnitude_grid'],
): { values: number[][]; rowLabels: string[]; colLabels: string[] } {
  if (!grid || grid.length === 0) {
    return { values: [], rowLabels: [], colLabels: [] }
  }
  const values = grid.map((link) => link.magnitudes)
  const rowLabels = grid.map((link) => `RX${link.rx}/TX${link.tx}`)
  const cols = Math.max(...values.map((row) => row.length), 0)
  const colLabels = Array.from({ length: cols }, (_, i) => String(i))
  return { values, rowLabels, colLabels }
}

export function Dashboard({
  connection,
  runtime,
  sensor,
  csiReplay,
  calibration,
  dsp,
  features,
  featuresLatest,
  perception,
  observationLatest,
  signalLatest,
  dspLatest,
  events,
  framesReceived,
  latestSequence,
  latestFrameTimestamp,
  framesPerSecond,
  restError,
}: DashboardProps) {
  const csiActive = runtime?.active_source === 'csi_replay' || Boolean(runtime?.csi_replay_enabled)
  const replayCompleted = csiReplay?.completion === 'completed'
  const pipeline = buildPipelineStages(csiReplay, calibration, dsp, features, perception)
  const dspDisabled = dsp !== null && !dsp.enabled
  const featuresDisabled = features !== null && !features.enabled
  const perceptionDisabled = perception !== null && !perception.enabled
  const signalLoading = signalLatest === null && connection === 'loading'
  const dspLatestLoading = dspLatest === null && connection === 'loading'
  const featuresLatestLoading = featuresLatest === null && connection === 'loading'

  const subcarriers = indexAxis(
    Math.max(
      signalLatest?.raw_amplitudes?.length ?? 0,
      signalLatest?.calibrated_amplitudes?.length ?? 0,
    ),
    signalLatest?.subcarrier_indices,
  )
  const hasAmplitude =
    Boolean(signalLatest?.available) &&
    ((signalLatest?.raw_amplitudes?.length ?? 0) > 0 ||
      (signalLatest?.calibrated_amplitudes?.length ?? 0) > 0)
  const hasPhase =
    Boolean(signalLatest?.available) &&
    ((signalLatest?.raw_wrapped_phases?.length ?? 0) > 0 ||
      (signalLatest?.calibrated_phases?.length ?? 0) > 0)
  const heatmap = heatmapFromGrid(signalLatest?.calibrated_magnitude_grid)
  const hasHeatmap = Boolean(signalLatest?.available) && heatmap.values.length > 0

  const motionX = dspLatest?.motion_energy_time_secs ?? []
  const motionY = dspLatest?.motion_energy_values ?? []
  const hasMotion =
    Boolean(dspLatest?.available) && motionX.length > 0 && motionY.length > 0

  const spectrumX = dspLatest?.spectrum_frequencies_hz ?? []
  const spectrumY = dspLatest?.spectrum_power ?? []
  const hasSpectrum =
    Boolean(dspLatest?.available) && spectrumX.length > 0 && spectrumY.length > 0
  const dominantHz =
    dspLatest?.dominant_non_dc_hz ?? dsp?.latest_dominant_non_dc_hz ?? null

  const selectedFeatures =
    featuresLatest?.features?.filter((feature) =>
      SELECTED_FEATURE_IDS.includes(feature.id as (typeof SELECTED_FEATURE_IDS)[number]),
    ) ?? []

  const lowBand = featureValueById(featuresLatest, 'low_frequency_power_ratio')
  const middleBand = featureValueById(featuresLatest, 'middle_frequency_power_ratio')
  const highBand = featureValueById(featuresLatest, 'high_frequency_power_ratio')
  const hasFrequencyBands =
    lowBand !== null && middleBand !== null && highBand !== null && featuresLatest?.available
  const frequencyBandX = [0, 1, 2]
  const frequencyBandY = [lowBand ?? 0, middleBand ?? 0, highBand ?? 0]

  const stagesText =
    calibration && calibration.stages.length > 0
      ? calibration.stages.map(formatStageName).join(' → ')
      : '—'

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <div>
          <h1>Aeryon</h1>
          <p className="tagline">Transforming Signals into Understanding</p>
        </div>
        <div className="source-badge" data-testid="source-badge">
          {sourceBadge(runtime)}
        </div>
      </header>

      <p className="source-note" data-testid="source-note">
        Deterministic fixture data. Not live WiFi RF sensing.
      </p>

      <section className="status-strip" aria-label="Status strip" data-testid="status-strip">
        <div className="status-cell">
          <span className="status-label">Connection</span>
          <span className="status-value" data-testid="connection-state" data-state={connection}>
            {connectionLabel(connection)}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Runtime</span>
          <span className="status-value ellipsis" data-testid="runtime-state">
            {runtime ? `${runtime.lifecycle_state} · ${formatUptime(runtime.uptime_secs)}` : '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Active source</span>
          <span className="status-value" data-testid="active-source">
            {runtime?.active_source ?? '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Replay</span>
          <span className="status-value" data-testid="replay-state">
            {presentReplayState(csiReplay)}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Calibration</span>
          <span className="status-value" data-testid="calibration-state">
            {presentCalibrationState(calibration, Boolean(replayCompleted))}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">DSP</span>
          <span className="status-value" data-testid="dsp-state">
            {presentDspState(dsp)}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Features</span>
          <span className="status-value" data-testid="features-state">
            {presentFeatureState(features)}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Observation</span>
          <span className="status-value" data-testid="observation-state">
            {presentObservationState(perception)}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">DSP backend</span>
          <span className="status-value" data-testid="dsp-backend">
            {dsp?.backend_display_name
              ?? (dsp?.active_backend === 'cpp'
                ? 'C++ native backend'
                : dsp?.active_backend === 'rust'
                  ? 'Rust reference backend'
                  : '—')}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Backend version</span>
          <span className="status-value" data-testid="dsp-backend-version">
            {dsp?.backend_version ?? '—'}
            {dsp?.backend_abi_version != null ? ` (ABI ${dsp.backend_abi_version})` : ''}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Backend health</span>
          <span className="status-value" data-testid="dsp-backend-health">
            {!dsp?.enabled
              ? '—'
              : dsp.backend_available
                ? (dsp.backend_init_status ?? 'ok')
                : (dsp.last_backend_error ?? 'unavailable')}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Latest sequence</span>
          <span className="status-value" data-testid="latest-sequence">
            {latestSequence ?? '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Frames accepted</span>
          <span className="status-value" data-testid="frames-accepted">
            {csiActive ? (csiReplay?.frames_accepted ?? '—') : framesReceived}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Windows processed</span>
          <span className="status-value" data-testid="windows-processed">
            {dsp?.windows_emitted ?? '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Feature vectors</span>
          <span className="status-value" data-testid="feature-vectors-produced">
            {features?.feature_vectors_produced ?? '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">Observations</span>
          <span className="status-value" data-testid="observations-produced">
            {perception?.observations_produced ?? '—'}
          </span>
        </div>
        <div className="status-cell">
          <span className="status-label">FPS</span>
          <span className="status-value" data-testid="frames-per-second">
            {framesPerSecond === null || Number.isNaN(framesPerSecond)
              ? '—'
              : framesPerSecond.toFixed(1)}
          </span>
        </div>
        {restError ? (
          <div className="status-cell status-cell-wide">
            <span className="status-label">REST error</span>
            <span className="status-value" data-testid="rest-error">
              {restError}
            </span>
          </div>
        ) : null}
      </section>

      {csiActive && csiReplay ? (
        <section className="panel panel-compact" aria-labelledby="csi-meta-heading">
          <h2 id="csi-meta-heading">CSI replay</h2>
          <dl className="metrics metrics-compact" data-testid="csi-replay-snapshot">
            <div>
              <dt>Fixture</dt>
              <dd
                className="path-basename ellipsis"
                data-testid="csi-fixture-path"
                title={csiReplay.fixture_path}
              >
                {formatBasename(csiReplay.fixture_path)}
              </dd>
            </div>
            <div>
              <dt>Completion</dt>
              <dd data-testid="csi-completion">{presentReplayState(csiReplay)}</dd>
            </div>
            <div>
              <dt>Antennas</dt>
              <dd>
                {csiReplay.receive_antennas ?? '—'} × {csiReplay.transmit_antennas ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Subcarriers</dt>
              <dd>{csiReplay.subcarrier_count ?? '—'}</dd>
            </div>
            <div>
              <dt>Latest frame</dt>
              <dd>{formatTimestamp(csiReplay.latest_frame_timestamp)}</dd>
            </div>
            <div>
              <dt>Classification</dt>
              <dd className="ellipsis" title={csiReplay.data_classification}>
                {csiReplay.data_classification}
              </dd>
            </div>
          </dl>
        </section>
      ) : null}

      {!csiActive && sensor ? (
        <section className="panel panel-compact" aria-labelledby="sensor-meta-heading">
          <h2 id="sensor-meta-heading">Synthetic sensor</h2>
          <dl className="metrics metrics-compact" data-testid="sensor-snapshot">
            <div>
              <dt>Sample rate</dt>
              <dd>{sensor.sample_rate_hz} Hz</dd>
            </div>
            <div>
              <dt>Samples / frame</dt>
              <dd>{sensor.samples_per_frame}</dd>
            </div>
            <div>
              <dt>Health</dt>
              <dd>{sensor.health ?? '—'}</dd>
            </div>
          </dl>
        </section>
      ) : null}

      {calibration ? (
        <section className="panel panel-compact" aria-labelledby="cal-meta-heading">
          <h2 id="cal-meta-heading">Calibration profile</h2>
          <dl className="metrics metrics-compact" data-testid="calibration-snapshot">
            <div>
              <dt>Profile</dt>
              <dd className="ellipsis" data-testid="calibration-profile" title={calibration.profile_id ?? undefined}>
                {calibration.profile_id ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Stages</dt>
              <dd data-testid="calibration-stages">{stagesText}</dd>
            </div>
            <div>
              <dt>Frames calibrated</dt>
              <dd data-testid="calibration-success">{calibration.frames_calibrated}</dd>
            </div>
            <div>
              <dt>Worker</dt>
              <dd data-testid="calibration-worker">{calibration.worker_state}</dd>
            </div>
          </dl>
        </section>
      ) : null}

      <section className="panel" aria-labelledby="pipeline-heading">
        <h2 id="pipeline-heading">Pipeline progress</h2>
        <ol className="pipeline" data-testid="pipeline">
          {pipeline.map((stage) => (
            <li
              key={stage.id}
              className="pipeline-stage"
              data-testid={`pipeline-stage-${stage.id}`}
              data-state={stage.state}
            >
              <span className="pipeline-label">{stage.label}</span>
              <span className="pipeline-state" data-testid={`pipeline-state-${stage.id}`}>
                {stageLabel(stage.state)}
              </span>
            </li>
          ))}
        </ol>
        <p className="muted pipeline-note" data-testid="occupancy-note">
          Occupancy/activity ML is not implemented in this milestone.
        </p>
      </section>

      {features ? (
        <section className="panel panel-compact" aria-labelledby="features-meta-heading">
          <h2 id="features-meta-heading">Feature extraction</h2>
          <dl className="metrics metrics-compact" data-testid="features-snapshot">
            <div>
              <dt>Profile</dt>
              <dd className="ellipsis" data-testid="features-profile">
                {features.profile.id ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Schema</dt>
              <dd className="ellipsis" data-testid="features-schema">
                {features.schema.id ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Vectors produced</dt>
              <dd data-testid="features-produced">{features.feature_vectors_produced}</dd>
            </div>
            <div>
              <dt>Worker</dt>
              <dd data-testid="features-worker">{features.worker_state}</dd>
            </div>
          </dl>
        </section>
      ) : null}

      <section className="panel" aria-labelledby="features-panel-heading">
        <h2 id="features-panel-heading">Feature vector</h2>
        {featuresDisabled ? (
          <p className="muted" data-testid="features-disabled-banner">
            Feature extraction is disabled.
          </p>
        ) : null}
        {!featuresLatest?.available ? (
          <p className="muted" data-testid="features-latest-empty">
            No feature vector available yet.
          </p>
        ) : (
          <>
            <p className="muted feature-semantics" data-testid="features-semantics">
              {featuresLatest.semantics_label ??
                'Deterministic CSI channel descriptors; not presence, occupancy, or activity labels.'}
            </p>
            <dl className="metrics metrics-compact" data-testid="features-selected">
              {selectedFeatures.map((feature) => (
                <div key={feature.id}>
                  <dt>{feature.id.replaceAll('_', ' ')}</dt>
                  <dd>{feature.value.toFixed(4)}</dd>
                </div>
              ))}
            </dl>
            <details className="feature-table-details" data-testid="features-table-details">
              <summary>All features ({featuresLatest.features?.length ?? 0})</summary>
              <table className="feature-table" data-testid="features-table">
                <thead>
                  <tr>
                    <th>Feature</th>
                    <th>Value</th>
                    <th>Unit</th>
                  </tr>
                </thead>
                <tbody>
                  {(featuresLatest.features ?? []).map((feature) => (
                    <tr key={feature.id}>
                      <td>{feature.id}</td>
                      <td>{feature.value.toFixed(6)}</td>
                      <td>{feature.unit}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </details>
            <dl className="metrics metrics-compact feature-provenance" data-testid="features-provenance">
              <div>
                <dt>DSP profile</dt>
                <dd>
                  {featuresLatest.dsp_profile_id ?? '—'} v{featuresLatest.dsp_profile_version ?? '—'}
                </dd>
              </div>
              <div>
                <dt>DSP backend</dt>
                <dd>
                  {featuresLatest.dsp_backend_id ?? '—'} {featuresLatest.dsp_backend_version ?? ''}
                </dd>
              </div>
              <div>
                <dt>Calibration profile</dt>
                <dd>
                  {featuresLatest.calibration_profile_id ?? '—'} v
                  {featuresLatest.calibration_profile_version ?? '—'}
                </dd>
              </div>
            </dl>
          </>
        )}
      </section>

      <section className="panel" aria-labelledby="observation-panel-heading">
        <h2 id="observation-panel-heading">Channel-change observation</h2>
        <p className="observation-disclaimer" data-testid="observation-disclaimer">
          {observationLatest?.disclaimer ?? OBSERVATION_DISCLAIMER}
        </p>
        {perceptionDisabled ? (
          <p className="muted" data-testid="observation-disabled-banner">
            Channel-change observation is disabled.
          </p>
        ) : null}
        {!observationLatest?.available ? (
          <p className="muted" data-testid="observation-latest-empty">
            No channel-change observation available yet.
          </p>
        ) : (
          <dl className="metrics metrics-compact" data-testid="observation-latest">
            <div>
              <dt>State</dt>
              <dd data-testid="observation-channel-state">
                {presentChannelChangeState(observationLatest.state)}
              </dd>
            </div>
            <div>
              <dt>Activity score</dt>
              <dd data-testid="observation-activity-score">
                {observationLatest.activity_score?.toFixed(4) ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Score semantics</dt>
              <dd className="ellipsis" data-testid="observation-score-semantics">
                {observationLatest.score_semantics ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Reliability</dt>
              <dd data-testid="observation-reliability">
                {observationLatest.uncertainty?.reliability_score.toFixed(3) ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Threshold margin</dt>
              <dd data-testid="observation-threshold-margin">
                {observationLatest.uncertainty?.threshold_margin.toFixed(4) ?? '—'}
              </dd>
            </div>
            <div>
              <dt>Window</dt>
              <dd>
                {observationLatest.first_sequence ?? '—'}–{observationLatest.last_sequence ?? '—'}
              </dd>
            </div>
          </dl>
        )}
      </section>

      <section className="panel" aria-labelledby="observatory-heading">
        <h2 id="observatory-heading">Signal Observatory</h2>
        {dspDisabled ? (
          <p className="muted" data-testid="dsp-disabled-banner">
            DSP is disabled. Spectral and motion charts are unavailable.
          </p>
        ) : null}

        <div className="chart-grid">
          <LineChart
            title="Raw vs Calibrated Amplitude"
            testId="amplitude-chart"
            ariaLabel="Raw versus calibrated amplitude by subcarrier"
            xValues={subcarriers}
            series={[
              {
                name: 'raw',
                values: signalLatest?.raw_amplitudes ?? [],
                color: '#5c6570',
              },
              {
                name: 'calibrated',
                values: signalLatest?.calibrated_amplitudes ?? [],
                color: '#1f4b6e',
              },
            ]}
            empty={!hasAmplitude}
            loading={signalLoading}
          />

          <LineChart
            title="Raw vs Calibrated Phase"
            testId="phase-chart"
            ariaLabel="Raw versus calibrated phase by subcarrier"
            xValues={subcarriers}
            series={[
              {
                name: 'raw',
                values: signalLatest?.raw_wrapped_phases ?? [],
                color: '#5c6570',
              },
              {
                name: 'calibrated',
                values: signalLatest?.calibrated_phases ?? [],
                color: '#8a5a12',
              },
            ]}
            empty={!hasPhase}
            loading={signalLoading}
            annotation="Calibrated phase = spatial unwrap + affine detrend (not full hardware calibration)."
          />

          <Heatmap
            title="Calibrated Amplitude Heatmap"
            testId="amplitude-heatmap"
            ariaLabel="Calibrated amplitude heatmap links by subcarriers"
            values={heatmap.values}
            rowLabels={heatmap.rowLabels}
            colLabels={heatmap.colLabels}
            empty={!hasHeatmap}
            loading={signalLoading}
          />

          <LineChart
            title="CSI Motion-Energy Proxy"
            testId="motion-chart"
            ariaLabel="CSI motion energy proxy over time"
            xValues={motionX}
            series={[
              {
                name: 'motion_energy',
                values: motionY,
                color: '#1f6b45',
              },
            ]}
            empty={dspDisabled || !hasMotion}
            loading={!dspDisabled && dspLatestLoading}
            error={dspDisabled ? 'DSP disabled' : null}
            annotation="Channel-change proxy — not human-motion classification. Backend selection uses identical DSP semantics and is validated through parity tests."
          />

          <LineChart
            title="Power Spectrum"
            testId="spectrum-chart"
            ariaLabel="Power spectrum with dominant non-DC frequency"
            xValues={spectrumX}
            series={[
              {
                name: 'power',
                values: spectrumY,
                color: '#1f4b6e',
              },
            ]}
            empty={dspDisabled || !hasSpectrum}
            loading={!dspDisabled && dspLatestLoading}
            error={dspDisabled ? 'DSP disabled' : null}
            annotation={
              dominantHz !== null && Number.isFinite(dominantHz)
                ? `Dominant non-DC: ${dominantHz.toFixed(3)} Hz. Peaks are not interpreted as activities.`
                : 'Peaks are not interpreted as activities.'
            }
          />

          <LineChart
            title="Frequency Power Distribution"
            testId="frequency-band-chart"
            ariaLabel="Low, middle, and high non-DC spectral power ratios"
            xValues={frequencyBandX}
            series={[
              {
                name: 'power_ratio',
                values: frequencyBandY,
                color: '#6b4b1f',
              },
            ]}
            empty={featuresDisabled || !hasFrequencyBands}
            loading={!featuresDisabled && featuresLatestLoading}
            error={featuresDisabled ? 'Feature extraction disabled' : null}
            annotation="Non-DC power fractions by relative frequency band; not activity labels."
          />
        </div>
      </section>

      <section className="panel panel-compact" aria-labelledby="activity-heading">
        <h2 id="activity-heading">Live activity</h2>
        <dl className="metrics metrics-compact" data-testid="activity-metrics">
          <div>
            <dt>Frames received</dt>
            <dd data-testid="frames-received">{framesReceived}</dd>
          </div>
          <div>
            <dt>Latest frame time</dt>
            <dd data-testid="latest-frame-time">{formatTimestamp(latestFrameTimestamp)}</dd>
          </div>
          <div>
            <dt>DSP windows</dt>
            <dd>{dsp?.windows_emitted ?? '—'}</dd>
          </div>
          <div>
            <dt>Calibrated frames</dt>
            <dd>{calibration?.frames_calibrated ?? '—'}</dd>
          </div>
        </dl>
      </section>

      <section className="panel" aria-labelledby="events-heading">
        <h2 id="events-heading">Recent event timeline</h2>
        {events.length === 0 ? (
          <p className="muted" data-testid="events-empty">
            No events yet.
          </p>
        ) : (
          <ol className="event-list" data-testid="event-timeline">
            {events.map((event, index) => (
              <li key={`${event.timestamp}-${event.type}-${index}`}>
                <span className="event-type ellipsis" title={event.type}>
                  {formatEventType(event.type)}
                </span>
                <span className="event-time" title={event.timestamp}>
                  {formatTimestamp(event.timestamp)}
                </span>
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  )
}
