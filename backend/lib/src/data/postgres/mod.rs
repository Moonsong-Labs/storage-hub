//! PostgreSQL data access module
//!
//! This module provides read-only access to the StorageHub indexer database,
//! allowing the backend to query blockchain-indexed data.

pub mod client;
pub mod connection;
pub mod pg_connection;
pub mod queries;

// WIP: Mock connection implementation - commented out until diesel traits are fully implemented
// #[cfg(feature = "mocks")]
// pub mod mock_connection;

// WIP: Mock types - commented out until diesel traits are fully implemented
// #[cfg(feature = "mocks")]
// pub use mock_connection::{MockDbConnection, MockErrorConfig, MockTestData};
pub use client::{PostgresClient, PostgresError};
pub use connection::{
    AnyDbConnection, ConnectionProvider, DbConfig, DbConnection, DbConnectionError,
};
pub use pg_connection::PgConnection;
