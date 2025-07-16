//! Fishing mode event handlers for the indexer service.
//!
//! This module implements the fishing mode which indexes only file creation/deletion
//! events and BSP-file associations needed for fisherman monitoring. This mode is
//! automatically selected when running a fisherman-only node to minimize database
//! load while maintaining file availability tracking.

use anyhow::Result;
use log::trace;
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_indexer_db::DbConnection;
use storage_hub_runtime::{Hash as H256, RuntimeEvent};

use super::IndexerService;

use pallet_file_system;
use pallet_storage_providers;

const LOG_TARGET: &str = "indexer-service::fishing_handlers";

impl<RuntimeApi> IndexerService<RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub(super) async fn index_event_fishing<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &RuntimeEvent,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match event {
            RuntimeEvent::FileSystem(fs_event) => {
                match fs_event {
                    // File creation events
                    pallet_file_system::Event::NewStorageRequest { .. } => {
                        trace!(target: LOG_TARGET, "Indexing NewStorageRequest event");
                        self.index_file_system_event(conn, fs_event).await?
                    }
                    // File deletion events
                    pallet_file_system::Event::StorageRequestRevoked { .. } |
                    pallet_file_system::Event::SpStopStoringInsolventUser { .. } => {
                        trace!(target: LOG_TARGET, "Indexing file deletion event");
                        self.index_file_system_event(conn, fs_event).await?
                    }
                    // BSP-file association events (needed to track which BSPs hold files)
                    pallet_file_system::Event::BspConfirmedStoring { .. } |
                    pallet_file_system::Event::BspConfirmStoppedStoring { .. } => {
                        trace!(target: LOG_TARGET, "Indexing BSP-file association event");
                        self.index_file_system_event(conn, fs_event).await?
                    }
                    // Bucket events (needed to maintain file-bucket relationships)
                    pallet_file_system::Event::NewBucket { .. } |
                    pallet_file_system::Event::BucketDeleted { .. } => {
                        trace!(target: LOG_TARGET, "Indexing bucket event");
                        self.index_file_system_event(conn, fs_event).await?
                    }
                    _ => {
                        trace!(target: LOG_TARGET, "Ignoring non-essential FileSystem event in fishing mode");
                    }
                }
            }
            RuntimeEvent::Providers(provider_event) => {
                match provider_event {
                    // BSP registration/deregistration (needed to maintain BSP records)
                    pallet_storage_providers::Event::BspSignUpSuccess { .. } |
                    pallet_storage_providers::Event::BspSignOffSuccess { .. } |
                    pallet_storage_providers::Event::BspDeleted { .. } => {
                        trace!(target: LOG_TARGET, "Indexing BSP provider event");
                        self.index_providers_event(conn, provider_event, block_hash).await?
                    }
                    _ => {
                        trace!(target: LOG_TARGET, "Ignoring non-BSP provider event in fishing mode");
                    }
                }
            }
            // Explicitly list all other runtime events to ensure compilation errors on new events
            RuntimeEvent::System(_) => {}
            RuntimeEvent::ParachainSystem(_) => {}
            RuntimeEvent::Balances(_) => {}
            RuntimeEvent::TransactionPayment(_) => {}
            RuntimeEvent::Sudo(_) => {}
            RuntimeEvent::CollatorSelection(_) => {}
            RuntimeEvent::Session(_) => {}
            RuntimeEvent::XcmpQueue(_) => {}
            RuntimeEvent::PolkadotXcm(_) => {}
            RuntimeEvent::CumulusXcm(_) => {}
            RuntimeEvent::MessageQueue(_) => {}
            RuntimeEvent::Nfts(_) => {}
            RuntimeEvent::Parameters(_) => {}
            RuntimeEvent::BucketNfts(_) => {}
            RuntimeEvent::PaymentStreams(_) => {}
            RuntimeEvent::ProofsDealer(_) => {}
            RuntimeEvent::Randomness(_) => {}
        }
        Ok(())
    }
}