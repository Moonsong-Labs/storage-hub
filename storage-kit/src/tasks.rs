use tracing::info;

use crate::{blockchain::events::ChallengeRequest, manager::StorageKitManager, EventHandler};

#[derive(Clone)]
pub struct ResolveBlockchainChallengeRequests {
    storage_kit_manager: StorageKitManager,
}

impl ResolveBlockchainChallengeRequests {
    pub fn new(storage_kit_manager: StorageKitManager) -> Self {
        Self {
            storage_kit_manager,
        }
    }
}

impl EventHandler<ChallengeRequest> for ResolveBlockchainChallengeRequests {
    async fn handle_event(&self, event: ChallengeRequest) -> anyhow::Result<()> {
        info!(
            "[ResolveBlockchainChallengeRequests] - challenge: {}",
            event.challenge
        );

        let registered_bsps = self
            .storage_kit_manager
            .blockchain_module_handle
            .registered_bsps()
            .await?;

        info!("Registered BSPs: {:?}", registered_bsps);

        let multiaddresses = self
            .storage_kit_manager
            .p2p_module_handle
            .multiaddresses()
            .await?;

        info!("P2P talking with: {:?}", multiaddresses);

        Ok(())
    }
}
