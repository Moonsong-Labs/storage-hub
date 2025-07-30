use log::error;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

use shc_client::builder::IndexerOptions;

use crate::command::ProviderOptions;

#[derive(Clone, Debug, Deserialize)]
pub struct RemoteFileOptions {
    /// Maximum file size in bytes (default: 10GB)
    #[serde(default)]
    pub max_file_size: u64,
    /// Connection timeout in seconds (default: 30)
    #[serde(default)]
    pub connection_timeout: u64,
    /// Read timeout in seconds (default: 300)
    #[serde(default)]
    pub read_timeout: u64,
    /// Whether to follow redirects (default: true)
    #[serde(default)]
    pub follow_redirects: bool,
    /// Maximum number of redirects (default: 10)
    #[serde(default)]
    pub max_redirects: u32,
    /// User agent string (default: "StorageHub-Client/1.0")
    #[serde(default)]
    pub user_agent: String,
    /// Chunk size in bytes (default: 8192)
    #[serde(default)]
    pub chunk_size: usize,
    /// Number of FILE_CHUNK_SIZE chunks to buffer (default: 512)
    #[serde(default)]
    pub chunks_buffer: usize,
}

impl Default for RemoteFileOptions {
    fn default() -> Self {
        let config = shc_rpc::remote_file::RemoteFileConfig::new(10 * 1024 * 1024 * 1024); // 10GB
        Self {
            max_file_size: config.max_file_size,
            connection_timeout: config.connection_timeout,
            read_timeout: config.read_timeout,
            follow_redirects: config.follow_redirects,
            max_redirects: config.max_redirects,
            user_agent: config.user_agent,
            chunk_size: config.chunk_size,
            chunks_buffer: config.chunks_buffer,
        }
    }
}

impl From<RemoteFileOptions> for shc_rpc::remote_file::RemoteFileConfig {
    fn from(options: RemoteFileOptions) -> Self {
        Self {
            max_file_size: options.max_file_size,
            connection_timeout: options.connection_timeout,
            read_timeout: options.read_timeout,
            follow_redirects: options.follow_redirects,
            max_redirects: options.max_redirects,
            user_agent: options.user_agent,
            chunk_size: options.chunk_size,
            chunks_buffer: options.chunks_buffer,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub provider: ProviderOptions,
    pub indexer: Option<IndexerOptions>,
}

pub fn read_config(path: &str) -> Option<Config> {
    let path = Path::new(path);

    if !path.exists() {
        error!("Fail to find config file ({:?})", path);

        return None;
    }

    let mut file = File::open(path).expect("config.toml file should exist");
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        error!("Fail to read config file : {}", err);

        return None;
    };

    let config = match toml::from_str(&contents) {
        Err(err) => {
            error!("Fail to parse config file : {}", err);

            return None;
        }
        Ok(c) => c,
    };

    return Some(config);
}
