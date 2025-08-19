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

pub mod error;
pub mod mock;
pub mod pool;
pub mod postgres;

// Re-export main types for convenience
pub use error::{RepositoryError, RepositoryResult};
pub use mock::MockRepository;
pub use pool::SmartPool;
pub use postgres::Repository;

// Re-export models from shc_indexer_db as our standard
pub use shc_indexer_db::models::{Bsp, Bucket, File};

// ============ Input Types for Creating Records ============

/// Input type for creating a new BSP
#[derive(Debug, Clone)]
pub struct NewBsp {
    pub account: String,
    pub capacity: BigDecimal,
    pub stake: BigDecimal,
    pub onchain_bsp_id: String,
    pub merkle_root: Vec<u8>,
    pub multiaddresses: Vec<Vec<u8>>,
}

/// Input type for creating a new Bucket
#[derive(Debug, Clone)]
pub struct NewBucket {
    pub msp_id: Option<i64>,
    pub account: String,
    pub onchain_bucket_id: Vec<u8>,
    pub name: Vec<u8>,
    pub collection_id: Option<String>,
    pub private: bool,
    pub merkle_root: Vec<u8>,
}

/// Input type for creating a new File
#[derive(Debug, Clone)]
pub struct NewFile {
    pub account: Vec<u8>,
    pub file_key: Vec<u8>,
    pub bucket_id: i64,
    pub location: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub size: i64,
    pub step: i32,
}

// ============ Repository Trait ============

/// Main trait defining all storage operations.
///
/// This trait provides a unified interface for database operations,
/// allowing both production PostgreSQL and mock in-memory implementations.
///
/// ## Implementation Notes
/// - All methods are async and return `RepositoryResult<T>`
/// - Methods follow consistent naming: `create_*`, `get_*_by_*`, `update_*`, `list_*`
/// - Pagination is supported through `limit` and `offset` parameters
/// - Optional return types indicate entities that may not exist
#[async_trait]
pub trait StorageOperations: Send + Sync {
    // ============ BSP Operations ============

    /// Create a new BSP in the database.
    ///
    /// # Arguments
    /// * `new_bsp` - BSP data to insert
    ///
    /// # Returns
    /// * The created BSP with generated ID and timestamps
    async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp>;

    /// Get a BSP by its database ID.
    ///
    /// # Arguments
    /// * `id` - Database ID of the BSP
    ///
    /// # Returns
    /// * `Some(Bsp)` if found, `None` otherwise
    async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>>;

    /// Update a BSP's capacity.
    ///
    /// # Arguments
    /// * `id` - Database ID of the BSP
    /// * `capacity` - New capacity value
    ///
    /// # Returns
    /// * The updated BSP
    async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp>;

    /// List BSPs with pagination.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// * Vector of BSPs
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>>;

    // ============ Bucket Operations ============

    /// Create a new Bucket in the database.
    ///
    /// # Arguments
    /// * `new_bucket` - Bucket data to insert
    ///
    /// # Returns
    /// * The created Bucket with generated ID and timestamps
    async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket>;

    /// Get a Bucket by its database ID.
    ///
    /// # Arguments
    /// * `id` - Database ID of the Bucket
    ///
    /// # Returns
    /// * `Some(Bucket)` if found, `None` otherwise
    async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>>;

    /// Get all Buckets for a user account.
    ///
    /// # Arguments
    /// * `user_account` - User account string
    ///
    /// # Returns
    /// * Vector of Buckets owned by the user
    async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>>;

    // ============ File Operations ============

    /// Get a File by its key.
    ///
    /// # Arguments
    /// * `key` - File key as bytes
    ///
    /// # Returns
    /// * `Some(File)` if found, `None` otherwise
    async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>>;

    /// Get all Files for a user account.
    ///
    /// # Arguments
    /// * `user_account` - User account as bytes
    ///
    /// # Returns
    /// * Vector of Files owned by the user
    async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>>;

    /// Get all Files in a Bucket.
    ///
    /// # Arguments
    /// * `bucket_id` - Database ID of the Bucket
    ///
    /// # Returns
    /// * Vector of Files in the bucket
    async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>>;
}
