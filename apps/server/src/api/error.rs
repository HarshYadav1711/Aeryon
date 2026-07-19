//! Typed API errors converted into HTTP responses at the boundary.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::dto::{ErrorBody, ErrorResponse};

/// Typed internal API error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    /// Creates an API error with an explicit status and code.
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    /// Resource or route was not found.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code.to_owned(),
                message: self.message,
            },
        };
        (self.status, Json(body)).into_response()
    }
}
