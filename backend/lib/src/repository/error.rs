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
        match self {
            Self::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _
            )) => true,
            Self::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _
            )) => true,
            _ => false,
        }
    }
}

/// Type alias for Results that may fail with RepositoryError
pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let err = RepositoryError::not_found("BSP");
        assert!(err.is_not_found());
        assert_eq!(err.to_string(), "Not found: BSP");
        
        let err = RepositoryError::invalid_input("Invalid capacity");
        assert_eq!(err.to_string(), "Invalid input: Invalid capacity");
        
        let err = RepositoryError::configuration("Missing database URL");
        assert_eq!(err.to_string(), "Configuration error: Missing database URL");
        
        let err = RepositoryError::transaction("Failed to commit");
        assert_eq!(err.to_string(), "Transaction error: Failed to commit");
    }
    
    #[test]
    fn test_is_not_found() {
        let err = RepositoryError::not_found("File");
        assert!(err.is_not_found());
        
        let err = RepositoryError::Pool("Connection failed".to_string());
        assert!(!err.is_not_found());
    }
    
    #[test]
    fn test_constraint_violation_detection() {
        use diesel::result::{DatabaseErrorKind, Error as DieselError};
        
        // Test unique violation detection
        let err = RepositoryError::Database(DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new("duplicate key".to_string())
        ));
        assert!(err.is_constraint_violation());
        
        // Test foreign key violation detection
        let err = RepositoryError::Database(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation,
            Box::new("foreign key violation".to_string())
        ));
        assert!(err.is_constraint_violation());
        
        // Test non-constraint error
        let err = RepositoryError::Database(DieselError::NotFound);
        assert!(!err.is_constraint_violation());
    }
}