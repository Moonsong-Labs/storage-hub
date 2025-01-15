//! Substrate Parachain Node Template CLI

#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod config;
mod rpc;
mod service;
mod services;
mod tasks;

fn main() -> sc_cli::Result<()> {
    command::run()
}
