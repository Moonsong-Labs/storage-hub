//! Database connection abstraction for PostgreSQL
//!
//! This module defines the `DbConnection` trait that abstracts database operations,
//! allowing for both real PostgreSQL connections and mock implementations for testing.

use async_trait::async_trait;
use diesel::result::Error as DieselError;
use diesel::QueryResult;
use diesel_async::AsyncConnection;
use std::fmt::Debug;

/// Trait representing a database connection abstraction
///
/// This trait provides a generic interface for database operations that can be
/// implemented by both real PostgreSQL connections and mock connections for testing.
/// It is designed to work with diesel-async for asynchronous database operations.
#[async_trait]
pub trait DbConnection: Send + Sync + Debug {
    /// Type representing the actual connection implementation
    type Connection: AsyncConnection<Backend = diesel::pg::Pg> + Send + 'static;

    /// Get a connection from the pool
    ///
    /// # Returns
    /// A connection that can be used to execute database queries
    ///
    /// # Errors
    /// Returns an error if the connection cannot be obtained (e.g., pool exhausted)
    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError>;

    /// Test the database connection
    ///
    /// This method attempts to obtain a connection to verify that the database
    /// is accessible and the connection configuration is valid.
    ///
    /// # Returns
    /// Ok(()) if the connection test succeeds, otherwise an error
    async fn test_connection(&self) -> Result<(), DbConnectionError> {
        let _conn = self.get_connection().await?;
        Ok(())
    }

    /// Execute a transaction with automatic rollback on error
    ///
    /// This method provides a way to execute multiple database operations
    /// within a single transaction. If any operation fails, the entire
    /// transaction is rolled back.
    ///
    /// # Arguments
    /// * `f` - A closure that performs database operations within the transaction
    ///
    /// # Returns
    /// The result of the transaction operations
    async fn transaction<F, R, E>(&self, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Self::Connection) -> Result<R, E> + Send,
        R: Send,
        E: From<DbConnectionError> + From<DieselError> + Send,
    {
        let mut conn = self.get_connection().await?;
        conn.transaction(f).await.map_err(E::from)
    }

    /// Check if the connection pool is healthy
    ///
    /// This method can be used for health checks to ensure the database
    /// connection pool is functioning properly.
    ///
    /// # Returns
    /// True if the pool is healthy, false otherwise
    async fn is_healthy(&self) -> bool {
        self.test_connection().await.is_ok()
    }
}

/// Errors that can occur during database connection operations
#[derive(Debug, thiserror::Error)]
pub enum DbConnectionError {
    /// Connection pool error (e.g., timeout, exhausted)
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// Configuration error (e.g., invalid connection string)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Generic database error
    #[error("Database error: {0}")]
    Database(String),
}

/// Extension trait for converting query results
///
/// This trait provides convenience methods for handling query results
/// and converting them to our application's Result type.
#[async_trait]
pub trait QueryResultExt<T> {
    /// Convert a QueryResult to our application Result type
    async fn to_result(self) -> Result<T, DbConnectionError>;
}

#[async_trait]
impl<T: Send> QueryResultExt<T> for QueryResult<T> {
    async fn to_result(self) -> Result<T, DbConnectionError> {
        self.map_err(|e| DbConnectionError::Database(e.to_string()))
    }
}

/// Configuration for database connections
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// Database connection URL
    pub database_url: String,
    
    /// Maximum number of connections in the pool
    pub max_connections: Option<u32>,
    
    /// Connection timeout in seconds
    pub connection_timeout: Option<u64>,
    
    /// Idle timeout in seconds
    pub idle_timeout: Option<u64>,
    
    /// Maximum lifetime of a connection in seconds
    pub max_lifetime: Option<u64>,
}

impl DbConfig {
    /// Create a new database configuration with default settings
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            max_connections: None,
            connection_timeout: None,
            idle_timeout: None,
            max_lifetime: None,
        }
    }

    /// Set the maximum number of connections in the pool
    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    /// Set the connection timeout in seconds
    pub fn with_connection_timeout(mut self, timeout: u64) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// Set the idle timeout in seconds
    pub fn with_idle_timeout(mut self, timeout: u64) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Set the maximum lifetime of a connection in seconds
    pub fn with_max_lifetime(mut self, lifetime: u64) -> Self {
        self.max_lifetime = Some(lifetime);
        self
    }
}

/// Trait for types that can provide a database connection
///
/// This trait is useful for dependency injection, allowing services
/// to accept any type that can provide a database connection.
#[async_trait]
pub trait ConnectionProvider: Send + Sync {
    /// The type of connection this provider returns
    type DbConn: DbConnection;

    /// Get the database connection
    fn get_db_connection(&self) -> &Self::DbConn;
}