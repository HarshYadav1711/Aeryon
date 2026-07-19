//! Plugin listing endpoint.

use axum::Json;
use axum::extract::State;

use crate::api::dto::PluginsResponse;
use crate::api::state::AppState;

pub async fn plugins_handler(State(state): State<AppState>) -> Json<PluginsResponse> {
    Json(state.plugins_snapshot().await)
}
