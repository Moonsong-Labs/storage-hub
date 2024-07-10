use sc_tracing::tracing::*;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::BspConfirmedStoring;
use shc_common::types::HasherOutT;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-upload-file-task";

/// BSP confirmed storing handler.
///
/// This handler is triggered when the runtime confirms that the BSP is now storing the file
/// so that the BSP can update it's Forest storage.
pub struct BspConfirmedStoringHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for BspConfirmedStoringHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> BspConfirmedStoringHandler<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> BspConfirmedStoringHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the `BspConfirmedStoring` event.
///
/// This event is triggered by the runtime confirming that the BSP is now storing the file.
impl<T, FL, FS> EventHandler<BspConfirmedStoring> for BspConfirmedStoringHandler<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: BspConfirmedStoring) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Runtime confirmed BSP storing file: {:?}",
            event.file_key,
        );

        let file_key: HasherOutT<T> = TryFrom::<[u8; 32]>::try_from(*event.file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;

        // Get the metadata of the stored file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let file_metadata = read_file_storage
            .get_metadata(&file_key)
            .expect("Failed to get metadata.");
        // Release the file storage lock.
        drop(read_file_storage);

        // Save [`FileMetadata`] of the newly confirmed stored file in the forest storage.
        let mut write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        write_forest_storage
            .insert_metadata(&file_metadata)
            .expect("Failed to insert metadata.");
        // Release the forest storage lock.
        drop(write_forest_storage);

        Ok(())
    }
}
