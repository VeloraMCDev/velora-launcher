//! Standard host error shape retained alongside Yggdrasil's protocol-specific errors.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}
pub type AppResult<T> = Result<T, AppError>;
impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into() }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!("{}", self.message);
        }
        (self.status, Json(serde_json::json!({"error":self.message}))).into_response()
    }
}
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("database error: {error}") }
    }
}
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("io error: {error}") }
    }
}
impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::bad_request(format!("invalid JSON: {error}"))
    }
}
