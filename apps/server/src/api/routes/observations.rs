//! Perception status and latest observation routes.

use axum::Json;
use axum::extract::State;

use crate::api::dto::{ObservationLatestResponse, PerceptionSnapshot};
use crate::api::state::AppState;

pub async fn perception_handler(State(state): State<AppState>) -> Json<PerceptionSnapshot> {
    Json(state.perception_snapshot().await)
}

pub async fn observation_latest_handler(
    State(state): State<AppState>,
) -> Json<ObservationLatestResponse> {
    Json(state.observation_latest_snapshot().await)
}
