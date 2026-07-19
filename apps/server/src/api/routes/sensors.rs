//! Sensor snapshot endpoints.

use axum::Json;
use axum::extract::State;

use crate::api::dto::{CsiReplaySnapshot, SyntheticSensorSnapshot};
use crate::api::state::AppState;

pub async fn synthetic_sensor_handler(
    State(state): State<AppState>,
) -> Json<SyntheticSensorSnapshot> {
    Json(state.synthetic_sensor_snapshot().await)
}

pub async fn csi_replay_handler(State(state): State<AppState>) -> Json<CsiReplaySnapshot> {
    Json(state.csi_replay_snapshot().await)
}
