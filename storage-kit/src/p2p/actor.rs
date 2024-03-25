use anyhow::{anyhow, Result};
use tokio::{select, sync::{mpsc::Receiver, oneshot}};
use std::time::Duration;
use libp2p::{futures::StreamExt, identify, identity::Keypair, noise, swarm::NetworkBehaviour, Multiaddr, Swarm};
use tracing::*;

use crate::{Actor, ActorEventLoop, Port};

/// Defines max_negotiating_inbound_streams constant for the swarm.
/// It must be set for large plots.
const SWARM_MAX_NEGOTIATING_INBOUND_STREAMS: usize = 100000;

/// How long will connection be allowed to be open without any usage.
const IDLE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub const IDENTIFY_PROTOCOL: &str = "/storagehub/id/0.0.1";

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
	identify: identify::Behaviour,
}

pub struct P2PModule {
    pub(crate) swarm: Swarm<Behaviour>,
}

impl P2PModule {
	/// Creates a new swarm agent with the given identity on the given `port`.
	pub fn new(identity: Keypair, port: Port) -> Result<P2PModule> {
		let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
			.with_tokio()
			.with_tcp(
				libp2p::tcp::Config::default(),
				noise::Config::new,
				libp2p::yamux::Config::default,
			)?
			.with_quic()
			.with_behaviour(|key| {
				Ok(Behaviour {
					identify: identify::Behaviour::new(identify::Config::new(
						IDENTIFY_PROTOCOL.into(),
						key.public(),
					)),
				})
			})?
			.with_swarm_config(|config| {
				config
					.with_max_negotiating_inbound_streams(SWARM_MAX_NEGOTIATING_INBOUND_STREAMS)
					.with_idle_connection_timeout(IDLE_CONNECTION_TIMEOUT)
			})
			.build();

		// Listen on all interfaces on the specified port.
		swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", port).parse()?)?;

		Ok(P2PModule {
			swarm,
		})
	}
}

pub struct P2PEventLoop {
	receiver: Receiver<P2PModuleCommand>,
	actor: P2PModule,
}

impl ActorEventLoop<P2PModule> for P2PEventLoop {
	fn new(actor: P2PModule, receiver: Receiver<P2PModuleCommand>) -> Self {
		Self { actor, receiver }
	}

	async fn run(&mut self) {
		info!("P2PModule starting up with peerId {:?}", self.actor.swarm.local_peer_id());
		loop {
			select! {
				event = self.actor.swarm.next() => {
					let event = event.ok_or_else(|| anyhow!("Event invalid!")).unwrap();
					self.actor.handle_swarm_event(event).await;
				},
				message = self.receiver.recv() => {
					let message = message.ok_or_else(|| anyhow!("Command invalid!")).unwrap();
					self.actor.handle_message(message).await;
				},
			}
		}
	}
}

/// P2PModule commands that can be sent to the service asynchronously through an mpsc channel.
/// For commands that require a response, a oneshot channel is used to send the response back.
#[derive(Debug)]
pub enum P2PModuleCommand {
	/// Dial an external peer.
	ExternalDial { multiaddr: Multiaddr, channel: oneshot::Sender<Result<()>> },
	/// Get the current list of multiaddresses we are listening on.
	Multiaddresses { channel: oneshot::Sender<Vec<Multiaddr>> },
}

impl Actor for P2PModule {
	type Message = P2PModuleCommand;
	type EventLoop = P2PEventLoop;

	async fn handle_message(&mut self, command: Self::Message) {
		match command {
			P2PModuleCommand::ExternalDial { multiaddr, channel } => {
				self.swarm.dial(multiaddr).unwrap();

				channel
					.send(Ok(()))
					.map_err(|_| anyhow!("Failed to send dial command")).unwrap();
			},
			P2PModuleCommand::Multiaddresses { channel } => {
				let multiaddresses: Vec<Multiaddr> =
					self.swarm.listeners().map(|addr| addr.clone()).collect();

				channel
					.send(multiaddresses)
					.map_err(|_| anyhow!("Failed to send multiaddresses")).unwrap();
			},
		}
	}
}
