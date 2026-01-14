//! Trusted File Transfer Server
//!
//! HTTP server to receive streamed file chunks via a trusted channel.

pub mod files;
pub mod follower_downloader;
pub mod server;
