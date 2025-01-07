use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use toml;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    dev: bool,
    provider: bool,
    provider_type: String,
    max_storage_capacity: u64, // Is this supposed to be big number ?
    jump_capacity: u64,        // Same question here
    name: String,
    no_hardware_benchmarks: bool,
    unsafe_rpc_external: bool,
    rpc_methods: bool,
}

pub fn read_config() -> Config {
    let mut file = File::open("config.toml").expect("config.toml file required");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let config: Config = toml::from_str(&contents).unwrap();

    return config;
}
