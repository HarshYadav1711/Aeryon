export type LineSeries = {
  name: string
  values: number[]
  color: string
}

export type LineChartProps = {
  title?: string
  xValues: number[]
  series: LineSeries[]
  width?: number
  height?: number
  empty?: boolean
  loading?: boolean
  error?: string | null
  annotation?: string | null
  testId?: string
  ariaLabel?: string
}

function niceExtent(values: number[]): [number, number] {
  if (values.length === 0) {
    return [0, 1]
  }
  let min = Math.min(...values)
  let max = Math.max(...values)
  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return [0, 1]
  }
  if (min === max) {
    const pad = Math.abs(min) > 0 ? Math.abs(min) * 0.1 : 1
    return [min - pad, max + pad]
  }
  const pad = (max - min) * 0.05
  return [min - pad, max + pad]
}

export function LineChart({
  title,
  xValues,
  series,
  width = 420,
  height = 180,
  empty = false,
  loading = false,
  error = null,
  annotation = null,
  testId = 'line-chart',
  ariaLabel,
}: LineChartProps) {
  const padL = 40
  const padR = 12
  const padT = 12
  const padB = 28
  const innerW = width - padL - padR
  const innerH = height - padT - padB

  const allY = series.flatMap((s) => s.values)
  const [yMin, yMax] = niceExtent(allY)
  const xMin = xValues.length > 0 ? Math.min(...xValues) : 0
  const xMax = xValues.length > 0 ? Math.max(...xValues) : 1
  const xSpan = xMax - xMin || 1
  const ySpan = yMax - yMin || 1

  const toX = (x: number) => padL + ((x - xMin) / xSpan) * innerW
  const toY = (y: number) => padT + innerH - ((y - yMin) / ySpan) * innerH

  const paths = series.map((s) => {
    const points: string[] = []
    const n = Math.min(s.values.length, xValues.length)
    for (let i = 0; i < n; i += 1) {
      const x = xValues[i]!
      const y = s.values[i]!
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        continue
      }
      points.push(`${toX(x).toFixed(2)},${toY(y).toFixed(2)}`)
    }
    return { ...s, d: points.length > 0 ? `M ${points.join(' L ')}` : '' }
  })

  const label = ariaLabel ?? title ?? 'Line chart'

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
      {!loading && !error && empty ? (
        <p className="muted chart-status" data-testid={`${testId}-empty`}>
          No data available.
        </p>
      ) : null}
      {!loading && !error && !empty ? (
        <>
          <svg
            className="line-chart-svg"
            viewBox={`0 0 ${width} ${height}`}
            width="100%"
            role="img"
            aria-label={label}
            data-testid={`${testId}-svg`}
          >
            <line
              x1={padL}
              y1={padT}
              x2={padL}
              y2={padT + innerH}
              stroke="currentColor"
              strokeOpacity={0.25}
            />
            <line
              x1={padL}
              y1={padT + innerH}
              x2={padL + innerW}
              y2={padT + innerH}
              stroke="currentColor"
              strokeOpacity={0.25}
            />
            <text x={padL} y={padT + 10} className="chart-axis-label" textAnchor="start">
              {yMax.toFixed(2)}
            </text>
            <text
              x={padL}
              y={padT + innerH}
              className="chart-axis-label"
              textAnchor="start"
              dy="-2"
            >
              {yMin.toFixed(2)}
            </text>
            <text
              x={padL}
              y={height - 6}
              className="chart-axis-label"
              textAnchor="start"
            >
              {xMin.toFixed(0)}
            </text>
            <text
              x={padL + innerW}
              y={height - 6}
              className="chart-axis-label"
              textAnchor="end"
            >
              {xMax.toFixed(0)}
            </text>
            {paths.map((p) =>
              p.d ? (
                <path
                  key={p.name}
                  d={p.d}
                  fill="none"
                  stroke={p.color}
                  strokeWidth={1.75}
                  data-testid={`${testId}-series-${p.name}`}
                />
              ) : null,
            )}
          </svg>
          <ul className="chart-legend" data-testid={`${testId}-legend`}>
            {series.map((s) => (
              <li key={s.name}>
                <span className="legend-swatch" style={{ background: s.color }} />
                {s.name}
              </li>
            ))}
          </ul>
          {annotation ? (
            <p className="chart-annotation" data-testid={`${testId}-annotation`}>
              {annotation}
            </p>
          ) : null}
        </>
      ) : null}
    </div>
  )
}
