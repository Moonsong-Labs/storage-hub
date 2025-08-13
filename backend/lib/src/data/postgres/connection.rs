//! Database connection abstraction for database backends

use std::fmt::Debug;

use async_trait::async_trait;
use diesel::QueryResult;
use diesel_async::AsyncConnection;

/// Trait representing a database connection abstraction
#[async_trait]
pub trait DbConnection: Send + Sync + Debug {
    /// Type representing the actual connection implementation
    type Connection: AsyncConnection + Send + 'static;

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
use crate::data::sqlite::sqlite_connection::SqliteConnection;
// WIP: Mock connection import - commented out until diesel traits are fully implemented
// #[cfg(feature = "mocks")]
// use super::mock_connection::MockDbConnection;

/// Enum wrapper for different database connection implementations
///
/// This enum allows using concrete types instead of trait objects,
/// solving trait object safety issues while maintaining flexibility
/// between different database backends and mock connections.
#[derive(Debug)]
pub enum AnyDbConnection {
    /// PostgreSQL connection
    Postgres(PgConnection),
    /// SQLite connection
    Sqlite(SqliteConnection),
    // WIP: Mock connection variant - commented out until diesel traits are fully implemented
    // #[cfg(feature = "mocks")]
    // Mock(MockDbConnection),
}

impl AnyDbConnection {
    /// Get the backend type for this connection
    pub fn backend_type(&self) -> crate::data::any_connection::BackendType {
        match self {
            AnyDbConnection::Postgres(_) => crate::data::any_connection::BackendType::Postgres,
            AnyDbConnection::Sqlite(_) => crate::data::any_connection::BackendType::Sqlite,
        }
    }
}

#[async_trait]
impl DbConnection for AnyDbConnection {
    // Use our AnyAsyncConnection enum that delegates to the inner connection
    type Connection = AnyAsyncConnection;

    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError> {
        match self {
            AnyDbConnection::Postgres(conn) => {
                // Get the PostgreSQL connection and wrap it in our enum
                let pg_conn = conn.get_connection().await?;
                Ok(AnyAsyncConnection::Postgres(pg_conn))
            }
            AnyDbConnection::Sqlite(conn) => {
                // Get the SQLite connection and wrap it in our enum
                let sqlite_conn = conn.get_connection().await?;
                Ok(AnyAsyncConnection::Sqlite(sqlite_conn))
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
            AnyDbConnection::Postgres(conn) => conn.test_connection().await,
            AnyDbConnection::Sqlite(conn) => conn.test_connection().await,
            // WIP: Mock connection handling
            // #[cfg(feature = "mocks")]
            // AnyDbConnection::Mock(conn) => conn.test_connection().await,
        }
    }

    async fn is_healthy(&self) -> bool {
        match self {
            AnyDbConnection::Postgres(conn) => conn.is_healthy().await,
            AnyDbConnection::Sqlite(conn) => conn.is_healthy().await,
            // WIP: Mock connection handling
            // #[cfg(feature = "mocks")]
            // AnyDbConnection::Mock(conn) => conn.is_healthy().await,
        }
    }
}

// Type alias for async SQLite connection
type AsyncSqliteConnection = diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>;

/// Enum wrapper for different async connection types
///
/// This enum allows us to switch between different database backends
/// while maintaining the same interface.
pub enum AnyAsyncConnection {
    /// Async PostgreSQL connection
    Postgres(diesel_async::AsyncPgConnection),
    /// Async SQLite connection (using SyncConnectionWrapper)
    Sqlite(AsyncSqliteConnection),
    // WIP: Mock async connection variant - commented out until diesel traits are fully implemented
    // #[cfg(feature = "mocks")]
    // Mock(super::mock_connection::MockAsyncConnection),
}

// Implement Debug manually since AsyncPgConnection doesn't implement Debug
impl std::fmt::Debug for AnyAsyncConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyAsyncConnection::Postgres(_) => f.debug_struct("AnyAsyncConnection::Postgres").finish(),
            AnyAsyncConnection::Sqlite(_) => f.debug_struct("AnyAsyncConnection::Sqlite").finish(),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(_) => f.debug_struct("AnyAsyncConnection::Mock").finish(),
        }
    }
}

