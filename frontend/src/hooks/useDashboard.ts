import { useCallback, useEffect, useRef, useState } from 'react'

import { ApiClientError, apiClient } from '../api/client'
import { EventStream } from '../api/eventStream'
import type {
  ApiEventEnvelope,
  ConnectionState,
  RuntimeSnapshot,
  SyntheticSensorSnapshot,
} from '../api/types'

export const MAX_EVENTS = 50
const REST_POLL_MS = 2_000

export type DashboardState = {
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

function emptyState(): DashboardState {
  return {
    connection: 'loading',
    runtime: null,
    sensor: null,
    events: [],
    framesReceived: 0,
    latestSequence: null,
    latestFrameTimestamp: null,
    framesPerSecond: null,
    restError: null,
  }
}

export function useDashboard(): DashboardState {
  const [state, setState] = useState<DashboardState>(emptyState)
  const frameTimesRef = useRef<number[]>([])
  const wsConnectedRef = useRef(false)

  const refreshRest = useCallback(async () => {
    try {
      const [runtime, sensor] = await Promise.all([
        apiClient.getRuntime(),
        apiClient.getSyntheticSensor(),
      ])
      setState((prev) => ({
        ...prev,
        runtime,
        sensor,
        restError: null,
        framesReceived: Math.max(prev.framesReceived, runtime.frames_received),
        latestSequence: runtime.last_frame_sequence ?? prev.latestSequence,
        latestFrameTimestamp: runtime.last_frame_timestamp ?? prev.latestFrameTimestamp,
        connection: wsConnectedRef.current
          ? 'connected'
          : prev.connection === 'reconnecting'
            ? 'reconnecting'
            : prev.connection === 'loading'
              ? 'loading'
              : prev.connection === 'disconnected'
                ? 'disconnected'
                : prev.connection,
      }))
    } catch (error) {
      if (error instanceof ApiClientError && error.code === 'server_unavailable') {
        setState((prev) => ({
          ...prev,
          restError: error.message,
          connection: wsConnectedRef.current ? prev.connection : 'server_unavailable',
        }))
      } else {
        setState((prev) => ({
          ...prev,
          connection: 'rest_error',
          restError: error instanceof Error ? error.message : 'REST request failed',
        }))
      }
    }
  }, [])

  useEffect(() => {
    let cancelled = false

    const bootstrap = async () => {
      try {
        const [runtime, sensor] = await Promise.all([
          apiClient.getRuntime(),
          apiClient.getSyntheticSensor(),
        ])
        if (cancelled) {
          return
        }
        setState((prev) => ({
          ...prev,
          runtime,
          sensor,
          framesReceived: runtime.frames_received,
          latestSequence: runtime.last_frame_sequence,
          latestFrameTimestamp: runtime.last_frame_timestamp,
          restError: null,
        }))
      } catch (error) {
        if (cancelled) {
          return
        }
        if (error instanceof ApiClientError && error.code === 'server_unavailable') {
          setState((prev) => ({
            ...prev,
            connection: 'server_unavailable',
            restError: error.message,
          }))
        } else {
          setState((prev) => ({
            ...prev,
            connection: 'rest_error',
            restError: error instanceof Error ? error.message : 'REST request failed',
          }))
        }
      }
    }

    void bootstrap()
    const poll = setInterval(() => {
      void refreshRest()
    }, REST_POLL_MS)

    const stream = new EventStream({
      onOpen: () => {
        wsConnectedRef.current = true
        setState((prev) => ({ ...prev, connection: 'connected' }))
        void refreshRest()
      },
      onClose: () => {
        wsConnectedRef.current = false
        setState((prev) => ({
          ...prev,
          connection: prev.connection === 'server_unavailable' ? prev.connection : 'reconnecting',
        }))
      },
      onError: () => {
        if (!wsConnectedRef.current) {
          setState((prev) =>
            prev.connection === 'loading' ? { ...prev, connection: 'disconnected' } : prev,
          )
        }
      },
      onEvent: (event) => {
        const now = Date.now()
        setState((prev) => {
          const events = [event, ...prev.events].slice(0, MAX_EVENTS)
          let framesReceived = prev.framesReceived
          let latestSequence = prev.latestSequence
          let latestFrameTimestamp = prev.latestFrameTimestamp
          let framesPerSecond = prev.framesPerSecond

          if (event.type === 'sensor_frame') {
            const sequence = numberField(event.payload.sequence)
            const capture = stringField(event.payload.capture_timestamp)
            framesReceived += 1
            if (sequence !== null) {
              latestSequence = sequence
            }
            if (capture) {
              latestFrameTimestamp = capture
            }
            frameTimesRef.current = [...frameTimesRef.current, now].filter(
              (time) => now - time <= 5_000,
            )
            const times = frameTimesRef.current
            if (times.length >= 2) {
              const spanMs = times[times.length - 1]! - times[0]!
              framesPerSecond = spanMs > 0 ? ((times.length - 1) * 1000) / spanMs : null
            }
          }

          return {
            ...prev,
            events,
            framesReceived,
            latestSequence,
            latestFrameTimestamp,
            framesPerSecond,
            connection: 'connected',
          }
        })
      },
    })

    stream.connect()

    return () => {
      cancelled = true
      clearInterval(poll)
      stream.disconnect()
    }
  }, [refreshRest])

  return state
}

function numberField(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function stringField(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null
}
