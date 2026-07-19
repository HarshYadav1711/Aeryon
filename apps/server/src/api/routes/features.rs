//! Feature extraction status and latest-vector routes.

use axum::Json;
use axum::extract::State;

use crate::api::dto::{FeaturesLatestResponse, FeaturesSnapshot};
use crate::api::state::AppState;

pub async fn features_handler(State(state): State<AppState>) -> Json<FeaturesSnapshot> {
    Json(state.features_snapshot().await)
}

pub async fn features_latest_handler(
    State(state): State<AppState>,
) -> Json<FeaturesLatestResponse> {
    Json(state.features_latest_snapshot().await)
}
