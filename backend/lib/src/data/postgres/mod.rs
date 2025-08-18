//! PostgreSQL data access module
//!
//! This module provides read-only access to the StorageHub indexer database,
//! allowing the backend to query blockchain-indexed data.

pub mod client;
pub mod queries;

pub use client::{PostgresClient, PostgresError};
