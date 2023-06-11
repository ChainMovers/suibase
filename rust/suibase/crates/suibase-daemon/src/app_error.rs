// Reference:
//    https://github.com/tokio-rs/axum/blob/v0.6.x/examples/anyhow-error-response/src/main.rs
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use anyhow::anyhow;

use serde_json::json;

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

impl AppError {
    pub fn new<E>(err: E) -> Self
    where
        E: Into<anyhow::Error>,
    {
        Self(err.into())
    }

    // Function that always return an AppError
    // built from a string.
    pub fn from_str(err: &str) -> Self {
        Self(anyhow!(err.to_string()))
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let inner_message = self.0.to_string();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": inner_message,
            })),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
