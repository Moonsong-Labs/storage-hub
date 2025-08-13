//! SQLite database connection module for StorageHub backend
//!
//! This module provides SQLite database connectivity using diesel-async's
//! SyncConnectionWrapper with connection pooling via bb8.

pub mod connection;
pub mod sqlite_connection;

// Re-export common types for convenience
pub use connection::{AsyncSqliteConnection, DbConfig, DbConnection, DbConnectionError};
pub use sqlite_connection::SqliteConnection;