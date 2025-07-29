//! Database connection abstraction for PostgreSQL
//!
//! This module defines the `DbConnection` trait that abstracts database operations,
//! allowing for both real PostgreSQL connections and mock implementations for testing.

use std::fmt::Debug;

use async_trait::async_trait;
use diesel::QueryResult;
use diesel_async::AsyncConnection;

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

    // WIP: Transaction method is temporarily removed because diesel-async requires
    // an async closure that returns a Future, not a sync closure returning Result.
    // This would require a major redesign of the trait interface.
    // For now, users should get a connection and use diesel-async's transaction method directly.

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

// Import concrete types for the enum
use super::pg_connection::PgConnection;
// WIP: Mock connection import - commented out until diesel traits are fully implemented
// #[cfg(feature = "mocks")]
// use super::mock_connection::MockDbConnection;

/// Enum wrapper for different database connection implementations
///
/// This enum allows using concrete types instead of trait objects,
/// solving trait object safety issues while maintaining flexibility
/// between real and mock connections.
#[derive(Debug)]
pub enum AnyDbConnection {
    /// Real PostgreSQL connection
    Real(PgConnection),
    // WIP: Mock connection variant - commented out until diesel traits are fully implemented
    // #[cfg(feature = "mocks")]
    // Mock(MockDbConnection),
}

#[async_trait]
impl DbConnection for AnyDbConnection {
    // Use our AnyAsyncConnection enum that delegates to the inner connection
    type Connection = AnyAsyncConnection;

    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError> {
        match self {
            AnyDbConnection::Real(conn) => {
                // Get the real connection and wrap it in our enum
                let real_conn = conn.get_connection().await?;
                Ok(AnyAsyncConnection::Real(real_conn))
            } // WIP: Mock connection handling - commented out until diesel traits are fully implemented
              // #[cfg(feature = "mocks")]
              // AnyDbConnection::Mock(conn) => {
              //     let mock_conn = conn.get_connection().await?;
              //     Ok(AnyAsyncConnection::Mock(mock_conn))
              // }
        }
    }

    async fn test_connection(&self) -> Result<(), DbConnectionError> {
        match self {
            AnyDbConnection::Real(conn) => conn.test_connection().await,
            // WIP: Mock connection handling
            // #[cfg(feature = "mocks")]
            // AnyDbConnection::Mock(conn) => conn.test_connection().await,
        }
    }

    // WIP: Transaction method removed - see trait definition for explanation

    async fn is_healthy(&self) -> bool {
        match self {
            AnyDbConnection::Real(conn) => conn.is_healthy().await,
            // WIP: Mock connection handling
            // #[cfg(feature = "mocks")]
            // AnyDbConnection::Mock(conn) => conn.is_healthy().await,
        }
    }
}

/// Enum wrapper for different async connection types
///
/// This enum allows us to switch between real and mock connections
/// while maintaining the same interface.
pub enum AnyAsyncConnection {
    /// Real async PostgreSQL connection
    Real(diesel_async::AsyncPgConnection),
    // WIP: Mock async connection variant - commented out until diesel traits are fully implemented
    // #[cfg(feature = "mocks")]
    // Mock(super::mock_connection::MockAsyncConnection),
}

// Implement Debug manually since AsyncPgConnection doesn't implement Debug
impl std::fmt::Debug for AnyAsyncConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyAsyncConnection::Real(_) => f.debug_struct("AnyAsyncConnection::Real").finish(),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(_) => f.debug_struct("AnyAsyncConnection::Mock").finish(),
        }
    }
}

// Implement SimpleAsyncConnection by delegating to the inner connection
#[async_trait]
impl diesel_async::SimpleAsyncConnection for AnyAsyncConnection {
    async fn batch_execute(&mut self, query: &str) -> diesel::QueryResult<()> {
        match self {
            AnyAsyncConnection::Real(conn) => conn.batch_execute(query).await,
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.batch_execute(query).await,
        }
    }
}

// Implement AsyncConnection by delegating to the inner connection
#[async_trait]
impl diesel_async::AsyncConnection for AnyAsyncConnection {
    type Backend = diesel::pg::Pg;

    // Use AnsiTransactionManager which is what AsyncPgConnection uses
    type TransactionManager = diesel_async::AnsiTransactionManager;

    // Delegate all associated types to the Real connection's types
    type ExecuteFuture<'conn, 'query>
        = <diesel_async::AsyncPgConnection as diesel_async::AsyncConnection>::ExecuteFuture<
        'conn,
        'query,
    >
    where
        Self: 'conn;

    type LoadFuture<'conn, 'query>
        = <diesel_async::AsyncPgConnection as diesel_async::AsyncConnection>::LoadFuture<
        'conn,
        'query,
    >
    where
        Self: 'conn;

    type Stream<'conn, 'query>
        = <diesel_async::AsyncPgConnection as diesel_async::AsyncConnection>::Stream<'conn, 'query>
    where
        Self: 'conn;

    type Row<'conn, 'query>
        = <diesel_async::AsyncPgConnection as diesel_async::AsyncConnection>::Row<'conn, 'query>
    where
        Self: 'conn;

    async fn establish(_database_url: &str) -> diesel::result::ConnectionResult<Self> {
        // We don't support establishing connections through the enum
        Err(diesel::result::ConnectionError::BadConnection(
            "Cannot establish connection through AnyAsyncConnection".to_string(),
        ))
    }

    fn load<'conn, 'query, T>(&'conn mut self, source: T) -> Self::LoadFuture<'conn, 'query>
    where
        T: diesel::query_builder::AsQuery + 'query,
        T::Query: diesel::query_builder::QueryFragment<Self::Backend>
            + diesel::query_builder::QueryId
            + 'query,
    {
        match self {
            AnyAsyncConnection::Real(conn) => conn.load(source),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.load(source),
        }
    }

    fn execute_returning_count<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> Self::ExecuteFuture<'conn, 'query>
    where
        T: diesel::query_builder::QueryFragment<Self::Backend>
            + diesel::query_builder::QueryId
            + 'query,
    {
        match self {
            AnyAsyncConnection::Real(conn) => conn.execute_returning_count(source),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.execute_returning_count(source),
        }
    }

    fn transaction_state(&mut self) -> &mut <Self::TransactionManager as diesel_async::TransactionManager<Self>>::TransactionStateData{
        match self {
            AnyAsyncConnection::Real(conn) => conn.transaction_state(),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.transaction_state(),
        }
    }

    fn instrumentation(&mut self) -> &mut dyn diesel::connection::Instrumentation {
        match self {
            AnyAsyncConnection::Real(conn) => conn.instrumentation(),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.instrumentation(),
        }
    }

    fn set_instrumentation(&mut self, instrumentation: impl diesel::connection::Instrumentation) {
        match self {
            AnyAsyncConnection::Real(conn) => conn.set_instrumentation(instrumentation),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.set_instrumentation(instrumentation),
        }
    }
}
