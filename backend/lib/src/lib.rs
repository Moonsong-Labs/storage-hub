//! # StorageHub Backend Library
//!
//! Core library for the StorageHub backend service.

pub mod api;
pub mod config;
pub mod data;
pub mod error;
pub mod services;

#[cfg(feature = "mocks")]
pub mod mocks;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};
