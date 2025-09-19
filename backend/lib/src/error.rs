use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::data::indexer_db::repository::error::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("RPC error: {0}")]
    Rpc(#[from] jsonrpsee::core::client::Error),

    #[error("Storage error: {0}")]
    Storage(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Rpc(err) => (StatusCode::BAD_GATEWAY, err.to_string()),
            Error::Storage(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Error::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Conflict(msg) => (StatusCode::CONFLICT, msg),
            Error::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

impl From<RepositoryError> for Error {
    fn from(value: RepositoryError) -> Self {
        match value {
            database @ RepositoryError::Database(_) => Self::Database(database.to_string()),
            RepositoryError::Configuration(err)
            | RepositoryError::Transaction(err)
            | RepositoryError::Pool(err) => Self::Database(err.to_string()),
            invalid_input @ RepositoryError::InvalidInput(_) => {
                Self::BadRequest(invalid_input.to_string())
            }
            not_found @ RepositoryError::NotFound { .. } => Self::NotFound(not_found.to_string()),
        }
    }
}
