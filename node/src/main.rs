//! Substrate Parachain Node Template CLI

#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod provider_requests_protocol;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
