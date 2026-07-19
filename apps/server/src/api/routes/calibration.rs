//! Calibration status route.

use axum::Json;
use axum::extract::State;

use crate::api::dto::CalibrationSnapshot;
use crate::api::state::AppState;

pub async fn calibration_handler(State(state): State<AppState>) -> Json<CalibrationSnapshot> {
    Json(state.calibration_snapshot().await)
}
