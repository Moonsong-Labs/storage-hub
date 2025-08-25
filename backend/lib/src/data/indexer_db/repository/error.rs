//! Error types for repository operations.
//!
//! ## Key Components
//! - [`RepositoryError`] - Main error type for all repository operations
//!
//! ## Error Categories
//! - Database errors from diesel operations
//! - Connection pool errors
//! - Not found errors for missing entities

use thiserror::Error;

/// Main error type for repository operations.
///
/// This error type encompasses all possible failures that can occur
/// when interacting with the database through the repository pattern.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Database operation error from diesel
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    /// Connection pool error
    #[error("Pool error: {0}")]
    Pool(String),

    /// Entity not found error
    #[error("Not found: {entity}")]
    NotFound {
        /// The type of entity that was not found
        entity: String,
    },

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),
}

impl RepositoryError {
    /// Create a new NotFound error for the given entity type.
    ///
    /// # Arguments
    /// * `entity` - The type of entity that was not found (e.g., "BSP", "Bucket", "File")
    pub fn not_found(entity: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
        }
    }

    /// Create a new InvalidInput error with the given message.
    ///
    /// # Arguments
    /// * `msg` - Description of what input was invalid
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a new Configuration error with the given message.
    ///
    /// # Arguments
    /// * `msg` - Description of the configuration problem
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a new Transaction error with the given message.
    ///
    /// # Arguments
    /// * `msg` - Description of the transaction problem
    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    /// Check if this error represents a not found condition.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Check if this error is due to a database constraint violation.
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self,
            Self::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) | Self::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            ))
        )
    }
}

/// Type alias for Results that may fail with RepositoryError
pub type RepositoryResult<T> = Result<T, RepositoryError>;
