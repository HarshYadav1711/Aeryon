//! WebSocket event stream over the existing typed event bus.

use std::sync::Arc;

use aeryon_events::BusError;
use aeryon_runtime::Runtime;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::{RwLock, mpsc};

use crate::api::dto::ApiEventEnvelope;
use crate::api::event_map::domain_event_to_envelope;
use crate::api::state::AppState;
use crate::api::time::now_rfc3339;

const OUTBOUND_BUFFER: usize = 64;

pub async fn events_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, Arc::clone(state.runtime())))
}

/// `GET /api/v1/events/recent` — bounded chronological event history.
pub async fn recent_events_handler(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<crate::api::dto::RecentEventsQuery>,
) -> axum::Json<crate::api::dto::RecentEventsResponse> {
    axum::Json(state.recent_events_snapshot(query.limit).await)
}

async fn handle_socket(socket: WebSocket, runtime: Arc<RwLock<Runtime>>) {
    let (event_bus, samples_per_frame) = {
        let guard = runtime.read().await;
        (
            guard.context().event_bus.clone(),
            guard.context().config.synthetic_sensor.samples_per_frame,
        )
    };

    let mut bus_rx = event_bus.subscribe();
    let (mut sink, mut stream) = socket.split();
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<ApiEventEnvelope>(OUTBOUND_BUFFER);

    tracing::info!("WebSocket event stream connected");

    let forward = tokio::spawn(async move {
        loop {
            match bus_rx.recv().await {
                Ok(event) => {
                    if let Some(envelope) = domain_event_to_envelope(event, samples_per_frame) {
                        match outbound_tx.try_send(envelope) {
                            Ok(()) => {}
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                tracing::warn!(
                                    "WebSocket event stream lagging; dropping outbound event"
                                );
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => break,
                        }
                    }
                }
                Err(BusError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "WebSocket event-bus subscriber lagged");
                }
                Err(BusError::Closed) => break,
                Err(BusError::NoSubscribers) => {}
            }
        }
    });

    loop {
        tokio::select! {
            envelope = outbound_rx.recv() => {
                match envelope {
                    Some(envelope) => {
                        match serde_json::to_string(&envelope) {
                            Ok(text) => {
                                if sink.send(Message::Text(text.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(error) => {
                                tracing::warn!(%error, "failed to serialize outbound event");
                            }
                        }
                    }
                    None => break,
                }
            }
            inbound = stream.next() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Text(_) | Message::Binary(_))) => {
                        let unsupported = ApiEventEnvelope {
                            version: 1,
                            event_type: "error".to_owned(),
                            timestamp: now_rfc3339(),
                            payload: json!({
                                "code": "unsupported_inbound",
                                "message": "WebSocket is server-to-client only in this milestone"
                            }),
                        };
                        if let Ok(text) = serde_json::to_string(&unsupported) {
                            if sink.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Err(_)) => break,
                }
            }
        }
    }

    forward.abort();
    tracing::info!("WebSocket event stream disconnected");
}
