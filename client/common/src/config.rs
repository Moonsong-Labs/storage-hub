use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use toml;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {}

pub fn read_config() -> Config {
    let mut file = File::open("config.toml").expect("config.toml file required");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let config: Config = toml::from_str(&contents).unwrap();

    return config;
}
