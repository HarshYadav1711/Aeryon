//! WebSocket event stream over the existing typed event bus.

use std::sync::Arc;

use aeryon_domain::{CsiReplayFailureKind, Event, SensorFailureKind};
use aeryon_events::BusError;
use aeryon_runtime::Runtime;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::{RwLock, mpsc};

use crate::api::dto::{
    ApiEventEnvelope, CsiFramePayload, CsiReplayLifecyclePayload, SensorFramePayload,
    SensorLifecyclePayload,
};
use crate::api::state::AppState;
use crate::api::time::{nanos_to_rfc3339, now_rfc3339};

const OUTBOUND_BUFFER: usize = 64;
const CSI_DATA_CLASSIFICATION: &str = "deterministic_development_fixture";

pub async fn events_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, Arc::clone(state.runtime())))
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

fn domain_event_to_envelope(event: Event, samples_per_frame: usize) -> Option<ApiEventEnvelope> {
    match event {
        Event::FrameReceived(frame) => {
            let payload = SensorFramePayload {
                sensor_id: frame.sensor_id.value(),
                sequence: frame.sequence,
                frame_id: frame.frame_id.value(),
                capture_timestamp: nanos_to_rfc3339(frame.timestamp.as_nanos()),
                samples_per_frame,
                source_type: "synthetic",
            };
            Some(ApiEventEnvelope {
                version: 1,
                event_type: "sensor_frame".to_owned(),
                timestamp: now_rfc3339(),
                payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
            })
        }
        Event::CsiFrameReceived(frame) => {
            let payload = CsiFramePayload {
                sensor_id: frame.sensor_id.value(),
                sequence: frame.sequence,
                frame_id: frame.frame_id.value(),
                capture_timestamp: nanos_to_rfc3339(frame.capture_timestamp.as_nanos()),
                receive_timestamp: nanos_to_rfc3339(frame.receive_timestamp.as_nanos()),
                receive_antennas: frame.receive_antennas,
                transmit_antennas: frame.transmit_antennas,
                subcarrier_count: frame.subcarrier_count,
                center_frequency_hz: frame.center_frequency_hz,
                bandwidth_hz: frame.bandwidth_hz,
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                live_hardware: false,
            };
            Some(ApiEventEnvelope {
                version: 1,
                event_type: "csi_frame".to_owned(),
                timestamp: now_rfc3339(),
                payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
            })
        }
        Event::SensorStarted(event) => Some(lifecycle_envelope(
            "sensor_started",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::SensorStopped(event) => Some(lifecycle_envelope(
            "sensor_stopped",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::SensorFailed(event) => Some(lifecycle_envelope(
            "sensor_failed",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: Some(failure_kind_label(event.kind)),
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayStarted(event) => Some(csi_lifecycle_envelope(
            "csi_replay_started",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayCompleted(event) => Some(csi_lifecycle_envelope(
            "csi_replay_completed",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: Some(event.frames_accepted),
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayStopped(event) => Some(csi_lifecycle_envelope(
            "csi_replay_stopped",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayFailed(event) => Some(csi_lifecycle_envelope(
            "csi_replay_failed",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: Some(csi_failure_kind_label(event.kind)),
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        _ => None,
    }
}

fn lifecycle_envelope(
    event_type: &str,
    payload: SensorLifecyclePayload,
    timestamp: String,
) -> ApiEventEnvelope {
    ApiEventEnvelope {
        version: 1,
        event_type: event_type.to_owned(),
        timestamp,
        payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
    }
}

fn csi_lifecycle_envelope(
    event_type: &str,
    payload: CsiReplayLifecyclePayload,
    timestamp: String,
) -> ApiEventEnvelope {
    ApiEventEnvelope {
        version: 1,
        event_type: event_type.to_owned(),
        timestamp,
        payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
    }
}

fn failure_kind_label(kind: SensorFailureKind) -> &'static str {
    match kind {
        SensorFailureKind::ProducerExited => "producer_exited",
        SensorFailureKind::PublishFailed => "publish_failed",
    }
}

fn csi_failure_kind_label(kind: CsiReplayFailureKind) -> &'static str {
    match kind {
        CsiReplayFailureKind::FixtureError => "fixture_error",
        CsiReplayFailureKind::MalformedFrame => "malformed_frame",
        CsiReplayFailureKind::PublishFailed => "publish_failed",
        CsiReplayFailureKind::ProducerExited => "producer_exited",
    }
}
