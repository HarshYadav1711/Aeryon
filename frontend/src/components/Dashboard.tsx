import type {
  ApiEventEnvelope,
  ConnectionState,
  RuntimeSnapshot,
  SyntheticSensorSnapshot,
} from '../api/types'

export type DashboardProps = {
  connection: ConnectionState
  runtime: RuntimeSnapshot | null
  sensor: SyntheticSensorSnapshot | null
  events: ApiEventEnvelope[]
  framesReceived: number
  latestSequence: number | null
  latestFrameTimestamp: string | null
  framesPerSecond: number | null
  restError: string | null
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

export function Dashboard({
  connection,
  runtime,
  sensor,
  events,
  framesReceived,
  latestSequence,
  latestFrameTimestamp,
  framesPerSecond,
  restError,
}: DashboardProps) {
  const noFrameYet =
    connection === 'connected' &&
    framesReceived === 0 &&
    latestSequence === null &&
    latestFrameTimestamp === null

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <div>
          <h1>Aeryon</h1>
          <p className="tagline">Transforming Signals into Understanding</p>
        </div>
        <div className="source-badge" data-testid="source-badge">
          Synthetic Development Source
        </div>
      </header>

      <p className="source-note">
        Deterministic Synthetic Sensor — platform integration validation only. Not WiFi CSI.
      </p>

      <section className="panel" aria-labelledby="connection-heading">
        <h2 id="connection-heading">Connection</h2>
        <dl className="metrics">
          <div>
            <dt>State</dt>
            <dd data-testid="connection-state" data-state={connection}>
              {connectionLabel(connection)}
            </dd>
          </div>
          {restError ? (
            <div>
              <dt>REST error</dt>
              <dd data-testid="rest-error">{restError}</dd>
            </div>
          ) : null}
        </dl>
      </section>

      <section className="panel" aria-labelledby="runtime-heading">
        <h2 id="runtime-heading">Runtime status</h2>
        {runtime ? (
          <dl className="metrics" data-testid="runtime-snapshot">
            <div>
              <dt>Lifecycle</dt>
              <dd>{runtime.lifecycle_state}</dd>
            </div>
            <div>
              <dt>Uptime</dt>
              <dd>{formatUptime(runtime.uptime_secs)}</dd>
            </div>
            <div>
              <dt>Version</dt>
              <dd>{runtime.application_version}</dd>
            </div>
            <div>
              <dt>Application</dt>
              <dd>{runtime.application_name}</dd>
            </div>
          </dl>
        ) : (
          <p className="muted" data-testid="runtime-empty">
            Waiting for runtime snapshot…
          </p>
        )}
      </section>

      <section className="panel" aria-labelledby="sensor-heading">
        <h2 id="sensor-heading">Sensor status</h2>
        {sensor ? (
          <dl className="metrics" data-testid="sensor-snapshot">
            <div>
              <dt>Enabled</dt>
              <dd>{sensor.enabled ? 'yes' : 'no'}</dd>
            </div>
            <div>
              <dt>Lifecycle</dt>
              <dd>{sensor.lifecycle_state ?? '—'}</dd>
            </div>
            <div>
              <dt>Health</dt>
              <dd>{sensor.health ?? '—'}</dd>
            </div>
            <div>
              <dt>Sample rate</dt>
              <dd>{sensor.sample_rate_hz} Hz</dd>
            </div>
            <div>
              <dt>Samples / frame</dt>
              <dd>{sensor.samples_per_frame}</dd>
            </div>
            <div>
              <dt>Frequencies</dt>
              <dd>
                {sensor.configured_frequencies_hz.primary_hz} Hz /{' '}
                {sensor.configured_frequencies_hz.secondary_hz} Hz
              </dd>
            </div>
          </dl>
        ) : (
          <p className="muted">Waiting for sensor snapshot…</p>
        )}
      </section>

      <section className="panel" aria-labelledby="activity-heading">
        <h2 id="activity-heading">Live signal activity</h2>
        {noFrameYet ? (
          <p className="muted" data-testid="no-frame-state">
            Connected — no frame received yet.
          </p>
        ) : null}
        <dl className="metrics" data-testid="activity-metrics">
          <div>
            <dt>Frames received</dt>
            <dd data-testid="frames-received">{framesReceived}</dd>
          </div>
          <div>
            <dt>Latest sequence</dt>
            <dd data-testid="latest-sequence">{latestSequence ?? '—'}</dd>
          </div>
          <div>
            <dt>Latest frame time</dt>
            <dd>{latestFrameTimestamp ?? '—'}</dd>
          </div>
          <div>
            <dt>Estimated FPS</dt>
            <dd>
              {framesPerSecond === null ? '—' : framesPerSecond.toFixed(1)}
            </dd>
          </div>
        </dl>
      </section>

      <section className="panel" aria-labelledby="events-heading">
        <h2 id="events-heading">Event timeline</h2>
        {events.length === 0 ? (
          <p className="muted" data-testid="events-empty">
            No live events yet.
          </p>
        ) : (
          <ol className="event-list" data-testid="event-timeline">
            {events.map((event, index) => (
              <li key={`${event.timestamp}-${event.type}-${index}`}>
                <span className="event-type">{event.type}</span>
                <span className="event-time">{event.timestamp}</span>
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  )
}
