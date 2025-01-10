use jsonrpsee::tracing::warn;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

use crate::command::ProviderOptions;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    // pub dev: bool,
    // pub name: String,
    // pub no_hardware_benchmarks: bool,
    // pub unsafe_rpc_external: bool,
    // pub rpc_methods: bool,
    // pub port: u16,
    // pub rpc_cors: String,
    // pub node_key: String,
    // pub bootnodes: String,
    // pub keystore_path: String,
    // pub sealing: Sealing,
    // pub base_path: String,
    pub provider: ProviderOptions,
}

pub fn read_config(path: &str) -> Option<Config> {
    let path = Path::new(path);

    if !path.exists() {
        println!("Fail find config file ({:?})", path);

        return None;
    }

    let mut file = File::open(path).expect("config.toml file should exist");
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        println!("Fail to read config file : {}", err);

        return None;
    };

    let config = match toml::from_str(&contents) {
        Err(err) => {
            println!("Fail to parse config file : {}", err);

            return None;
        }
        Ok(c) => c,
    };

    return Some(config);
}
