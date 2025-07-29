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

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use shc_indexer_db::models::{Bucket, File, FileStorageRequestStep, Msp};

use crate::error::Result;

// Main client
pub use client::{PostgresClient, PostgresError};
// Connection types
pub use connection::{
    AnyDbConnection, ConnectionProvider, DbConfig, DbConnection, DbConnectionError,
};
pub use pg_connection::PgConnection;
// WIP: Mock types - commented out until diesel traits are fully implemented
// #[cfg(feature = "mocks")]
// pub use mock_connection::{MockDbConnection, MockErrorConfig, MockTestData};

/// Pagination parameters for database queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Number of items to return
    pub limit: Option<i64>,
    /// Number of items to skip
    pub offset: Option<i64>,
}

/// Trait defining PostgreSQL client operations
///
/// This trait allows for mock implementations during testing
/// while maintaining the same interface as the real PostgreSQL client.
#[async_trait]
pub trait PostgresClientTrait: Send + Sync {
    /// Test the database connection
    async fn test_connection(&self) -> Result<()>;

    /// Get a file by its key
    async fn get_file_by_key(&self, file_key: &[u8]) -> Result<File>;

    /// Get all files for a user
    async fn get_files_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>>;

    /// Get files for a user stored by a specific MSP
    async fn get_files_by_user_and_msp(
        &self,
        user_account: &[u8],
        msp_id: i64,
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>>;

    /// Get all files in a bucket
    async fn get_files_by_bucket_id(
        &self,
        bucket_id: i64,
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<File>>;

    /// Create a new file record
    async fn create_file(&self, file: File) -> Result<File>;

    /// Update file storage step
    async fn update_file_step(&self, file_key: &[u8], step: FileStorageRequestStep) -> Result<()>;

    /// Delete a file record
    async fn delete_file(&self, file_key: &[u8]) -> Result<()>;

    /// Get a bucket by its ID
    async fn get_bucket_by_id(&self, bucket_id: i64) -> Result<Bucket>;

    /// Get all buckets for a user
    async fn get_buckets_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<PaginationParams>,
    ) -> Result<Vec<Bucket>>;

    /// Get an MSP by its ID
    async fn get_msp_by_id(&self, msp_id: i64) -> Result<Msp>;

    /// Get all MSPs
    async fn get_all_msps(&self, pagination: Option<PaginationParams>) -> Result<Vec<Msp>>;

    /// Execute a raw SQL query (for advanced use cases)
    async fn execute_raw_query(&self, query: &str) -> Result<Vec<serde_json::Value>>;
}
