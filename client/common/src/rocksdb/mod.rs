//! # RocksDB Support Module
//!
//! This module provides database opening and migration support for RocksDB.
//!
//! ## Submodules
//!
//! - [`database`]: Database opening functions and error types
//! - [`migrations`]: Schema migration system for managing column family lifecycle

mod database;
mod migrations;

#[cfg(test)]
mod tests;

pub use database::{
    default_db_options, merge_column_families, open_db, open_db_with_migrations, DatabaseError,
};
pub use migrations::{
    Migration, MigrationError, MigrationRunner, SCHEMA_VERSION_CF, SCHEMA_VERSION_KEY,
};
