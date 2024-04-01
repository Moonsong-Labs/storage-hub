//! Substrate Parachain Node Template CLI

#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;
mod services;

fn main() -> sc_cli::Result<()> {
    command::run()
}
