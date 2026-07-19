export type HeatmapProps = {
  title?: string
  /** Row-major magnitude grid: rows × cols. */
  values: number[][]
  rowLabels?: string[]
  colLabels?: string[]
  empty?: boolean
  loading?: boolean
  error?: string | null
  testId?: string
  ariaLabel?: string
}

function clamp01(v: number): number {
  if (!Number.isFinite(v)) {
    return 0
  }
  return Math.min(1, Math.max(0, v))
}

/** Cool→warm scale on existing accent/ok palette (no purple). */
function magnitudeColor(t: number): string {
  const x = clamp01(t)
  // #eef2f5 → #1f4b6e → #1f6b45
  if (x < 0.5) {
    const u = x / 0.5
    const r = Math.round(238 + (31 - 238) * u)
    const g = Math.round(242 + (75 - 242) * u)
    const b = Math.round(245 + (110 - 245) * u)
    return `rgb(${r},${g},${b})`
  }
  const u = (x - 0.5) / 0.5
  const r = Math.round(31 + (31 - 31) * u)
  const g = Math.round(75 + (107 - 75) * u)
  const b = Math.round(110 + (69 - 110) * u)
  return `rgb(${r},${g},${b})`
}

export function Heatmap({
  title,
  values,
  rowLabels,
  colLabels,
  empty = false,
  loading = false,
  error = null,
  testId = 'heatmap',
  ariaLabel,
}: HeatmapProps) {
  const rows = values.length
  const cols = rows > 0 ? Math.max(...values.map((r) => r.length)) : 0
  const flat = values.flat()
  const min = flat.length > 0 ? Math.min(...flat) : 0
  const max = flat.length > 0 ? Math.max(...flat) : 1
  const span = max - min || 1
  const label = ariaLabel ?? title ?? 'Heatmap'

  return (
    <div className="chart-card" data-testid={testId}>
      {title ? <h3 className="chart-title">{title}</h3> : null}
      {loading ? (
        <p className="muted chart-status" data-testid={`${testId}-loading`}>
          Loading…
        </p>
      ) : null}
      {error ? (
        <p className="chart-status chart-error" data-testid={`${testId}-error`}>
          {error}
        </p>
      ) : null}
      {!loading && !error && (empty || rows === 0 || cols === 0) ? (
        <p className="muted chart-status" data-testid={`${testId}-empty`}>
          No data available.
        </p>
      ) : null}
      {!loading && !error && !empty && rows > 0 && cols > 0 ? (
        <>
          <div
            className="heatmap-grid"
            role="img"
            aria-label={label}
            data-testid={`${testId}-grid`}
            data-rows={rows}
            data-cols={cols}
            style={{
              gridTemplateColumns: `auto repeat(${cols}, minmax(8px, 1fr))`,
            }}
          >
            <div className="heatmap-corner" />
            {Array.from({ length: cols }, (_, c) => (
              <div key={`col-${c}`} className="heatmap-col-label" title={colLabels?.[c]}>
                {colLabels?.[c] ?? c}
              </div>
            ))}
            {values.map((row, r) => (
              <div key={`row-${r}`} className="heatmap-row" style={{ display: 'contents' }}>
                <div className="heatmap-row-label" title={rowLabels?.[r]}>
                  {rowLabels?.[r] ?? `r${r}`}
                </div>
                {Array.from({ length: cols }, (_, c) => {
                  const v = row[c] ?? min
                  const t = (v - min) / span
                  return (
                    <div
                      key={`cell-${r}-${c}`}
                      className="heatmap-cell"
                      data-testid={`${testId}-cell-${r}-${c}`}
                      title={`${rowLabels?.[r] ?? r} / sc ${colLabels?.[c] ?? c}: ${v.toFixed(3)}`}
                      style={{ background: magnitudeColor(t) }}
                    />
                  )
                })}
              </div>
            ))}
          </div>
          <div className="heatmap-legend" data-testid={`${testId}-legend`} aria-hidden="true">
            <span>{min.toFixed(2)}</span>
            <div className="heatmap-legend-bar" />
            <span>{max.toFixed(2)}</span>
          </div>
        </>
      ) : null}
    </div>
  )
}
