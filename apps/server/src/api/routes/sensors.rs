//! Synthetic sensor snapshot endpoint.

use axum::Json;
use axum::extract::State;

use crate::api::dto::SyntheticSensorSnapshot;
use crate::api::state::AppState;

pub async fn synthetic_sensor_handler(
    State(state): State<AppState>,
) -> Json<SyntheticSensorSnapshot> {
    Json(state.synthetic_sensor_snapshot().await)
}
