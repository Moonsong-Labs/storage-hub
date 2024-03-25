use libp2p::identify::Event as IdentifyEvent;
use tracing::debug;

use super::actor::P2PModule;

impl P2PModule {
    pub(crate) fn handle_identify(&mut self, identify_event: IdentifyEvent) {
        match identify_event {
            IdentifyEvent::Received { peer_id, info } => {
                debug!(
                    "[IdentifyEvent::Received] - with version {} has been received from a peer {}.",
                    info.protocol_version, peer_id
                );

                debug!(
                    "Available protocols for peer {}: {:?}.",
                    peer_id, info.protocols
                );
            }
            IdentifyEvent::Sent { peer_id } => {
                debug!("[IdentifyEvent::Sent] - to peer {}.", peer_id);
            }
            IdentifyEvent::Pushed { peer_id, info } => {
                debug!(
                    "[IdentifyEvent::Pushed] - to peer {} with info {:?}.",
                    peer_id, info
                );
            }
            IdentifyEvent::Error { peer_id, error } => {
                debug!(
                    "[IdentifyEvent::Error] - with peer {} and error {:?}.",
                    peer_id, error
                );
            }
        }
    }
}
