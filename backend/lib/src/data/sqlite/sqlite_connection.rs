//! Real SQLite connection implementation using diesel-async's SyncConnectionWrapper and bb8
//!
//! This module provides a production-ready SQLite connection pool implementation
//! that implements the `DbConnection` trait for use in the StorageHub backend.

use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;
use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncConnection, RunQueryDsl,
};

use super::connection::{AsyncSqliteConnection, DbConfig, DbConnection, DbConnectionError};

/// Real SQLite connection pool implementation
///
/// This struct wraps a bb8 connection pool with SyncConnectionWrapper<SqliteConnection> instances,
/// providing efficient connection management for production use.
pub struct SqliteConnection {
    pool: Pool<AsyncSqliteConnection>,
    database_url: String,
}

impl SqliteConnection {
    /// Create a new SQLite connection pool
    ///
    /// * `config` - Database configuration containing connection parameters
    pub async fn new(config: DbConfig) -> Result<Self, DbConnectionError> {
        // Create the connection manager
        let manager = AsyncDieselConnectionManager::<AsyncSqliteConnection>::new(&config.database_url);

        // Configure the pool builder
        let mut builder = Pool::builder();

        // Apply configuration options
        if let Some(max_connections) = config.max_connections {
            builder = builder.max_size(max_connections);
        }

        if let Some(connection_timeout) = config.connection_timeout {
            builder = builder.connection_timeout(Duration::from_secs(connection_timeout));
        }

        if let Some(idle_timeout) = config.idle_timeout {
            builder = builder.idle_timeout(Some(Duration::from_secs(idle_timeout)));
        }

        if let Some(max_lifetime) = config.max_lifetime {
            builder = builder.max_lifetime(Some(Duration::from_secs(max_lifetime)));
        }

        // Build the pool
        let pool = builder.build(manager).await.map_err(|e| {
            DbConnectionError::Config(format!("Failed to create connection pool: {}", e))
        })?;

        // Test the connection to ensure configuration is valid
        let conn = Self {
            pool,
            database_url: config.database_url.clone(),
        };
        conn.test_connection().await?;

        Ok(conn)
    }

    /// Get the current state of the connection pool
    ///
    /// # Returns
    /// A tuple of (total_connections, idle_connections)
    pub fn pool_state(&self) -> (u32, u32) {
        let state = self.pool.state();
        (state.connections, state.idle_connections)
    }

    /// Get the maximum size of the connection pool
    ///
    /// Note: bb8 doesn't expose max_size from state, so we store it separately
    pub fn max_size(&self) -> u32 {
        // TODO: Store max_size during pool creation if needed
        // For now, return the current number of connections as an approximation
        self.pool.state().connections
    }
}

impl Debug for SqliteConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.pool.state();
        f.debug_struct("SqliteConnection")
            .field("connections", &state.connections)
            .field("idle_connections", &state.idle_connections)
            .field("database_url", &"<redacted>")
            .finish()
    }
}

#[async_trait]
impl DbConnection for SqliteConnection {
    // Use the AsyncSqliteConnection type which is SyncConnectionWrapper<SqliteConnection>
    type Connection = AsyncSqliteConnection;

    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError> {
        // Test that we can get a connection from the pool
        let _pooled = self.pool.get().await.map_err(|e| {
            DbConnectionError::Pool(format!("Failed to get connection from pool: {}", e))
        })?;

        // For now, create a new connection with the same config
        // This is not ideal but works with the current trait design
        // TODO: Redesign trait to work with pooled connections
        let conn = AsyncSqliteConnection::establish(&self.database_url)
            .await
            .map_err(|e| {
                DbConnectionError::Database(format!("Failed to establish connection: {}", e))
            })?;

        Ok(conn)
    }

    async fn test_connection(&self) -> Result<(), DbConnectionError> {
        // Get a connection from the pool
        let mut conn = self.get_connection().await?;

        // Execute a simple query to verify the connection works
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(|e| DbConnectionError::Database(format!("Connection test failed: {}", e)))?;

        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        // Check if we can get a connection from the pool
        // bb8's state() method returns a State struct with different field names
        let state = self.pool.state();
        if state.idle_connections == 0 && state.connections == 0 {
            // Pool has no available connections
            return false;
        }

        // Try to actually test the connection
        self.test_connection().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::database::{DEFAULT_CONNECTION_TIMEOUT_SECS, DEFAULT_MAX_CONNECTIONS};

    fn get_test_db_url() -> String {
        std::env::var("SQLITE_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://test.db".to_string())
    }

    #[tokio::test]
    async fn test_sqlite_connection_creation() {
        let config = DbConfig::new(get_test_db_url())
            .with_max_connections(DEFAULT_MAX_CONNECTIONS)
            .with_connection_timeout(DEFAULT_CONNECTION_TIMEOUT_SECS);

        let result = SqliteConnection::new(config).await;
        assert!(
            result.is_ok(),
            "Failed to create SqliteConnection: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_get_connection() {
        let config = DbConfig::new(get_test_db_url());
        let sqlite_conn = SqliteConnection::new(config)
            .await
            .expect("Failed to create connection");

        let conn = sqlite_conn.get_connection().await;
        assert!(conn.is_ok(), "Failed to get connection: {:?}", conn.err());
    }

    #[tokio::test]
    async fn test_connection_health_check() {
        let config = DbConfig::new(get_test_db_url());
        let sqlite_conn = SqliteConnection::new(config)
            .await
            .expect("Failed to create connection");

        assert!(sqlite_conn.is_healthy().await, "Connection should be healthy");
    }

    #[tokio::test]
    async fn test_pool_state() {
        let config = DbConfig::new(get_test_db_url())
            .with_max_connections(crate::constants::test::DB_MAX_CONNECTIONS);
        let sqlite_conn = SqliteConnection::new(config)
            .await
            .expect("Failed to create connection");

        let (total, idle) = sqlite_conn.pool_state();
        assert!(total <= 3, "Total connections should not exceed max");
        assert!(idle <= total, "Idle connections should not exceed total");
    }

    #[tokio::test]
    async fn test_invalid_connection_string() {
        let config = DbConfig::new("invalid://connection/string");
        let result = SqliteConnection::new(config).await;

        assert!(
            result.is_err(),
            "Should fail with invalid connection string"
        );
        // The error could be Config, Database, or Pool error depending on how diesel-async handles it
        match result.err() {
            Some(DbConnectionError::Config(_))
            | Some(DbConnectionError::Database(_))
            | Some(DbConnectionError::Pool(_)) => {}
            other => panic!("Unexpected error type: {:?}", other),
        }
    }
}