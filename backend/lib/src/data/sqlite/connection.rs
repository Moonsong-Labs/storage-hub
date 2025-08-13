//! SQLite-specific connection types and re-exports
//!
//! This module re-exports the shared database connection traits and provides
//! SQLite-specific type aliases.

// Re-export the shared connection traits from postgres module
pub use crate::data::postgres::connection::{
    ConnectionProvider, DbConfig, DbConnection, DbConnectionError, QueryResultExt,
};

// Type alias for async SQLite connection using SyncConnectionWrapper
pub type AsyncSqliteConnection = diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>;