impl AnyAsyncConnection {
    /// Execute a raw SQL query that works on both backends
    pub async fn execute_raw_sql(&mut self, sql: &str) -> diesel::QueryResult<usize> {
        use diesel_async::SimpleAsyncConnection;
        
        match self {
            AnyAsyncConnection::Postgres(conn) => {
                conn.batch_execute(sql).await?;
                Ok(0) // batch_execute doesn't return affected rows
            }
            AnyAsyncConnection::Sqlite(conn) => {
                conn.batch_execute(sql).await?;
                Ok(0) // batch_execute doesn't return affected rows
            }
        }
    }
    
    /// Get the backend type as a string
    pub fn backend_name(&self) -> &'static str {
        match self {
            AnyAsyncConnection::Postgres(_) => "PostgreSQL",
            AnyAsyncConnection::Sqlite(_) => "SQLite",
        }
    }
    
    /// Check if this connection uses PostgreSQL backend
    pub fn is_postgres(&self) -> bool {
        matches!(self, AnyAsyncConnection::Postgres(_))
    }
    
    /// Check if this connection uses SQLite backend
    pub fn is_sqlite(&self) -> bool {
        matches!(self, AnyAsyncConnection::Sqlite(_))
    }
    
    /// Get the backend type for this connection
    pub fn backend_type(&self) -> crate::data::any_connection::BackendType {
        match self {
            AnyAsyncConnection::Postgres(_) => crate::data::any_connection::BackendType::Postgres,
            AnyAsyncConnection::Sqlite(_) => crate::data::any_connection::BackendType::Sqlite,
        }
    }
}

// Implement SimpleAsyncConnection by delegating to the inner connection
#[async_trait]
impl diesel_async::SimpleAsyncConnection for AnyAsyncConnection {
    async fn batch_execute(&mut self, query: &str) -> diesel::QueryResult<()> {
        match self {
            AnyAsyncConnection::Postgres(conn) => conn.batch_execute(query).await,
            AnyAsyncConnection::Sqlite(conn) => conn.batch_execute(query).await,
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.batch_execute(query).await,
        }
    }
}

// Note: We cannot implement AsyncConnection for AnyAsyncConnection directly
// because PostgreSQL and SQLite have different Backend types.
// However, we can provide backend-specific operations through delegation.
// For now, this implementation assumes PostgreSQL backend for compatibility,
// but future work should introduce AnyBackend to properly abstract over both.
#[async_trait]
impl diesel_async::AsyncConnection for AnyAsyncConnection {
    // FIXME: This currently hardcodes PostgreSQL backend
    // TODO: Implement AnyBackend to properly support both PostgreSQL and SQLite
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
            AnyAsyncConnection::Postgres(conn) => conn.load(source),
            AnyAsyncConnection::Sqlite(_) => panic!("Cannot use SQLite connection with PostgreSQL queries"),
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
            AnyAsyncConnection::Postgres(conn) => conn.execute_returning_count(source),
            AnyAsyncConnection::Sqlite(_) => panic!("Cannot use SQLite connection with PostgreSQL queries"),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.execute_returning_count(source),
        }
    }

    fn transaction_state(&mut self) -> &mut <Self::TransactionManager as diesel_async::TransactionManager<Self>>::TransactionStateData{
        match self {
            AnyAsyncConnection::Postgres(conn) => conn.transaction_state(),
            AnyAsyncConnection::Sqlite(_) => panic!("Cannot use SQLite connection with PostgreSQL queries"),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.transaction_state(),
        }
    }

    fn instrumentation(&mut self) -> &mut dyn diesel::connection::Instrumentation {
        match self {
            AnyAsyncConnection::Postgres(conn) => conn.instrumentation(),
            AnyAsyncConnection::Sqlite(_) => panic!("Cannot use SQLite connection with PostgreSQL queries"),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.instrumentation(),
        }
    }

    fn set_instrumentation(&mut self, instrumentation: impl diesel::connection::Instrumentation) {
        match self {
            AnyAsyncConnection::Postgres(conn) => conn.set_instrumentation(instrumentation),
            AnyAsyncConnection::Sqlite(_) => panic!("Cannot use SQLite connection with PostgreSQL queries"),
            // #[cfg(feature = "mocks")]
            // AnyAsyncConnection::Mock(conn) => conn.set_instrumentation(instrumentation),
        }
    }
}
