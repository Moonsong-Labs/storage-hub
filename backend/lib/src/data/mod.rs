//! Data module for StorageHub backend
//!
//! This module provides the interface to various data sources:
//! - PostgreSQL database (via indexer)
//! - Local storage (for temporary/cache data)
//! - RPC connections to StorageHub nodes

pub mod postgres;
pub mod rpc;
pub mod storage;
