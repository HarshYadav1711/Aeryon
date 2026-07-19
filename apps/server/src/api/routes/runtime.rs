//! Runtime snapshot endpoint.

use axum::Json;
use axum::extract::State;

use crate::api::dto::RuntimeSnapshot;
use crate::api::state::AppState;

pub async fn runtime_handler(State(state): State<AppState>) -> Json<RuntimeSnapshot> {
    Json(state.runtime_snapshot().await)
}
