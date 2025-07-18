//! Mock implementations for testing
//!
//! This module provides mock implementations of various services for testing purposes.
//! All mocks are feature-gated with `#[cfg(feature = "mocks")]` to ensure they are
//! not included in production builds.

#[cfg(feature = "mocks")]
pub mod postgres_mock;

#[cfg(feature = "mocks")]
pub mod rpc_mock;

#[cfg(all(feature = "mocks", test))]
mod tests;

// Re-export mock types for convenience
#[cfg(feature = "mocks")]
pub use postgres_mock::MockPostgresClient;

#[cfg(feature = "mocks")]
pub use rpc_mock::MockStorageHubRpc;
