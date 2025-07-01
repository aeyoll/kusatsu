use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("File not found")]
    FileNotFound,

    #[error("File expired")]
    FileExpired,

    #[error("Download limit exceeded")]
    DownloadLimitExceeded,

    #[error("File too large")]
    FileTooLarge,

    #[error("Invalid file format")]
    InvalidFileFormat,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error")]
    InternalServerError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::FileNotFound => (StatusCode::NOT_FOUND, "File not found"),
            AppError::FileExpired => (StatusCode::GONE, "File has expired"),
            AppError::DownloadLimitExceeded => (StatusCode::GONE, "Download limit exceeded"),
            AppError::FileTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "File too large"),
            AppError::InvalidFileFormat => (StatusCode::BAD_REQUEST, "Invalid file format"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "Bad request"),
            AppError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error"),
            AppError::DatabaseError(_) => {
                tracing::error!("Database error: {}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            AppError::IoError(_) => {
                tracing::error!("IO error: {}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "IO error")
            }
            AppError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            AppError::ServerError(_) => {
                tracing::error!("Server error: {}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Server error")
            }
            AppError::InternalServerError => {
                tracing::error!("Internal server error: {}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "message": self.to_string()
        }));

        (status, body).into_response()
    }
}
