import type { ApiEventEnvelope } from './types'
import { wsBaseUrl } from './client'

export type EventStreamHandlers = {
  onOpen?: () => void
  onClose?: () => void
  onError?: () => void
  onEvent?: (event: ApiEventEnvelope) => void
}

const INITIAL_BACKOFF_MS = 500
const MAX_BACKOFF_MS = 8_000

/**
 * Server-to-client WebSocket stream with bounded reconnect backoff.
 */
export class EventStream {
  private socket: WebSocket | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private backoffMs = INITIAL_BACKOFF_MS
  private closedByUser = false
  private readonly handlers: EventStreamHandlers

  constructor(handlers: EventStreamHandlers) {
    this.handlers = handlers
  }

  connect(): void {
    this.closedByUser = false
    this.openSocket()
  }

  disconnect(): void {
    this.closedByUser = true
    this.clearReconnectTimer()
    if (this.socket) {
      this.socket.close()
      this.socket = null
    }
  }

  private openSocket(): void {
    const url = `${wsBaseUrl()}/api/v1/events/ws`
    const socket = new WebSocket(url)
    this.socket = socket

    socket.onopen = () => {
      this.backoffMs = INITIAL_BACKOFF_MS
      this.handlers.onOpen?.()
    }

    socket.onmessage = (message) => {
      try {
        const parsed = JSON.parse(String(message.data)) as ApiEventEnvelope
        if (parsed && typeof parsed.type === 'string') {
          this.handlers.onEvent?.(parsed)
        }
      } catch {
        // Ignore malformed frames.
      }
    }

    socket.onerror = () => {
      this.handlers.onError?.()
    }

    socket.onclose = () => {
      this.socket = null
      this.handlers.onClose?.()
      if (!this.closedByUser) {
        this.scheduleReconnect()
      }
    }
  }

  private scheduleReconnect(): void {
    this.clearReconnectTimer()
    const delay = this.backoffMs
    this.backoffMs = Math.min(this.backoffMs * 2, MAX_BACKOFF_MS)
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      if (!this.closedByUser) {
        this.openSocket()
      }
    }, delay)
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }
}
