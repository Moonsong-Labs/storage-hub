//! PostgreSQL data access module
//!
//! This module provides read-only access to the StorageHub indexer database,
//! allowing the backend to query blockchain-indexed data.

pub mod client;
// pub mod queries; // TODO: Fix compilation errors in queries module

pub use client::PostgresClient;
