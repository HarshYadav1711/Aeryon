//! DSP status and latest-result routes.

use axum::Json;
use axum::extract::{Query, State};

use crate::api::dto::{DspLatestResponse, DspSnapshot, LinkQuery};
use crate::api::error::ApiError;
use crate::api::state::AppState;

pub async fn dsp_handler(State(state): State<AppState>) -> Json<DspSnapshot> {
    Json(state.dsp_snapshot().await)
}

pub async fn dsp_latest_handler(
    State(state): State<AppState>,
    Query(query): Query<LinkQuery>,
) -> Result<Json<DspLatestResponse>, ApiError> {
    let (rx, tx) = query.resolve();
    Ok(Json(state.dsp_latest_snapshot(rx, tx).await?))
}
