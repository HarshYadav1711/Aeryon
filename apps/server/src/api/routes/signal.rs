//! Latest raw/calibrated signal snapshot route.

use axum::Json;
use axum::extract::{Query, State};

use crate::api::dto::{LinkQuery, SignalLatestResponse};
use crate::api::error::ApiError;
use crate::api::state::AppState;

pub async fn signal_latest_handler(
    State(state): State<AppState>,
    Query(query): Query<LinkQuery>,
) -> Result<Json<SignalLatestResponse>, ApiError> {
    let (rx, tx) = query.resolve();
    Ok(Json(state.signal_latest_snapshot(rx, tx).await?))
}
