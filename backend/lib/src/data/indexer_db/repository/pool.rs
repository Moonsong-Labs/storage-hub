//! SmartPool implementation for automatic test transaction management.
//!
//! ## Key Components
//! - [`SmartPool`] - Connection pool with automatic test transaction support
//!
//! ## Features
//! - Automatic test transactions in test mode (single connection)
//! - Normal pooling in production mode (32 connections)
//! - Zero runtime overhead (test code compiled out in release)

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(test)]
use diesel_async::AsyncConnection;
use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};

use super::error::RepositoryError;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> =
    diesel_async::pooled_connection::bb8::PooledConnection<'a, AsyncPgConnection>;

/// Smart connection pool that automatically manages test transactions.
///
/// In test mode:
/// - Uses single connection to enable test transactions
/// - Automatically begins test transaction on first connection
/// - Transaction automatically rolls back when test ends
///
/// In production mode:
/// - Uses normal connection pooling with 32 connections
/// - No test transaction overhead
pub struct SmartPool {
    /// The underlying bb8 pool
    inner: Arc<DbPool>,

    /// Track whether test transaction has been initialized (test mode only)
    #[cfg(test)]
    test_tx_initialized: AtomicBool,
}

impl SmartPool {
    /// Create a new SmartPool with the given database URL.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, RepositoryError>` - The configured pool or error
    pub async fn new(database_url: &str) -> Result<Self, RepositoryError> {
        // Create the connection manager
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        // Configure pool based on compile mode
        #[cfg(test)]
        let pool = {
            // Single connection for test transactions
            Pool::builder()
                .max_size(1)
                .build(manager)
                .await
                .map_err(|e| RepositoryError::Pool(format!("Failed to create test pool: {}", e)))?
        };

        #[cfg(not(test))]
        let pool = {
            // Normal pool size for production
            Pool::builder()
                .max_size(32)
                .build(manager)
                .await
                .map_err(|e| {
                    RepositoryError::Pool(format!("Failed to create production pool: {}", e))
                })?
        };

        Ok(Self {
            inner: Arc::new(pool),
            #[cfg(test)]
            test_tx_initialized: AtomicBool::new(false),
        })
    }

    /// Get a connection from the pool.
    ///
    /// In test mode, this will automatically begin a test transaction
    /// on the first call, which will be rolled back when the test ends.
    ///
    /// # Returns
    /// * `Result<DbConnection, RepositoryError>` - Database connection or error
    pub async fn get(&self) -> Result<DbConnection<'_>, RepositoryError> {
        // Get connection from pool
        #[allow(unused_mut)]
        let mut conn = self
            .inner
            .get()
            .await
            .map_err(|e| RepositoryError::Pool(format!("Failed to get connection: {}", e)))?;

        // Begin test transaction if in test mode and not yet initialized
        #[cfg(test)]
        {
            if !self.test_tx_initialized.load(Ordering::Acquire) {
                // Begin test transaction that will rollback automatically
                conn.begin_test_transaction()
                    .await
                    .map_err(RepositoryError::Database)?;

                // Mark as initialized
                self.test_tx_initialized.store(true, Ordering::Release);
            }
        }

        Ok(conn)
    }

    /// Get the maximum size of the connection pool.
    ///
    /// Returns 1 in test mode, 32 in production mode.
    pub fn max_size(&self) -> usize {
        #[cfg(test)]
        return 1;

        #[cfg(not(test))]
        return 32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual database connection
    async fn test_pool_creation() {
        // This test will fail without a valid database URL
        // It's mainly to verify compilation
        let result = SmartPool::new("postgres://invalid:invalid@localhost/invalid").await;
        assert!(result.is_err());
    }
}
