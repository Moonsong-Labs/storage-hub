//! Mock implementations for testing
//!
//! This module provides mock implementations of various services for testing purposes.
//! The parent module is already feature-gated with `#[cfg(feature = "mocks")]` to ensure
//! mocks are not included in production builds.

pub mod postgres_mock;
pub mod rpc_mock;

#[cfg(test)]
mod tests;

// Re-export mock types for convenience
pub use postgres_mock::MockPostgresClient;
pub use rpc_mock::MockStorageHubRpc;
