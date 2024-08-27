use log::info;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::ProofAccepted};
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;
use sp_runtime::AccountId32;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-charge-fees-task";

pub struct BspChargeFeesTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    storage_hub_handler: StorageHubHandler<FL, FS>,
}

impl<FL, FS> Clone for BspChargeFeesTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    fn clone(&self) -> BspChargeFeesTask<FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FS> BspChargeFeesTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    fn new(storage_hub_handler: StorageHubHandler<FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FS> EventHandler<ProofAccepted> for BspChargeFeesTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: ProofAccepted) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "A proof was accepted for provider {:?} and users' fees are going to be charged.", event.provider_id);

        // given some condition that will require a runtime API we will call charge_payment_streams for each user

        let call = storage_hub_runtime::RuntimeCall::PaymentStreams(
            pallet_payment_streams::Call::charge_payment_streams {
                user_account: AccountId32::new([0u8; 32]),
            },
        );

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?;

        Ok(())
    }
}
