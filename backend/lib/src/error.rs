use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::{error, warn};

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
            Error::Config(ref msg) => {
                error!(error_msg = %msg, "Configuration error");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            Error::Rpc(ref err) => {
                error!(error = %err, "RPC error (Bad Gateway)");
                (StatusCode::BAD_GATEWAY, err.to_string())
            }
            Error::Storage(ref err) => {
                error!(error = %err, "Storage error");
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            Error::Database(ref msg) => {
                error!(error_msg = %msg, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            Error::NotFound(ref msg) => {
                warn!(error_msg = %msg, "Resource not found");
                (StatusCode::NOT_FOUND, msg.clone())
            }
            Error::BadRequest(ref msg) => {
                warn!(error_msg = %msg, "Bad request");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            Error::Unauthorized(ref msg) => {
                warn!(error_msg = %msg, "Unauthorized access attempt");
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            Error::Forbidden(ref msg) => {
                warn!(error_msg = %msg, "Forbidden access attempt");
                (StatusCode::FORBIDDEN, msg.clone())
            }
            Error::Conflict(ref msg) => {
                warn!(error_msg = %msg, "Conflict");
                (StatusCode::CONFLICT, msg.clone())
            }
            Error::Internal => {
                error!("Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error".to_string(),
                )
            }
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
