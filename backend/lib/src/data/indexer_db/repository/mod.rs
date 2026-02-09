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
use bigdecimal::BigDecimal;

use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainBspId, OnchainMspId,
};
use shp_types::Hash;

pub mod error;
pub mod pool;
pub mod postgres;

use error::RepositoryResult;

/// Represents the different types of payment streams
#[derive(Debug, Clone)]
pub enum PaymentStreamKind {
    Fixed { rate: BigDecimal },
    Dynamic { amount_provided: BigDecimal },
}

/// Payment stream data from the database
#[derive(Debug, Clone)]
pub struct PaymentStreamData {
    pub provider: String,
    pub total_amount_paid: BigDecimal,
    pub kind: PaymentStreamKind,
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
    async fn get_msp_by_onchain_id(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<Msp>;

    /// Retrieve the information of the given bucket
    ///
    /// # Arguments
    /// * `bucket` - the Bucket ID (onchain)
    async fn get_bucket_by_onchain_id(&self, bucket: &Hash) -> RepositoryResult<Bucket>;

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

    /// Retrieve the file identified with the given File Key
    ///
    /// There can be multiple file records for a given file key if there were multiple
    /// storage requests for the same file key. We get the oldest one created, which
    /// would be the original storage request that first created the file.
    /// This is good enough for the purpose of this query.
    ///
    /// # Arguments
    /// * `key` - the File Key to search
    async fn get_file_by_file_key(&self, file_key: &Hash) -> RepositoryResult<File>;

    /// Get all payment streams for a user account
    ///
    /// # Arguments
    /// * `user_account` - The user's account address
    ///
    /// # Returns
    /// * Vector of payment stream data
    async fn get_payment_streams_for_user(
        &self,
        user_account: &str,
    ) -> RepositoryResult<Vec<PaymentStreamData>>;

    /// Get the number of files stored by the given MSP
    ///
    /// # Arguments
    /// * `msp` - The on-chain MSP ID
    ///
    /// # Returns
    /// * The number of files stored by that MSP
    async fn get_number_of_files_stored_by_msp(
        &self,
        onchain_msp_id: &OnchainMspId,
    ) -> RepositoryResult<u64>;
}

/// Mutable operations for test environments.
///
/// This trait extends `IndexerOps` with write operations,
/// ensuring they are only available in test environments.
///
/// ## Implementation Notes
/// - All methods are async and return `RepositoryResult<T>`
/// - Methods follow consistent naming: `create_*`, `delete_*`
/// - This trait always exists but implementations are conditional
#[async_trait]
pub trait IndexerOpsMut: IndexerOps {
    /// Create a new MSP.
    ///
    /// # Arguments
    /// * `account` - Account address of the MSP
    /// * `onchain_msp_id` - Onchain identifier for the MSP
    async fn create_msp(
        &self,
        account: &str,
        onchain_msp_id: OnchainMspId,
    ) -> RepositoryResult<Msp>;

    /// Delete an MSP by its onchain ID.
    async fn delete_msp(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<()>;

    /// Create a new BSP.
    ///
    /// # Arguments
    /// * `account` - Account address of the BSP
    /// * `onchain_bsp_id` - Onchain identifier for the BSP
    /// * `capacity` - Storage capacity
    /// * `stake` - Staked amount
    async fn create_bsp(
        &self,
        account: &str,
        onchain_bsp_id: OnchainBspId,
        capacity: BigDecimal,
        stake: BigDecimal,
    ) -> RepositoryResult<Bsp>;

    /// Delete a BSP by its onchain ID.
    async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> RepositoryResult<()>;

    /// Create a new bucket.
    ///
    /// # Arguments
    /// * `account` - Owner account address
    /// * `msp_id` - Optional MSP database ID
    /// * `name` - Bucket name
    /// * `onchain_bucket_id` - Onchain bucket identifier
    /// * `private` - Privacy flag
    async fn create_bucket(
        &self,
        account: &str,
        msp_id: Option<i64>,
        name: &[u8],
        onchain_bucket_id: &Hash,
        private: bool,
    ) -> RepositoryResult<Bucket>;

    /// Delete a bucket by its onchain ID.
    async fn delete_bucket(&self, onchain_bucket_id: &Hash) -> RepositoryResult<()>;

    /// Create a new file.
    ///
    /// # Arguments
    /// * `account` - Owner account
    /// * `file_key` - Unique file key
    /// * `bucket_id` - Database ID of the bucket
    /// * `onchain_bucket_id` - Onchain bucket identifier
    /// * `location` - File location/path
    /// * `fingerprint` - File fingerprint
    /// * `size` - File size in bytes
    #[allow(clippy::too_many_arguments)]
    async fn create_file(
        &self,
        account: &[u8],
        file_key: &Hash,
        bucket_id: i64,
        onchain_bucket_id: &Hash,
        location: &[u8],
        fingerprint: &[u8],
        size: i64,
    ) -> RepositoryResult<File>;

    /// Delete a file by its file key.
    async fn delete_file(&self, file_key: &Hash) -> RepositoryResult<()>;

    /// Create a new payment stream.
    ///
    /// # Arguments
    /// * `user_account` - User account address
    /// * `provider` - Provider address
    /// * `total_amount_paid` - Total amount paid in the stream
    /// * `kind` - Payment stream kind (Fixed or Dynamic)
    async fn create_payment_stream(
        &self,
        user_account: &str,
        provider: &str,
        total_amount_paid: BigDecimal,
        kind: PaymentStreamKind,
    ) -> RepositoryResult<PaymentStreamData>;
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
