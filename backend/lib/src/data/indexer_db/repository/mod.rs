//! Repository pattern implementation for database operations.
//!
//! This module provides a clean abstraction over database operations using the repository pattern.
//! It includes automatic test transaction management through SmartPool and comprehensive error handling.
//!
//! ## Key Components
//! - [`SmartPool`] - Connection pool with automatic test transaction support
//! - [`RepositoryError`] - Comprehensive error types for repository operations
//! - [`StorageOperations`] - Trait defining all database operations
//! - [`Repository`] - PostgreSQL implementation
//!
//! ## Architecture
//! The repository pattern provides:
//! - Clean separation between business logic and data access
//! - Automatic test transaction management (rollback after each test)
//! - Type-safe database operations through diesel
//! - Mock repository support for unit testing
//!
//! ## Usage Example
//! ```ignore
//! use repository::{Repository, StorageOperations};
//!
//! let repo = Repository::new(&database_url).await?;
//! let bsp = repo.get_bsp_by_id(1).await?;
//! ```

use async_trait::async_trait;
use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainBspId, OnchainMspId,
};

pub mod error;
pub mod pool;
pub mod postgres;

use error::RepositoryResult;

/// Represents an onchain Bucket ID
///
/// This is used to differentiate between the database id and the onchain id
// TODO: replace with appropriate type from runtime
pub struct BucketId<'a>(pub &'a [u8]);
impl<'a> From<&'a [u8]> for BucketId<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(value)
    }
}

/// Read-only operations for indexer data access.
///
/// This trait provides read-only access to database entities,
/// ensuring that production code cannot accidentally modify data.
///
/// ## Implementation Notes
/// - All methods are async and return `RepositoryResult<T>`
/// - Methods follow consistent naming: `get_*_by_*`, `list_*`
/// - Pagination is supported through `limit` and `offset` parameters
/// - Optional return types indicate entities that may not exist
#[async_trait]
pub trait IndexerOps: Send + Sync {
    /// List BSPs with pagination.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// * Vector of BSPs
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>>;

    /// Retrieve the specified MSP's information given its onchain id
    async fn get_msp_by_onchain_id(&self, msp: &OnchainMspId) -> RepositoryResult<Msp>;

    /// Retrieve the information of the given bucket
    ///
    /// # Arguments
    /// * `bucket` - the Bucket ID (onchain)
    async fn get_bucket_by_onchain_id(&self, bucket: BucketId<'_>) -> RepositoryResult<Bucket>;

    /// List the account's buckets with the given MSP
    ///
    /// # Arguments
    /// * `msp` - the MSP (database) ID where the bucket is held
    /// * `account` - the User account that owns the bucket
    async fn get_buckets_by_user_and_msp(
        &self,
        msp: i64,
        account: &str,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Bucket>>;

    /// Retrieve all the files belonging to the given bucket
    ///
    /// # Arguments
    /// * `bucket` - the Bucket (database) ID to search
    async fn get_files_by_bucket(
        &self,
        bucket: i64,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<File>>;
}

/// Mutable operations for test environments.
///
/// This trait extends `IndexerOps` with write operations,
/// ensuring they are only available in test environments.
///
/// ## Implementation Notes
/// - All methods are async and return `RepositoryResult<T>`
/// - Methods follow consistent naming: `create_*`, `update_*`, `delete_*`
/// - This trait always exists but implementations are conditional
#[async_trait]
pub trait IndexerOpsMut: IndexerOps {
    // TODO(SCAFFOLDING): The methods are for demonstration.
    // Should be replaced with appropriate methods for what needs to be
    // accessed from the indexer's db

    /// Delete a BSP by account.
    ///
    /// # Arguments
    /// * `account` - Account of the BSP to delete
    async fn delete_bsp(&self, account: &OnchainBspId) -> RepositoryResult<()>;
}

// The following trait aliases are so when compiling for unit tests we get access to write operations
// transparently, without changing or using a dedicated client for tests
//
// For non-unit test builds, we explicitly only expose `IndexerOps` as we only want read operations available

#[cfg(not(test))]
pub trait StorageOperations: IndexerOps {}

#[cfg(not(test))]
impl<T: IndexerOps> StorageOperations for T {}

#[cfg(test)]
pub trait StorageOperations: IndexerOps + IndexerOpsMut {}

#[cfg(test)]
impl<T: IndexerOps + IndexerOpsMut> StorageOperations for T {}
