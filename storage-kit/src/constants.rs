use std::time::Duration;

use crate::Port;

pub const DEFAULT_P2P_PORT: Port = 30333;

/// Defines max_negotiating_inbound_streams constant for the swarm.
/// It must be set for large plots.
pub const SWARM_MAX_NEGOTIATING_INBOUND_STREAMS: usize = 100000;

/// How long will connection be allowed to be open without any usage.
pub const IDLE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub const IDENTIFY_PROTOCOL: &str = "/storagehub/id/0.0.1";

pub const FILE_CHUNK_SIZE: usize = 1024 * 1024;
