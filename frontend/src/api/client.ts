import type {
  HealthResponse,
  PluginsResponse,
  RuntimeSnapshot,
  SyntheticSensorSnapshot,
} from './types'

const DEFAULT_API_URL = 'http://127.0.0.1:8080'
const DEFAULT_WS_URL = 'ws://127.0.0.1:8080'

export function apiBaseUrl(): string {
  return (import.meta.env.VITE_AERYON_API_URL ?? DEFAULT_API_URL).replace(/\/$/, '')
}

export function wsBaseUrl(): string {
  return (import.meta.env.VITE_AERYON_WS_URL ?? DEFAULT_WS_URL).replace(/\/$/, '')
}

export class ApiClientError extends Error {
  readonly status: number
  readonly code?: string

  constructor(message: string, status: number, code?: string) {
    super(message)
    this.name = 'ApiClientError'
    this.status = status
    this.code = code
  }
}

async function getJson<T>(path: string): Promise<T> {
  let response: Response
  try {
    response = await fetch(`${apiBaseUrl()}${path}`)
  } catch {
    throw new ApiClientError('Server unavailable', 0, 'server_unavailable')
  }

  const text = await response.text()
  let body: unknown = null
  if (text) {
    try {
      body = JSON.parse(text) as unknown
    } catch {
      body = null
    }
  }

  if (!response.ok) {
    const errorBody = body as { error?: { code?: string; message?: string } } | null
    throw new ApiClientError(
      errorBody?.error?.message ?? `Request failed (${response.status})`,
      response.status,
      errorBody?.error?.code,
    )
  }

  return body as T
}

export const apiClient = {
  getHealth(): Promise<HealthResponse> {
    return getJson<HealthResponse>('/health')
  },
  getRuntime(): Promise<RuntimeSnapshot> {
    return getJson<RuntimeSnapshot>('/api/v1/runtime')
  },
  getPlugins(): Promise<PluginsResponse> {
    return getJson<PluginsResponse>('/api/v1/plugins')
  },
  getSyntheticSensor(): Promise<SyntheticSensorSnapshot> {
    return getJson<SyntheticSensorSnapshot>('/api/v1/sensors/synthetic')
  },
}
