//! PostgreSQL client for accessing StorageHub indexer database
//!
//! This module provides a client wrapper around diesel-async connections
//! for querying the existing StorageHub indexer database in a read-only manner.

use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use thiserror::Error;

/// Errors that can occur during PostgreSQL operations
#[derive(Debug, Error)]
pub enum PostgresError {
    /// Connection pool error
    #[error("Connection pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::bb8::RunError),

    /// Database query error
    #[error("Database error: {0}")]
    Query(#[from] diesel::result::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// PostgreSQL client for read-only access to StorageHub indexer database
///
/// This client provides methods to query BSP/MSP information, file metadata,
/// payment streams, and other blockchain-indexed data.
#[derive(Clone)]
pub struct PostgresClient {
    /// Connection pool for async database operations
    pool: Pool<AsyncPgConnection>,
}

impl PostgresClient {
    /// Create a new PostgreSQL client with the given database URL
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Example
    /// ```no_run
    /// # use sh_backend_lib::data::postgres::PostgresClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = PostgresClient::new("postgres://user:pass@localhost/storagehub").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> Result<Self, PostgresError> {
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(config)
            .await
            .map_err(|e| PostgresError::Config(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Test the database connection
    ///
    /// # Returns
    /// Ok(()) if the connection is successful, otherwise an error
    pub async fn test_connection(&self) -> Result<(), PostgresError> {
        let _conn = self.pool.get().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual database
    async fn test_client_creation() {
        let result = PostgresClient::new("postgres://localhost/test").await;
        assert!(result.is_ok());
    }
}