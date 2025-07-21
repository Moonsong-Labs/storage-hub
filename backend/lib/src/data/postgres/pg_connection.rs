//! Real PostgreSQL connection implementation using diesel-async and bb8
//!
//! This module provides a production-ready PostgreSQL connection pool implementation
//! that implements the `DbConnection` trait for use in the StorageHub backend.

use super::connection::{DbConfig, DbConnection, DbConnectionError};
use async_trait::async_trait;
use diesel_async::{
    pooled_connection::{bb8, AsyncDieselConnectionManager, PoolError},
    AsyncPgConnection,
};
use std::fmt::Debug;
use std::time::Duration;

/// Real PostgreSQL connection pool implementation
///
/// This struct wraps a bb8 connection pool with AsyncPgConnection instances,
/// providing efficient connection management for production use.
pub struct PgConnection {
    pool: bb8::Pool<AsyncPgConnection>,
}

impl PgConnection {
    /// Create a new PostgreSQL connection pool
    ///
    /// # Arguments
    /// * `config` - Database configuration containing connection parameters
    ///
    /// # Returns
    /// A new PgConnection instance or an error if pool creation fails
    pub async fn new(config: DbConfig) -> Result<Self, DbConnectionError> {
        // Create the connection manager
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&config.database_url);

        // Configure the pool builder
        let mut builder = bb8::Pool::builder();

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
        let pool = builder
            .build(manager)
            .await
            .map_err(|e| DbConnectionError::Config(format!("Failed to create connection pool: {}", e)))?;

        // Test the connection to ensure configuration is valid
        let conn = Self { pool };
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
    pub fn max_size(&self) -> u32 {
        self.pool.max_size()
    }
}

impl Debug for PgConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.pool.state();
        f.debug_struct("PgConnection")
            .field("max_size", &self.pool.max_size())
            .field("connections", &state.connections)
            .field("idle_connections", &state.idle_connections)
            .finish()
    }
}

#[async_trait]
impl DbConnection for PgConnection {
    type Connection = AsyncPgConnection;

    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError> {
        self.pool
            .get()
            .await
            .map(|conn| conn.into_inner())
            .map_err(|e| match e {
                PoolError::Timeout => DbConnectionError::Pool("Connection pool timeout".to_string()),
                PoolError::Inner(e) => DbConnectionError::Database(format!("Database connection error: {}", e)),
                PoolError::Closed => DbConnectionError::Pool("Connection pool is closed".to_string()),
                _ => DbConnectionError::Pool(format!("Pool error: {}", e)),
            })
    }

    async fn test_connection(&self) -> Result<(), DbConnectionError> {
        // Get a connection from the pool
        let mut conn = self.get_connection().await?;
        
        // Execute a simple query to verify the connection works
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(|e| DbConnectionError::Database(format!("Connection test failed: {}", e)))?;
            
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        // Check if we can get a connection and the pool is not closed
        if self.pool.state().connections == 0 && self.pool.max_size() > 0 {
            // Pool has no connections but should have some
            return false;
        }
        
        // Try to actually test the connection
        self.test_connection().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_db_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/test_db".to_string())
    }

    #[tokio::test]
    #[ignore = "Requires a running PostgreSQL instance"]
    async fn test_pg_connection_creation() {
        let config = DbConfig::new(get_test_db_url())
            .with_max_connections(5)
            .with_connection_timeout(10);

        let result = PgConnection::new(config).await;
        assert!(result.is_ok(), "Failed to create PgConnection: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "Requires a running PostgreSQL instance"]
    async fn test_get_connection() {
        let config = DbConfig::new(get_test_db_url());
        let pg_conn = PgConnection::new(config).await.expect("Failed to create connection");

        let conn = pg_conn.get_connection().await;
        assert!(conn.is_ok(), "Failed to get connection: {:?}", conn.err());
    }

    #[tokio::test]
    #[ignore = "Requires a running PostgreSQL instance"]
    async fn test_connection_health_check() {
        let config = DbConfig::new(get_test_db_url());
        let pg_conn = PgConnection::new(config).await.expect("Failed to create connection");

        assert!(pg_conn.is_healthy().await, "Connection should be healthy");
    }

    #[tokio::test]
    #[ignore = "Requires a running PostgreSQL instance"]
    async fn test_pool_state() {
        let config = DbConfig::new(get_test_db_url()).with_max_connections(3);
        let pg_conn = PgConnection::new(config).await.expect("Failed to create connection");

        let (total, idle) = pg_conn.pool_state();
        assert!(total <= 3, "Total connections should not exceed max");
        assert!(idle <= total, "Idle connections should not exceed total");
    }

    #[tokio::test]
    async fn test_invalid_connection_string() {
        let config = DbConfig::new("invalid://connection/string");
        let result = PgConnection::new(config).await;
        
        assert!(result.is_err(), "Should fail with invalid connection string");
        match result.err() {
            Some(DbConnectionError::Config(_)) | Some(DbConnectionError::Database(_)) => {},
            _ => panic!("Expected Config or Database error"),
        }
    }

    #[tokio::test]
    #[ignore = "Requires a running PostgreSQL instance"]
    async fn test_transaction() {
        let config = DbConfig::new(get_test_db_url());
        let pg_conn = PgConnection::new(config).await.expect("Failed to create connection");

        // Test a simple transaction
        let result: Result<i32, DbConnectionError> = pg_conn.transaction(|_conn| {
            // In a real scenario, you would perform database operations here
            Ok(42)
        }).await;

        assert_eq!(result.unwrap(), 42);
    }
}