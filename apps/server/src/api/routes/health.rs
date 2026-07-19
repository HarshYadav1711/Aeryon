//! Health endpoint.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::api::state::AppState;

pub async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let (body, healthy) = state.health_snapshot().await;
    let status = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(body))
}
