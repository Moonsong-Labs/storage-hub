//! Repository pattern implementation for database operations.
//!
//! This module provides a clean abstraction over database operations using the repository pattern.
//! It includes automatic test transaction management through SmartPool and comprehensive error handling.
//!
//! ## Key Components
//! - [`SmartPool`] - Connection pool with automatic test transaction support
//! - [`RepositoryError`] - Comprehensive error types for repository operations
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
//! use repository::{SmartPool, RepositoryError};
//!
//! let pool = SmartPool::new(&database_url).await?;
//! let conn = pool.get().await?;
//! // Use connection for database operations
//! ```

pub mod error;
pub mod pool;

// Re-export main types for convenience
pub use error::{RepositoryError, RepositoryResult};
pub use pool::SmartPool;