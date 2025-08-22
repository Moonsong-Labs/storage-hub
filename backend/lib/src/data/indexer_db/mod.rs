//! Indexer database access module using repository pattern
//!
//! This module provides database access through a repository abstraction,
//! allowing the backend to query blockchain-indexed data with support for
//! both production PostgreSQL and mock implementations.

pub mod db_client;
pub mod repository;

pub use db_client::DBClient;
pub use repository::{error::RepositoryError, pool::SmartPool, postgres::Repository};

#[cfg(feature = "mocks")]
pub use repository::mock::MockRepository;
