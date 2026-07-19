import type {
  ApiEventEnvelope,
  CalibrationSnapshot,
  ConnectionState,
  CsiReplaySnapshot,
  DspLatestResponse,
  DspSnapshot,
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

export function formatStageName(stage: string): string {
  return STAGE_DISPLAY[stage] ?? stage.replaceAll('_', ' ')
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

  return [
    { id: 'csi_replay', label: 'CSI Replay', state: replayState },
    { id: 'validation', label: 'Validation', state: validationState },
    { id: 'calibration', label: 'Calibration', state: calibrationState },
    { id: 'windowing', label: 'Windowing', state: windowingState },
    { id: 'spectral_dsp', label: 'Spectral DSP', state: spectralState },
    { id: 'perception', label: 'Perception', state: 'not_implemented' },
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
  const pipeline = buildPipelineStages(csiReplay, calibration, dsp)
  const dspDisabled = dsp !== null && !dsp.enabled
  const signalLoading = signalLatest === null && connection === 'loading'
  const dspLatestLoading = dspLatest === null && connection === 'loading'

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
        <p className="muted pipeline-note" data-testid="perception-note">
          Perception is not implemented in this milestone.
        </p>
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
                  {event.type}
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
