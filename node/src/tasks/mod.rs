// TODO: Remove this once we don't need the examples in this file
#![allow(dead_code)]

pub mod bsp_charge_fees;
pub mod bsp_download_file;
pub mod bsp_submit_proof;
pub mod bsp_upload_file;
pub mod mock_bsp_volunteer;
pub mod mock_sp_react_to_event;
pub mod sp_slash_provider;
pub mod user_sends_file;

use sc_tracing::tracing::info;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{AcceptedBspVolunteer, NewStorageRequest};
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::events::RemoteUploadRequest;
use shc_forest_manager::traits::ForestStorageHandler;

use crate::services::handler::StorageHubHandler;

// ! The following are examples of task definitions.
pub struct ResolveRemoteUploadRequest<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    _storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for ResolveRemoteUploadRequest<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    fn clone(&self) -> ResolveRemoteUploadRequest<FL, FSH> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> ResolveRemoteUploadRequest<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<RemoteUploadRequest> for ResolveRemoteUploadRequest<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        info!(
            "[ResolveRemoteUploadRequest] - file location: {:?}",
            event.file_key
        );

        // self.storage_hub_handler.storage.store_chunk().await?;

        Ok(())
    }
}

pub struct NewStorageRequestHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    _storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> NewStorageRequestHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<FL, FSH> Clone for NewStorageRequestHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    fn clone(&self) -> NewStorageRequestHandler<FL, FSH> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> EventHandler<NewStorageRequest> for NewStorageRequestHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}

pub struct AcceptedBspVolunteerHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    _storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for AcceptedBspVolunteerHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    fn clone(&self) -> AcceptedBspVolunteerHandler<FL, FSH> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> AcceptedBspVolunteerHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<AcceptedBspVolunteer> for AcceptedBspVolunteerHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}
