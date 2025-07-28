//! Storage data access module
//!
//! This module provides storage interfaces and implementations for backend-specific data

pub mod boxed;
pub mod memory;
pub mod traits;

pub use boxed::{BoxedStorage, BoxedStorageWrapper};
pub use memory::InMemoryStorage;
pub use traits::Storage;
