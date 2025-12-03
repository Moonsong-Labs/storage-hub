use log::{info, warn};
use sp_runtime::traits::SaturatedConversion;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{AcceptedBspVolunteer, NewStorageRequest},
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{FileMetadata, HashT, StorageProofsMerkleTrieLayout},
};
use shc_file_transfer_service::commands::FileTransferServiceCommandInterfaceExt;

use crate::{
    handler::StorageHubHandler, tasks::shared::chunk_uploader::ChunkUploaderExt, types::ShNodeType,
};

const LOG_TARGET: &str = "user-sends-file-task";

/// [`UserSendsFileTask`]: Handles the events related to users sending a file to be stored by BSPs
/// volunteering for that file.
/// It can serve multiple BSPs volunteering to store each file, since
/// it reacts to every [`AcceptedBspVolunteer`] from the runtime.
pub struct UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Reacts to a new storage request from the runtime, which is triggered by a user sending a file to be stored.
    /// It generates the file metadata and sends it to the BSPs volunteering to store the file.
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<String> {
        let node_pub_key = self
            .storage_hub_handler
            .blockchain
            .get_node_public_key()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get node public key: {:?}", e))?;

        if event.who != node_pub_key.into() {
            // Skip if the storage request was not created by this user node.
            return Ok("Skipped NewStorageRequest not created by this user node".into());
        }

        info!(
            target: LOG_TARGET,
            "Handling new storage request from user [{:?}], with location [{:?}]",
            event.who,
            event.location,
        );

        let Some(msp_id) = self
            .storage_hub_handler
            .blockchain
            .query_msp_id_of_bucket_id(event.bucket_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP ID of bucket ID {:?}\n Error: {:?}",
                    event.bucket_id,
                    e
                )
            })?
        else {
            warn!(
                target: LOG_TARGET,
                "Skipping storage request - no MSP ID found for bucket ID {:?}",
                event.bucket_id
            );
            return Ok(format!(
                "Skipped NewStorageRequest - no MSP ID for bucket [{:x}]",
                event.bucket_id
            ));
        };

        let multiaddress_vec = self
            .storage_hub_handler
            .blockchain
            .query_provider_multiaddresses(msp_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP multiaddresses of MSP ID {:?}\n Error: {:?}",
                    msp_id,
                    e
                )
            })?;

        // Adds the multiaddresses of the MSP to the known addresses of the file transfer service.
        // This is required to establish a connection to the MSP.
        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(multiaddress_vec)
            .await;

        let who = event.who.as_ref().to_vec();
        let file_metadata = FileMetadata::new(
            who,
            event.bucket_id.as_ref().to_vec(),
            event.location.into_inner(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file key {:?}", file_key);
        }

        self.storage_hub_handler
            .upload_file_to_peer_ids(peer_ids, &file_metadata)
            .await?;

        Ok(format!(
            "Handled NewStorageRequest from user [{}] for file key [{:x}]",
            hex::encode(event.who),
            file_key
        ))
    }
}

impl<NT, Runtime> EventHandler<AcceptedBspVolunteer<Runtime>> for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Reacts to BSPs volunteering (`AcceptedBspVolunteer` from the runtime) to store the user's file,
    /// establishes a connection to each BSPs through the p2p network and sends the file.
    /// At this point we assume that the file is merkleised and already in file storage, and
    /// for this reason the file transfer to the BSP should not fail unless the p2p connection fails.
    async fn handle_event(
        &mut self,
        event: AcceptedBspVolunteer<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Handling BSP volunteering to store a file from user [{:?}], with location [{:?}]",
            event.owner,
            event.location,
        );

        let owner = event.owner.as_ref().to_vec();
        let file_metadata = FileMetadata::new(
            owner,
            event.bucket_id.as_ref().to_vec(),
            event.location.into_inner(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

        // Adds the multiaddresses of the BSP volunteering to store the file to the known addresses of the file transfer service.
        // This is required to establish a connection to the BSP.
        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(event.multiaddresses)
            .await;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file key {:?}", file_key);
        }

        self.storage_hub_handler
            .upload_file_to_peer_ids(peer_ids, &file_metadata)
            .await?;

        Ok(format!(
            "Handled AcceptedBspVolunteer for file key [{:x}]",
            file_key
        ))
    }
}
