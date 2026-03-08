use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeatureNumberingError {
    #[error("Feature not found: {0}")]
    FeatureNotFound(String),

    #[error("Feature number already assigned: {0}")]
    NumberAlreadyAssigned(u32),

    #[error("Invalid feature status transition")]
    InvalidStatusTransition,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, FeatureNumberingError>;

// Implement IntoResponse for axum error handling
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};

impl IntoResponse for FeatureNumberingError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            FeatureNumberingError::FeatureNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            FeatureNumberingError::NumberAlreadyAssigned(_) => (StatusCode::CONFLICT, self.to_string()),
            FeatureNumberingError::InvalidStatusTransition => (StatusCode::BAD_REQUEST, self.to_string()),
            FeatureNumberingError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(serde_json::json!({
            "error": "FeatureNumberingError",
            "message": error_message
        }))).into_response()
    }
}