// TODO: Remove this once we don't need the examples in this file
#![allow(dead_code)]

pub mod bsp_upload_file;
pub mod bsp_volunteer_mock;
pub mod user_sends_file;

use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;
use sc_tracing::tracing::info;
use shc_actors_framework::event_bus::EventHandler;
use shc_common::types::HasherOutT;
use sp_trie::TrieLayout;

use crate::services::blockchain::events::{AcceptedBspVolunteer, NewStorageRequest};
use crate::services::file_transfer::events::RemoteUploadRequest;
use crate::services::handler::StorageHubHandler;

// ! The following are examples of task definitions.
pub struct ResolveRemoteUploadRequest<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    _storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for ResolveRemoteUploadRequest<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> ResolveRemoteUploadRequest<T, FL, FS> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> ResolveRemoteUploadRequest<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<RemoteUploadRequest> for ResolveRemoteUploadRequest<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
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

pub struct NewStorageRequestHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    _storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> NewStorageRequestHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<T, FL, FS> Clone for NewStorageRequestHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> NewStorageRequestHandler<T, FL, FS> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> EventHandler<NewStorageRequest> for NewStorageRequestHandler<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}

pub struct AcceptedBspVolunteerHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    _storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for AcceptedBspVolunteerHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> AcceptedBspVolunteerHandler<T, FL, FS> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> AcceptedBspVolunteerHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<AcceptedBspVolunteer> for AcceptedBspVolunteerHandler<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}
