use libp2p::swarm::SwarmEvent;
use tracing::{debug, info};

use super::actor::{BehaviourEvent, P2PModule};

impl P2PModule {
    pub(crate) async fn handle_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(
                    "[SwarmEvent::NewListenAddr] - listen address: {}/p2p/{}",
                    address,
                    self.swarm.local_peer_id()
                );
            }
            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } => {
                debug!("[SwarmEvent::Dialing] - peer id: {}", peer_id);
            }
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
                ..
            } => {
                debug!(
                    "[SwarmEvent::IncomingConnection] - local addr: {}, send back addr: {}",
                    local_addr, send_back_addr
                );
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!("[SwarmEvent::ConnectionEstablished] - peer id: {}", peer_id);
            }
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Identify(identify_event) => self.handle_identify(identify_event),
            },
            SwarmEvent::IncomingConnectionError {
                local_addr, error, ..
            } => {
                debug!(
                    "[SwarmEvent::IncomingConnectionError] - local addr: {}, error: {}",
                    local_addr, error
                );
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established,
                cause,
                ..
            } => {
                debug!(
                    "[SwarmEvent::ConnectionClosed] - peer id: {}, num established: {}, cause: {:?}",
                    peer_id, num_established, cause
                );
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                debug!(
                    "[SwarmEvent::OutgoingConnectionError] - peer id: {:?}, error: {}",
                    peer_id, error
                );
            }
            SwarmEvent::NewExternalAddrCandidate { address } => {
                debug!(
                    "[SwarmEvent::NewExternalAddrCandidate] - address: {}",
                    address
                );
            }
            e => panic!("{e:?}"),
        }
    }
}
