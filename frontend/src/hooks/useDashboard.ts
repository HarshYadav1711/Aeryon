import { useCallback, useEffect, useRef, useState } from 'react'

import { ApiClientError, apiClient } from '../api/client'
import { EventStream } from '../api/eventStream'
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

export const MAX_EVENTS = 50
const REST_POLL_MS = 2_000
const FPS_WINDOW_MS = 5_000

export type DashboardState = {
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

function emptyState(): DashboardState {
  return {
    connection: 'loading',
    runtime: null,
    sensor: null,
    csiReplay: null,
    calibration: null,
    dsp: null,
    features: null,
    featuresLatest: null,
    perception: null,
    observationLatest: null,
    signalLatest: null,
    dspLatest: null,
    events: [],
    framesReceived: 0,
    latestSequence: null,
    latestFrameTimestamp: null,
    framesPerSecond: null,
    restError: null,
  }
}

function isFrameEvent(type: string): boolean {
  return type === 'sensor_frame' || type === 'csi_frame'
}

function shouldRefetchSignal(type: string): boolean {
  return (
    type === 'dsp_window_processed' ||
    type === 'csi_frame_calibrated' ||
    type === 'feature_vector_produced' ||
    type === 'channel_change_observed'
  )
}

function eventKey(event: ApiEventEnvelope): string {
  return `${event.type}|${event.timestamp}`
}

function mergeEventsNewestFirst(
  existing: ApiEventEnvelope[],
  incoming: ApiEventEnvelope[],
): ApiEventEnvelope[] {
  const seen = new Set<string>()
  const merged: ApiEventEnvelope[] = []
  for (const event of [...incoming, ...existing]) {
    const key = eventKey(event)
    if (seen.has(key)) {
      continue
    }
    seen.add(key)
    merged.push(event)
    if (merged.length >= MAX_EVENTS) {
      break
    }
  }
  return merged
}

function computeFps(
  frameTimes: number[],
  now: number,
  replayCompleted: boolean,
): number | null {
  const recent = frameTimes.filter((time) => now - time <= FPS_WINDOW_MS)
  if (replayCompleted && recent.length === 0) {
    return null
  }
  if (recent.length < 2) {
    return null
  }
  const spanMs = recent[recent.length - 1]! - recent[0]!
  if (spanMs <= 0) {
    return null
  }
  return ((recent.length - 1) * 1000) / spanMs
}

export function useDashboard(): DashboardState {
  const [state, setState] = useState<DashboardState>(emptyState)
  const frameTimesRef = useRef<number[]>([])
  const wsConnectedRef = useRef(false)
  const replayCompletedRef = useRef(false)

  const refreshSignalViews = useCallback(async () => {
    try {
      const [signalLatest, dspLatest, featuresLatest, observationLatest] = await Promise.all([
        apiClient.getSignalLatest(),
        apiClient.getDspLatest(),
        apiClient.getFeaturesLatest(),
        apiClient.getObservationLatest(),
      ])
      setState((prev) => ({
        ...prev,
        signalLatest,
        dspLatest,
        featuresLatest,
        observationLatest,
      }))
    } catch {
      // Keep last signal views; status poll reports REST failures.
    }
  }, [])

  const refreshRest = useCallback(async () => {
    try {
      const [
        runtime,
        sensor,
        csiReplay,
        calibration,
        dsp,
        features,
        perception,
        signalLatest,
        dspLatest,
        featuresLatest,
        observationLatest,
      ] = await Promise.all([
        apiClient.getRuntime(),
        apiClient.getSyntheticSensor(),
        apiClient.getCsiReplay(),
        apiClient.getCalibration(),
        apiClient.getDsp(),
        apiClient.getFeatures(),
        apiClient.getPerception(),
        apiClient.getSignalLatest(),
        apiClient.getDspLatest(),
        apiClient.getFeaturesLatest(),
        apiClient.getObservationLatest(),
      ])
      replayCompletedRef.current = csiReplay.completion === 'completed'
      const now = Date.now()
      frameTimesRef.current = frameTimesRef.current.filter((time) => now - time <= FPS_WINDOW_MS)
      const framesPerSecond = computeFps(
        frameTimesRef.current,
        now,
        replayCompletedRef.current,
      )
      setState((prev) => ({
        ...prev,
        runtime,
        sensor,
        csiReplay,
        calibration,
        dsp,
        features,
        perception,
        signalLatest,
        dspLatest,
        featuresLatest,
        observationLatest,
        restError: null,
        framesReceived: Math.max(prev.framesReceived, runtime.frames_received),
        latestSequence: runtime.last_frame_sequence ?? prev.latestSequence,
        latestFrameTimestamp: runtime.last_frame_timestamp ?? prev.latestFrameTimestamp,
        framesPerSecond,
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
        // Recent events first so the timeline is seeded before WS appends.
        const recent = await apiClient.getRecentEvents(MAX_EVENTS)
        if (cancelled) {
          return
        }
        const seeded = [...recent.events].reverse().slice(0, MAX_EVENTS)
        setState((prev) => ({
          ...prev,
          events: mergeEventsNewestFirst([], seeded),
        }))

        const [
          runtime,
          sensor,
          csiReplay,
          calibration,
          dsp,
          features,
          perception,
          signalLatest,
          dspLatest,
          featuresLatest,
          observationLatest,
        ] = await Promise.all([
          apiClient.getRuntime(),
          apiClient.getSyntheticSensor(),
          apiClient.getCsiReplay(),
          apiClient.getCalibration(),
          apiClient.getDsp(),
          apiClient.getFeatures(),
          apiClient.getPerception(),
          apiClient.getSignalLatest(),
          apiClient.getDspLatest(),
          apiClient.getFeaturesLatest(),
          apiClient.getObservationLatest(),
        ])
        if (cancelled) {
          return
        }
        replayCompletedRef.current = csiReplay.completion === 'completed'
        setState((prev) => ({
          ...prev,
          runtime,
          sensor,
          csiReplay,
          calibration,
          dsp,
          features,
          perception,
          signalLatest,
          dspLatest,
          featuresLatest,
          observationLatest,
          framesReceived: runtime.frames_received,
          latestSequence: runtime.last_frame_sequence,
          latestFrameTimestamp: runtime.last_frame_timestamp,
          framesPerSecond: computeFps(
            frameTimesRef.current,
            Date.now(),
            replayCompletedRef.current,
          ),
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
        if (shouldRefetchSignal(event.type)) {
          void refreshSignalViews()
        }
        setState((prev) => {
          const events = mergeEventsNewestFirst(prev.events, [event])
          let framesReceived = prev.framesReceived
          let latestSequence = prev.latestSequence
          let latestFrameTimestamp = prev.latestFrameTimestamp

          if (isFrameEvent(event.type)) {
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
              (time) => now - time <= FPS_WINDOW_MS,
            )
          }

          const framesPerSecond = computeFps(
            frameTimesRef.current,
            now,
            replayCompletedRef.current || prev.csiReplay?.completion === 'completed',
          )

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
  }, [refreshRest, refreshSignalViews])

  return state
}

function numberField(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function stringField(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null
}
