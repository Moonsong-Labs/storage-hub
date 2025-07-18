//! Storage data access module
//!
//! This module provides storage interfaces and implementations for backend-specific data
//! that is not part of the StorageHub indexer database.

pub mod boxed;
pub mod memory;
pub mod traits;

pub use boxed::{BoxedStorage, BoxedStorageWrapper};
pub use memory::InMemoryStorage;
pub use traits::Storage;
