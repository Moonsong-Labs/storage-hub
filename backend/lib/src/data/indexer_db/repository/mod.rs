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
use shc_indexer_db::{models::Bsp, OnchainBspId};

pub mod error;
pub mod pool;
pub mod postgres;

use error::RepositoryResult;

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
    // TODO(SCAFFOLDING): The methods are for demonstration.
    // Should be replaced with appropriate methods for what needs to be
    // accessed from the indexer's db

    /// List BSPs with pagination.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// * Vector of BSPs
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>>;
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
