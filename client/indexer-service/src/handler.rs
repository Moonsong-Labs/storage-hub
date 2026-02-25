use std::sync::Arc;

use anyhow::anyhow;
use bigdecimal::BigDecimal;
use codec::Encode;
use diesel_async::AsyncConnection;
use futures::prelude::*;
use log::{error, info};
use thiserror::Error;

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use sc_client_api::{BlockBackend, BlockchainEvents};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::{
        convert_raw_multiaddress_to_multiaddr, get_ethereum_block_hash, get_events_at_block,
        EventsRetrievalError,
    },
    traits::StorageEnableRuntime,
    types::{StorageEnableEvents, StorageHubClient, StorageProviderId},
};
use shc_indexer_db::{models::*, DbConnection, DbPool, OnchainBspId, OnchainMspId};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::{Header, NumberFor, SaturatedConversion};

mod fishing;
mod lite;

pub(crate) const LOG_TARGET: &str = "indexer-service";

// Since the indexed data should be used directly from the database,
// we don't need to implement commands.
#[derive(Debug)]
pub enum IndexerServiceCommand {}

// The IndexerService actor
pub struct IndexerService<Runtime: StorageEnableRuntime> {
    client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    db_pool: DbPool,
    indexer_mode: crate::IndexerMode,
}

// Implement the Actor trait for IndexerService
impl<Runtime: StorageEnableRuntime> Actor for IndexerService<Runtime> {
    type Message = IndexerServiceCommand;
    type EventLoop = IndexerServiceEventLoop<Runtime>;
    type EventBusProvider = (); // We're not using an event bus for now

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match message {
                // No commands for now
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &()
    }
}

// Implement methods for IndexerService
impl<Runtime: StorageEnableRuntime> IndexerService<Runtime> {
    pub fn new(
        client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
        db_pool: DbPool,
        indexer_mode: crate::IndexerMode,
    ) -> Self {
        Self {
            client,
            db_pool,
            indexer_mode,
        }
    }

    async fn handle_finality_notification<Block>(
        &mut self,
        notification: sc_client_api::FinalityNotification<Block>,
    ) -> Result<(), HandleFinalityNotificationError>
    where
        Block: sp_runtime::traits::Block<Hash = H256>,
        Block::Header: Header,
    {
        let finalized_block_hash = notification.hash;
        let finalized_block_number: u64 = (*notification.header.number()).saturated_into();

        info!(target: LOG_TARGET, "Finality notification (#{}): {}", finalized_block_number, finalized_block_hash);

        let mut db_conn = self.db_pool.get().await?;

        let service_state = ServiceState::get(&mut db_conn).await?;

        let mut next_block = service_state.last_indexed_finalized_block as u64;
        next_block = next_block.saturating_add(1);

        while next_block <= finalized_block_number {
            let block_hash = self
                .client
                .block_hash(next_block.saturated_into())?
                .ok_or(HandleFinalityNotificationError::BlockHashNotFound)?;
            let next_block_rt: NumberFor<Runtime::Block> = next_block.saturated_into();
            self.index_block(&mut db_conn, next_block_rt, block_hash)
                .await?;
            next_block = next_block.saturating_add(1);
        }

        Ok(())
    }

    async fn index_block<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        block_number: NumberFor<Runtime::Block>,
        block_hash: H256,
    ) -> Result<(), IndexBlockError> {
        info!(target: LOG_TARGET, "Indexing block #{}: {}", block_number, block_hash);

        let block_events = get_events_at_block::<Runtime>(&self.client, &block_hash)?;

        // Build a map of extrinsic index to transaction hash for events in a block.
        let evm_tx_map: std::collections::HashMap<u32, H256> =
            Runtime::build_transaction_hash_map(&block_events);

        conn.transaction::<(), IndexBlockError, _>(move |conn| {
            Box::pin(async move {
                for ev in &block_events {
                    // Get the EVM transaction hash for the event if it exists
                    let maybe_evm_tx_hash =
                        if let frame_system::Phase::ApplyExtrinsic(idx) = ev.phase {
                            evm_tx_map.get(&idx).copied()
                        } else {
                            None
                        };

                    self.route_event(
                        conn,
                        &ev.event.clone().into(),
                        block_hash,
                        block_number,
                        maybe_evm_tx_hash,
                    )
                    .await?;
                }

                // Update the last indexed finalized block after indexing all events
                let block_number_u64: u64 = block_number.saturated_into();
                let block_number_i64: i64 = block_number_u64 as i64;
                ServiceState::update(conn, block_number_i64).await?;

                Ok(())
            })
        })
        .await?;

        info!(target: LOG_TARGET, "Successfully indexed block #{}: {}", block_number, block_hash);

        Ok(())
    }

    async fn route_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: H256,
        block_number: NumberFor<Runtime::Block>,
        evm_tx_hash: Option<H256>,
    ) -> Result<(), IndexBlockError> {
        match self.indexer_mode {
            crate::IndexerMode::Full => {
                self.index_event(conn, event, block_hash, block_number, evm_tx_hash)
                    .await
            }
            crate::IndexerMode::Lite => {
                self.index_event_lite(conn, event, block_hash, block_number, evm_tx_hash)
                    .await
            }
            crate::IndexerMode::Fishing => {
                self.index_event_fishing(conn, event, block_hash, block_number, evm_tx_hash)
                    .await
            }
        }
    }

    async fn index_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: H256,
        block_number: NumberFor<Runtime::Block>,
        evm_tx_hash: Option<H256>,
    ) -> Result<(), IndexBlockError> {
        match event {
            StorageEnableEvents::BucketNfts(event) => {
                self.index_bucket_nfts_event(conn, event).await?
            }
            StorageEnableEvents::FileSystem(event) => {
                self.index_file_system_event(conn, event, block_hash, block_number, evm_tx_hash)
                    .await?
            }
            StorageEnableEvents::PaymentStreams(event) => {
                self.index_payment_streams_event(conn, event, block_number)
                    .await?
            }
            StorageEnableEvents::ProofsDealer(event) => {
                self.index_proofs_dealer_event(conn, event, block_hash, block_number)
                    .await?
            }
            StorageEnableEvents::StorageProviders(event) => {
                self.index_providers_event(conn, event, block_hash, block_number)
                    .await?
            }
            StorageEnableEvents::Randomness(event) => {
                self.index_randomness_event(conn, event, block_number)
                    .await?
            }
            // TODO: We have to index the events from the CrRandomness pallet when we integrate it to the runtime,
            // since they contain the information about the commit-reveal deadlines for Providers.
            // RuntimeEvent::CrRandomness(event) => self.index_cr_randomness_event(conn, event).await?,
            // Runtime events that we're not interested in.
            // We add them here instead of directly matching (_ => {})
            // to ensure the compiler will let us know to treat future events when added.
            StorageEnableEvents::System(_) => {}
            StorageEnableEvents::Balances(_) => {}
            StorageEnableEvents::TransactionPayment(_) => {}
            StorageEnableEvents::Other(_) => {}
        }

        Ok(())
    }

    async fn index_bucket_nfts_event<'a, 'b: 'a>(
        &'b self,
        _conn: &mut DbConnection<'a>,
        event: &pallet_bucket_nfts::Event<Runtime>,
    ) -> Result<(), IndexBlockError> {
        match event {
            pallet_bucket_nfts::Event::AccessShared { .. } => {}
            pallet_bucket_nfts::Event::ItemReadAccessUpdated { .. } => {}
            pallet_bucket_nfts::Event::ItemBurned { .. } => {}
            pallet_bucket_nfts::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_file_system_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_file_system::Event<Runtime>,
        block_hash: H256,
        block_number: NumberFor<Runtime::Block>,
        evm_tx_hash: Option<H256>,
    ) -> Result<(), IndexBlockError> {
        match event {
            // Bucket lifecycle events
            pallet_file_system::Event::NewBucket {
                who,
                msp_id,
                bucket_id,
                name,
                collection_id,
                private,
                value_prop_id,
                root,
            } => {
                let msp = Some(
                    Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id))
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "NewBucket (get MSP)".to_string(),
                        })?,
                );

                Bucket::create(
                    conn,
                    msp.map(|m| m.id),
                    who.to_string(),
                    bucket_id.as_ref().to_vec(),
                    name.to_vec(),
                    collection_id.map(|id| id.to_string()),
                    *private,
                    root.as_ref().to_vec(),
                    format!("{:#?}", value_prop_id), // using .to_string() leads to truncation
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "NewBucket (create bucket)".to_string(),
                })?;
            }
            pallet_file_system::Event::BucketDeleted {
                who: _,
                bucket_id,
                maybe_collection_id: _,
            } => {
                Bucket::delete(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "BucketDeleted (delete bucket)".to_string(),
                    })?;
            }
            pallet_file_system::Event::BucketPrivacyUpdated {
                who,
                bucket_id,
                collection_id,
                private,
            } => {
                Bucket::update_privacy(
                    conn,
                    who.to_string(),
                    bucket_id.as_ref().to_vec(),
                    collection_id.map(|id| id.to_string()),
                    *private,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BucketPrivacyUpdated".to_string(),
                })?;
            }
            pallet_file_system::Event::NewCollectionAndAssociation { .. } => {}

            // Move bucket events
            pallet_file_system::Event::MoveBucketRequested { .. } => {}
            pallet_file_system::Event::MoveBucketRequestExpired { .. } => {}
            pallet_file_system::Event::MoveBucketAccepted {
                old_msp_id,
                new_msp_id,
                bucket_id,
                value_prop_id: _,
            } => {
                let old_msp = if let Some(id) = old_msp_id {
                    Some(
                        Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*id))
                            .await
                            .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "MoveBucketAccepted (old MSP)".to_string(),
                            })?,
                    )
                } else {
                    None
                };
                let new_msp = Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*new_msp_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MoveBucketAccepted (new MSP)".to_string(),
                    })?;

                // Delete any old MSP associations that the files in the bucket had
                if let Some(old_msp) = old_msp {
                    MspFile::delete_by_bucket(conn, bucket_id.as_ref(), old_msp.id)
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "MoveBucketAccepted (delete old MSP-file associations)"
                                .to_string(),
                        })?;
                }

                // Create new MSP associations for all files in the bucket
                MspFile::create_for_bucket(conn, bucket_id.as_ref(), new_msp.id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MoveBucketAccepted (create MSP-file associations)".to_string(),
                    })?;

                // Update bucket's MSP reference
                Bucket::update_msp(conn, bucket_id.as_ref().to_vec(), new_msp.id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MoveBucketAccepted (update bucket MSP reference)".to_string(),
                    })?;
            }
            pallet_file_system::Event::MoveBucketRejected { .. } => {}

            // Storage request lifecycle events
            pallet_file_system::Event::NewStorageRequest {
                who,
                file_key,
                bucket_id,
                location,
                fingerprint,
                size,
                peer_ids,
                expires_at: _,
                bsps_required,
                msp_id,
            } => {
                let bucket = Bucket::get_by_onchain_bucket_id(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "NewStorageRequest (get bucket)".to_string(),
                    })?;

                let mut sql_peer_ids = Vec::new();
                for peer_id in peer_ids {
                    sql_peer_ids.push(PeerId::create(conn, peer_id.to_vec()).await.map_err(
                        |e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "NewStorageRequest (create peer ID)".to_string(),
                        },
                    )?);
                }

                let size: u64 = (*size).saturated_into();
                let size: i64 = size.saturated_into();
                let who = who.as_ref().to_vec();

                // Get the runtime-specific block hash from storage.
                // For a standard Substrate runtime, the Ethereum block hash won't exist in storage,
                // so we'll fallback to using the Substrate block hash (blake2_256) which is what we want.
                // For a EVM-compatible runtime, we'll get the EVM block hash (keccak256) from storage,
                // which is different from the Substrate block hash.
                let block_number_u32: u32 = block_number.saturated_into();
                let runtime_block_hash =
                    get_ethereum_block_hash(&self.client, &block_hash, block_number_u32)
                        .unwrap_or(None)
                        .unwrap_or(block_hash);
                let block_hash_bytes = runtime_block_hash.as_bytes().to_vec();

                // Convert EVM tx hash to bytes if present
                let tx_hash_bytes = evm_tx_hash.map(|h| h.as_bytes().to_vec());

                // Check if this file key is already present in the bucket of the MSP
                // This could happen if there was a previous storage request for this file key that
                // the MSP accepted, and the new storage request was issued by the user to add redundancy to it.
                // We do this check because in this scenario,the `MutationsApplied` event won't be emitted for this
                // file key when the MSP accepts it, as the MSP is already storing it.
                let is_in_bucket = File::is_file_key_in_bucket(conn, file_key.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "NewStorageRequest (check if file key is in bucket)"
                            .to_string(),
                    })?;

                // Compute desired_replicas using the rolling max formula.
                // System-initiated SRs (msp_id is None) carry forward the previous desired value.
                // User-initiated SRs (msp_id is Some) apply: max(prev_desired, current_bsp_count + bsps_required).
                let is_system_sr = msp_id.is_none();
                let bsps_required_val: i32 = if is_system_sr {
                    1i32
                } else {
                    let val: u64 = (*bsps_required).into();
                    i32::try_from(val).unwrap_or(i32::MAX)
                };

                let prev_desired =
                    File::get_max_desired_replicas_by_file_key(conn, file_key.as_ref().to_vec())
                        .await
                        .unwrap_or(0);

                let new_desired = if is_system_sr {
                    prev_desired
                } else {
                    let current_bsp_count = File::count_bsp_associations_by_file_key(
                        conn,
                        file_key.as_ref().to_vec(),
                    )
                    .await
                    .unwrap_or(0) as i32;
                    std::cmp::max(prev_desired, current_bsp_count + bsps_required_val)
                };

                File::create(
                    conn,
                    who,
                    file_key.as_ref().to_vec(),
                    bucket.id,
                    bucket_id.as_ref().to_vec(),
                    location.to_vec(),
                    fingerprint.as_ref().to_vec(),
                    size,
                    FileStorageRequestStep::Requested,
                    sql_peer_ids,
                    block_hash_bytes,
                    tx_hash_bytes,
                    is_in_bucket,
                    bsps_required_val,
                    new_desired,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "NewStorageRequest (create file step)".to_string(),
                })?;
            }
            pallet_file_system::Event::MspAcceptedStorageRequest {
                file_key,
                file_metadata,
            } => {
                // There can be multiple file records for a given file key if there were multiple
                // storage requests for the same file key. We get the latest one created, which
                // has to be the one that was accepted, given that there can't be two storage
                // requests for the same file key at the same time.
                let file = match File::get_latest_by_file_key(conn, file_key.as_ref().to_vec())
                    .await
                {
                    Ok(file) => file,
                    Err(diesel::result::Error::NotFound) => {
                        // This can happen if the file was completely deleted from the DB (so all BSP and MSP associations were deleted)
                        // but a storage request was still present on-chain so the MSP accepted it.
                        log::info!(
                            target: LOG_TARGET,
                            "File record not found for file_key [{:x}] during MspAcceptedStorageRequest. \
                            Recreating from event metadata (recovery).",
                            file_key
                        );

                        // Recreate the file record from the metadata in the event
                        let bucket = Bucket::get_by_onchain_bucket_id(
                            conn,
                            file_metadata.bucket_id().to_vec(),
                        )
                        .await
                        .map_err(|e| {
                            IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "MspAcceptedStorageRequest (get bucket)".to_string(),
                            }
                        })?;

                        let size: u64 = file_metadata.file_size();
                        let size: i64 = size.saturated_into();

                        let block_hash_bytes = block_hash.as_bytes().to_vec();
                        let tx_hash_bytes = evm_tx_hash.map(|h| h.as_bytes().to_vec());

                        // Check if this file key is already present in the bucket of the MSP
                        // In this scenario, this will always return false, since there's no other file record
                        // in the DB, but it's still a good practice to check it.
                        let is_in_bucket =
                            File::is_file_key_in_bucket(conn, file_key.as_ref().to_vec())
                                .await
                                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name:
                                        "MspAcceptedStorageRequest (check if file key is in bucket)"
                                            .to_string(),
                                })?;

                        // Create file with Requested step since we will change it to Stored when the storage request is fulfilled
                        File::create(
                            conn,
                            file_metadata.owner().clone(),
                            file_key.as_ref().to_vec(),
                            bucket.id,
                            file_metadata.bucket_id().clone(),
                            file_metadata.location().clone(),
                            file_metadata.fingerprint().as_ref().to_vec(),
                            size,
                            FileStorageRequestStep::Requested,
                            vec![], // No peer_ids available from acceptance event
                            block_hash_bytes,
                            tx_hash_bytes,
                            is_in_bucket,
                            0, // bsps_required not available from acceptance event
                            0, // desired_replicas not available from acceptance event
                        )
                        .await
                        .map_err(|e| {
                            IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "MspAcceptedStorageRequest (create file)".to_string(),
                            }
                        })?
                    }
                    Err(e) => {
                        return Err(IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "MspAcceptedStorageRequest (get file)".to_string(),
                        })
                    }
                };

                let bucket = Bucket::get_by_onchain_bucket_id(conn, file.onchain_bucket_id.clone())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspAcceptedStorageRequest (get bucket)".to_string(),
                    })?;
                if let Some(msp_id) = bucket.msp_id {
                    MspFile::create(conn, msp_id, file.id).await.map_err(|e| {
                        IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "MspAcceptedStorageRequest (create MSP-file association)"
                                .to_string(),
                        }
                    })?;
                }
            }
            pallet_file_system::Event::StorageRequestFulfilled { file_key } => {
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Stored,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "StorageRequestFulfilled (update file step)".to_string(),
                })?;
            }
            pallet_file_system::Event::StorageRequestExpired { file_key } => {
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Expired,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "StorageRequestExpired (update file step)".to_string(),
                })?;
            }
            pallet_file_system::Event::StorageRequestRevoked { file_key } => {
                // Mark storage request as revoked so it's not protected from deletion
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Revoked,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "StorageRequestRevoked (update file step)".to_string(),
                })?;
                // Delete file if it has no storage (not in bucket forest and no BSP associations)
                // This happens when storage request is revoked before any BSPs or MSP confirms or accepted respectively.
                File::delete_if_orphaned(conn, file_key.as_ref())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "StorageRequestRevoked (delete file if orphaned)".to_string(),
                    })?;
                // If the file has storage, the `IncompleteStorageRequest` event will handle it
            }
            pallet_file_system::Event::StorageRequestRejected {
                file_key,
                msp_id: _,
                bucket_id: _,
                reason: _,
            } => {
                // Mark storage request as rejected so it's not protected from deletion
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Rejected,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "StorageRequestRejected (update file step)".to_string(),
                })?;
                // Delete file if it has no storage (not in bucket forest and no BSP associations)
                // This happens when a storage request is rejected by the MSP.
                // It is possible that there might be no BSP associations.
                File::delete_if_orphaned(conn, file_key.as_ref())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "StorageRequestRejected (delete file if orphaned)".to_string(),
                    })?;
                // If the file has storage, the `IncompleteStorageRequest` event will handle it
            }
            // This event covers all scenarios where a storage request was unfulfilled while there were BSPs and/or the MSP who have confirmed to store the file
            // and necessitates a fisherman to delete this file.
            pallet_file_system::Event::IncompleteStorageRequest { file_key } => {
                // Check if file is in bucket or has BSP associations
                // There can be multiple file records for a given file key if there were multiple
                // storage requests for the same file key. We get the latest one created, which
                // has to be the incomplete one, given that there can't be two storage
                // requests for the same file key at the same time.
                let file_record = File::get_latest_by_file_key(conn, file_key.as_ref().to_vec())
                    .await
                    .ok();

                // If the file record is not found, it means the file has been deleted already.
                if let Some(file_record) = file_record {
                    let is_in_bucket = file_record.is_in_bucket;
                    let has_bsp = File::has_bsp_associations(conn, file_record.id)
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name:
                                "IncompleteStorageRequest (check if file has BSP associations)"
                                    .to_string(),
                        })?;

                    if is_in_bucket || has_bsp {
                        // File is still being stored, check if it has already been marked for deletion
                        // and if not, mark it for deletion.
                        // This is because a deletion request (with the user's signed intention) takes precedence,
                        // and we don't want to clear the user's signature.
                        if file_record.deletion_status.is_none() {
                            File::update_deletion_status(
                                conn,
                                file_key.as_ref(),
                                FileDeletionStatus::InProgress,
                                None,
                            )
                            .await
                            .map_err(|e| {
                                IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name:
                                        "IncompleteStorageRequest (update file deletion status)"
                                            .to_string(),
                                }
                            })?;

                            log::debug!(
                        		"Incomplete storage request for file {:?} (id: {:?}) is still being stored (in_bucket: {}, BSP: {}), marked for deletion without signature",
                        		file_key, file_record.id, is_in_bucket, has_bsp
                    		);
                        }
                    } else {
                        // No storage, safe to delete immediately
                        File::delete(conn, file_record.id).await.map_err(|e| {
                            IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "IncompleteStorageRequest (delete file)".to_string(),
                            }
                        })?;
                        log::debug!(
                            "Incomplete storage request for file key [{:x}] and id {:?} is not being stored, deleted immediately",
                            file_key, file_record.id,
                        );
                    }
                }
            }
            // This event is emitted when an incomplete storage request has been fully cleaned up,
            // i.e. when no more providers are pending removal.
            //
            // We need to delete the MSP-file association and the file record for this incomplete
            // storage request, as there's no further cleanup needed on-chain.
            pallet_file_system::Event::IncompleteStorageRequestCleanedUp { file_key } => {
                // Get the latest file record for this file key
                // We are sure this is the correct file record that we need to clean up as an `IncompleteStorageRequest`
                // is always associated with the latest storage request, and no other storage request can be issued while
                // this one is still incomplete.
                let file_record = File::get_latest_by_file_key(conn, file_key.as_ref().to_vec())
                    .await
                    .ok();

                if let Some(file_record) = file_record {
                    // Get the bucket to find the MSP
                    let bucket = Bucket::get_by_onchain_bucket_id(
                        conn,
                        file_record.onchain_bucket_id.clone(),
                    )
                    .await
                    .map_err(|e| {
                        IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "IncompleteStorageRequestCleanedUp (get bucket)"
                                .to_string(),
                        }
                    })?;

                    // Delete the MSP-file association if this file is associated with an MSP
                    // We have to do this as this association might have not been cleared before if
                    // the MSP accepted the storage request with an inclusion proof, because in that scenario
                    // the fisherman does not delete the file from the bucket.
                    if let Some(msp_id) = bucket.msp_id {
                        MspFile::delete_latest_by_file_key(conn, msp_id, file_key.as_ref()).await.map_err(|e| {
                            IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "IncompleteStorageRequestCleanedUp (delete MSP-file association)"
                                    .to_string(),
                            }
                        })?;
                        log::debug!(
                            "Deleted MSP-file association for incomplete storage request file {:?} (id: {:?})",
                            file_key,
                            file_record.id
                        );
                    }

                    // Try to delete the latest file record if it has no BSP or MSP associations, which at this point it shouldn't
                    File::delete_latest_by_file_key(conn, file_key.as_ref()).await.
                    map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "IncompleteStorageRequestCleanedUp (delete latest file record by file key)"
                            .to_string(),
                    })?;
                }
            }

            // BSP volunteer and confirmation events
            pallet_file_system::Event::AcceptedBspVolunteer { .. } => {}
            pallet_file_system::Event::BspConfirmedStoring {
                who: _,
                bsp_id,
                confirmed_file_keys,
                skipped_file_keys: _,
                new_root,
            } => {
                Bsp::update_merkle_root(
                    conn,
                    OnchainBspId::from(*bsp_id),
                    new_root.as_ref().to_vec(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BspConfirmedStoring (update BSP merkle root)".to_string(),
                })?;

                let bsp = Bsp::get_by_onchain_bsp_id(conn, OnchainBspId::from(*bsp_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "BspConfirmedStoring (get BSP)".to_string(),
                    })?;
                for (file_key, file_metadata) in confirmed_file_keys {
                    // There can be multiple file records for a given file key if there were multiple
                    // storage requests for the same file key. We get the latest one created, which
                    // has to be the one that was confirmed, given that there can't be two storage
                    // requests for the same file key at the same time.
                    let file = match File::get_latest_by_file_key(conn, file_key.as_ref().to_vec())
                        .await
                    {
                        Ok(file) => file,
                        Err(diesel::result::Error::NotFound) => {
                            // This can happen if the file was completely deleted from the DB (so all BSP and MSP associations were deleted)
                            // but a storage request was still present on-chain so a BSP confirmed storing it.
                            log::info!(
                                target: LOG_TARGET,
                                "File record not found for file_key [{:x}] during BspConfirmedStoring. \
                                Recreating from event metadata (recovery).",
                                file_key
                            );

                            // Recreate the file record from the metadata in the event
                            let bucket = Bucket::get_by_onchain_bucket_id(
                                conn,
                                file_metadata.bucket_id().to_vec(),
                            )
                            .await
                            .map_err(|e| {
                                IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name: "BspConfirmedStoring (get bucket)".to_string(),
                                }
                            })?;

                            let size: u64 = file_metadata.file_size();
                            let size: i64 = size.saturated_into();

                            let block_hash_bytes = block_hash.as_bytes().to_vec();
                            let tx_hash_bytes = evm_tx_hash.map(|h| h.as_bytes().to_vec());

                            // Check if this file key is already present in the bucket of the MSP
                            // In this scenario, this will always return false, since there's no other file record
                            // in the DB, but it's still good practice to check it.
                            let is_in_bucket =
                                File::is_file_key_in_bucket(conn, file_key.as_ref().to_vec())
                                    .await
                                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                        database_error: e,
                                        block_number: block_number.saturated_into(),
                                        event_name:
                                            "BspConfirmedStoring (check if file key is in bucket)"
                                                .to_string(),
                                    })?;

                            // Create file with Requested step since we will change it to Stored when the storage request is fulfilled
                            File::create(
                                conn,
                                file_metadata.owner().clone(),
                                file_key.as_ref().to_vec(),
                                bucket.id,
                                file_metadata.bucket_id().clone(),
                                file_metadata.location().clone(),
                                file_metadata.fingerprint().as_ref().to_vec(),
                                size,
                                FileStorageRequestStep::Requested,
                                vec![], // No peer_ids available from confirmation event
                                block_hash_bytes,
                                tx_hash_bytes,
                                is_in_bucket,
                                0, // bsps_required not available from confirmation event
                                0, // desired_replicas not available from confirmation event
                            )
                            .await
                            .map_err(|e| {
                                IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name: "BspConfirmedStoring (create file)".to_string(),
                                }
                            })?
                        }
                        Err(e) => {
                            return Err(IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name: "BspConfirmedStoring (get file)".to_string(),
                            })
                        }
                    };

                    BspFile::create(conn, bsp.id, file.id).await.map_err(|e| {
                        IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "BspConfirmedStoring (create BSP-file association)"
                                .to_string(),
                        }
                    })?;
                }
            }
            pallet_file_system::Event::BspChallengeCycleInitialised { .. } => {}

            // Stop storing events
            pallet_file_system::Event::BspRequestedToStopStoring { .. } => {}
            pallet_file_system::Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key,
                new_root,
            } => {
                Bsp::update_merkle_root(
                    conn,
                    OnchainBspId::from(*bsp_id),
                    new_root.as_ref().to_vec(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BspConfirmStoppedStoring (update BSP merkle root)".to_string(),
                })?;
                BspFile::delete_for_bsp(conn, file_key.as_ref(), OnchainBspId::from(*bsp_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "BspConfirmStoppedStoring (delete BSP-file associations)"
                            .to_string(),
                    })?;
            }
            pallet_file_system::Event::MspStoppedStoringBucket {
                msp_id,
                owner: _,
                bucket_id,
            } => {
                // In this scenario, there's no need to update the `is_in_bucket` field of the files in the bucket,
                // since the bucket still exists and is still storing the files (according to its on-chain forest root).

                // Delete the MSP-file associations for all files in the bucket
                let msp = Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStoppedStoringBucket (get MSP)".to_string(),
                    })?;
                MspFile::delete_by_bucket(conn, bucket_id.as_ref(), msp.id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStoppedStoringBucket (delete MSP-file associations)"
                            .to_string(),
                    })?;

                // Unset the MSP from the bucket to reflect on-chain state
                Bucket::unset_msp(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStoppedStoringBucket (unset bucket MSP reference)"
                            .to_string(),
                    })?;
            }

            // Insolvent user events
            pallet_file_system::Event::SpStopStoringInsolventUser {
                sp_id,
                file_key,
                owner: _,
                location: _,
                new_root: _,
            } => {
                // Get the file's bucket ID before any deletions in case we have to clean up the bucket afterwards
                let file_record = File::get_latest_by_file_key(conn, file_key.as_ref().to_vec())
                    .await
                    .ok();
                let onchain_bucket_id = file_record.map(|f| f.onchain_bucket_id);

                // This event can be emitted by either a BSP or MSP stopping storage for an insolvent user.
                // We need to check which type of provider it is and handle accordingly.
                let bsp_result = Bsp::get_by_onchain_bsp_id(conn, OnchainBspId::from(*sp_id)).await;

                // If it's a BSP, delete the BSP-file association
                if bsp_result.is_ok() {
                    BspFile::delete_for_bsp(conn, file_key, OnchainBspId::from(*sp_id))
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "SpStopStoringInsolventUser (delete BSP-file associations)"
                                .to_string(),
                        })?;
                } else {
                    // It's an MSP, delete the MSP-file association
                    MspFile::delete(conn, file_key.as_ref(), OnchainMspId::from(*sp_id))
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "SpStopStoringInsolventUser (delete MSP-file associations)"
                                .to_string(),
                        })?;
                }

                // Clean up the file if it has no remaining associations
                File::delete_if_orphaned(conn, file_key.as_ref())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "SpStopStoringInsolventUser (delete file if orphaned)"
                            .to_string(),
                    })?;

                // If the file was deleted and belonged to a bucket that was deleted on-chain
                // try to clean up the bucket as well
                if let Some(bucket_id) = onchain_bucket_id {
                    // Only attempt bucket cleanup if the bucket has no MSP, as this means
                    // the MSP has already deleted the bucket.
                    if let Ok(bucket) =
                        Bucket::get_by_onchain_bucket_id(conn, bucket_id.clone()).await
                    {
                        if bucket.msp_id.is_none() {
                            Bucket::delete_if_orphaned(conn, bucket_id)
                                .await
                                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name:
                                        "SpStopStoringInsolventUser (delete bucket if orphaned)"
                                            .to_string(),
                                })?;
                        }
                    }
                }
            }
            pallet_file_system::Event::MspStopStoringBucketInsolventUser {
                msp_id,
                owner: _,
                bucket_id,
            } => {
                let msp = Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStopStoringBucketInsolventUser (get MSP)".to_string(),
                    })?;

                // Get all files in this bucket before deleting associations
                let files = File::get_by_onchain_bucket_id(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStopStoringBucketInsolventUser (get files)".to_string(),
                    })?;

                // Delete the MSP-file associations for all files in the bucket
                MspFile::delete_by_bucket(conn, bucket_id.as_ref(), msp.id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name:
                            "MspStopStoringBucketInsolventUser (delete MSP-file associations)"
                                .to_string(),
                    })?;

                // Mark all files as no longer in the bucket and clean up orphaned files.
                for file in files {
                    File::update_bucket_membership(conn, &file.file_key, bucket_id.as_ref(), false)
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name:
                                "MspStopStoringBucketInsolventUser (update file bucket membership)"
                                    .to_string(),
                        })?;
                    File::delete_if_orphaned(conn, &file.file_key)
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name:
                                "MspStopStoringBucketInsolventUser (delete file if orphaned)"
                                    .to_string(),
                        })?;
                }

                // Unset the MSP from the bucket to reflect on-chain state
                Bucket::unset_msp(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name:
                            "MspStopStoringBucketInsolventUser (unset bucket MSP reference)"
                                .to_string(),
                    })?;

                // Try to delete the bucket if no files reference it anymore.
                // If files still exist (e.g. BSPs are still storing them), the bucket
                // record is kept and will be cleaned up when the last file is deleted.
                Bucket::delete_if_orphaned(conn, bucket_id.as_ref().to_vec())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspStopStoringBucketInsolventUser (delete bucket if orphaned)"
                            .to_string(),
                    })?;
            }

            // File deletion events
            pallet_file_system::Event::FileDeletionRequested {
                signed_delete_intention,
                signature,
            } => {
                // Mark the file for deletion with the user signed deletion intention.
                let file_key = &signed_delete_intention.file_key;
                let signature_bytes = signature.encode();
                File::update_deletion_status(
                    conn,
                    file_key.as_ref(),
                    FileDeletionStatus::InProgress,
                    Some(signature_bytes),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "FileDeletionRequested (update file deletion status)".to_string(),
                })?;
            }
            pallet_file_system::Event::BucketFileDeletionsCompleted {
                user: _,
                file_keys,
                bucket_id,
                msp_id: maybe_msp_id,
                old_root: _,
                new_root,
            } => {
                // Delete MSP-file associations for all files in the batch
                if let Some(msp_id) = maybe_msp_id {
                    for file_key in file_keys.iter() {
                        MspFile::delete(conn, file_key.as_ref(), OnchainMspId::from(*msp_id))
                            .await
                            .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                database_error: e,
                                block_number: block_number.saturated_into(),
                                event_name:
                                    "BucketFileDeletionsCompleted (delete MSP-file associations)"
                                        .to_string(),
                            })?;
                    }
                }

                // Check if files should be deleted (no more associations)
                for file_key in file_keys.iter() {
                    File::delete_if_orphaned(conn, file_key.as_ref())
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "BucketFileDeletionsCompleted (delete file if orphaned)"
                                .to_string(),
                        })?;
                }

                // Update bucket merkle root
                Bucket::update_merkle_root(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    new_root.as_ref().to_vec(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BucketFileDeletionsCompleted (update bucket merkle root)"
                        .to_string(),
                })?;
            }
            pallet_file_system::Event::BspFileDeletionsCompleted {
                users: _,
                file_keys,
                bsp_id,
                old_root: _,
                new_root,
            } => {
                // Delete BSP-file associations for all files in the batch
                for file_key in file_keys.iter() {
                    BspFile::delete_for_bsp(conn, file_key.as_ref(), OnchainBspId::from(*bsp_id))
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "BspFileDeletionsCompleted (delete BSP-file associations)"
                                .to_string(),
                        })?;
                }

                // Check if files should be deleted (no more associations)
                for file_key in file_keys.iter() {
                    File::delete_if_orphaned(conn, file_key.as_ref())
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "BspFileDeletionsCompleted (delete file if orphaned)"
                                .to_string(),
                        })?;
                }

                // Update BSP merkle root
                Bsp::update_merkle_root(
                    conn,
                    OnchainBspId::from(*bsp_id),
                    new_root.as_ref().to_vec(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BspFileDeletionsCompleted (update BSP merkle root)".to_string(),
                })?;
            }

            // System and error events
            pallet_file_system::Event::UsedCapacityShouldBeZero { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_file_system::Event::FailedToReleaseStorageRequestCreationDeposit { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }

            // User operation pause flags events
            pallet_file_system::Event::UserOperationPauseFlagsUpdated { .. } => {}

            pallet_file_system::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_payment_streams_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_payment_streams::Event<Runtime>,
        block_number: NumberFor<Runtime::Block>,
    ) -> Result<(), IndexBlockError> {
        match event {
            pallet_payment_streams::Event::DynamicRatePaymentStreamCreated {
                provider_id,
                user_account,
                amount_provided,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);
                PaymentStream::create_dynamic_rate(
                    conn,
                    user_account.to_string(),
                    provider_id,
                    (*amount_provided).into().into(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name:
                        "DynamicRatePaymentStreamCreated (create dynamic rate payment stream)"
                            .to_string(),
                })?;
            }
            pallet_payment_streams::Event::DynamicRatePaymentStreamUpdated {
                provider_id,
                user_account,
                new_amount_provided,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name:
                            "DynamicRatePaymentStreamUpdated (get dynamic rate payment stream)"
                                .to_string(),
                    })?;

                PaymentStream::update_dynamic_rate(
                    conn,
                    ps.id,
                    (*new_amount_provided).into().into(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name:
                        "DynamicRatePaymentStreamUpdated (update dynamic rate payment stream)"
                            .to_string(),
                })?;
            }
            pallet_payment_streams::Event::DynamicRatePaymentStreamDeleted { .. } => {}
            pallet_payment_streams::Event::FixedRatePaymentStreamCreated {
                provider_id,
                user_account,
                rate,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                PaymentStream::create_fixed_rate(
                    conn,
                    user_account.to_string(),
                    provider_id,
                    (*rate).into(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "FixedRatePaymentStreamCreated (create fixed rate payment stream)"
                        .to_string(),
                })?;
            }
            pallet_payment_streams::Event::FixedRatePaymentStreamUpdated {
                provider_id,
                user_account,
                new_rate,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "FixedRatePaymentStreamUpdated (get fixed rate payment stream)"
                            .to_string(),
                    })?;
                PaymentStream::update_fixed_rate(conn, ps.id, (*new_rate).into())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name:
                            "FixedRatePaymentStreamUpdated (update fixed rate payment stream)"
                                .to_string(),
                    })?;
            }
            pallet_payment_streams::Event::FixedRatePaymentStreamDeleted { .. } => {}
            pallet_payment_streams::Event::PaymentStreamCharged {
                user_account,
                provider_id,
                amount,
                last_tick_charged,
                charged_at_tick,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                // We want to handle this and update the payment stream total amount
                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "PaymentStreamCharged (get payment stream)".to_string(),
                    })?;
                let amount: BigDecimal = (*amount).into();
                let new_total_amount = ps.total_amount_paid + amount;
                let last_tick_charged: u64 = (*last_tick_charged).saturated_into();
                let charged_at_tick: u64 = (*charged_at_tick).saturated_into();
                PaymentStream::update_total_amount(
                    conn,
                    ps.id,
                    new_total_amount,
                    last_tick_charged.saturated_into(),
                    charged_at_tick.saturated_into(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "PaymentStreamCharged (update payment stream total amount)"
                        .to_string(),
                })?;
            }
            pallet_payment_streams::Event::UsersCharged { .. } => {}
            pallet_payment_streams::Event::LastChargeableInfoUpdated { .. } => {}
            pallet_payment_streams::Event::UserWithoutFunds { .. } => {}
            pallet_payment_streams::Event::UserPaidAllDebts { .. } => {}
            pallet_payment_streams::Event::UserPaidSomeDebts { .. } => {}
            pallet_payment_streams::Event::UserSolvent { .. } => {}
            pallet_payment_streams::Event::InconsistentTickProcessing { .. } => {}
            pallet_payment_streams::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_proofs_dealer_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_proofs_dealer::Event<Runtime>,
        block_hash: H256,
        block_number: NumberFor<Runtime::Block>,
    ) -> Result<(), IndexBlockError> {
        match event {
            pallet_proofs_dealer::Event::MutationsAppliedForProvider { .. } => {}
            pallet_proofs_dealer::Event::MutationsApplied {
                mutations,
                event_info,
                ..
            } => {
                // In StorageHub, we assume that all `MutationsApplied` events are emitted by bucket
                // root changes, and they should contain the encoded `BucketId` of the bucket that was mutated
                // in the `event_info` field.
                let Some(event_info) = event_info else {
                    error!(
                        target: LOG_TARGET,
                        "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated."
                    );
                    return Ok(());
                };

                let bucket_id = match self
                    .client
                    .runtime_api()
                    .decode_generic_apply_delta_event_info(block_hash, event_info.clone())
                {
                    Ok(runtime_api_result) => match runtime_api_result {
                        Ok(bucket_id) => bucket_id,
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to decode BucketId from event info: {:?}",
                                e
                            );
                            return Ok(());
                        }
                    },
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Error while calling runtime API to decode BucketId from event info: {:?}",
                            e
                        );
                        return Ok(());
                    }
                };

                let onchain_bucket_id = bucket_id.as_ref().to_vec();

                // Process each mutation to update file bucket membership
                for (file_key, mutation) in mutations {
                    let file_key_bytes = file_key.as_ref().to_vec();

                    // Index whether the file is in the bucket's forest based on the mutation type
                    // - `Add` mutation: file was added to the bucket's forest
                    // - `Remove` mutation: file was removed from the bucket's forest
                    let is_in_bucket = matches!(mutation, shc_common::types::TrieMutation::Add(_));
                    File::update_bucket_membership(
                        conn,
                        &file_key_bytes,
                        &onchain_bucket_id,
                        is_in_bucket,
                    )
                    .await
                    .map_err(|e| {
                        IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name:
                                "MutationsAppliedForProvider (update file bucket membership)"
                                    .to_string(),
                        }
                    })?;
                }
            }
            pallet_proofs_dealer::Event::NewChallenge { .. } => {}
            pallet_proofs_dealer::Event::NewPriorityChallenge { .. } => {}
            pallet_proofs_dealer::Event::ProofAccepted {
                provider_id: provider,
                proof: _proof,
                last_tick_proven,
            } => {
                let last_tick_proven: u64 = (*last_tick_proven).saturated_into();
                Bsp::update_last_tick_proven(
                    conn,
                    OnchainBspId::from(*provider),
                    last_tick_proven.saturated_into(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "ProofAccepted (update BSP last tick proven)".to_string(),
                })?;
            }
            pallet_proofs_dealer::Event::NewChallengeSeed { .. } => {}
            pallet_proofs_dealer::Event::NewCheckpointChallenge { .. } => {}
            pallet_proofs_dealer::Event::SlashableProvider { .. } => {}
            pallet_proofs_dealer::Event::NoRecordOfLastSubmittedProof { .. } => {}
            pallet_proofs_dealer::Event::NewChallengeCycleInitialised { .. } => {}
            pallet_proofs_dealer::Event::ChallengesTickerSet { .. } => {}
            pallet_proofs_dealer::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_providers_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_storage_providers::Event<Runtime>,
        block_hash: H256,
        block_number: NumberFor<Runtime::Block>,
    ) -> Result<(), IndexBlockError> {
        match event {
            pallet_storage_providers::Event::BspRequestSignUpSuccess { .. } => {}
            pallet_storage_providers::Event::BspSignUpSuccess {
                who,
                bsp_id,
                root,
                multiaddresses,
                capacity,
            } => {
                let stake = self
                    .client
                    .runtime_api()
                    .get_bsp_stake(block_hash, bsp_id)
                    .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                        anyhow_error: anyhow!("Runtime API error: {:?}", e),
                        block_number: block_number.saturated_into(),
                        event_name: "BspRequestSignUpSuccess (get BSP stake)".to_string(),
                    })?
                    .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                        anyhow_error: anyhow!("get_bsp_stake runtime API error: {:?}", e),
                        block_number: block_number.saturated_into(),
                        event_name: "BspRequestSignUpSuccess (get BSP stake)".to_string(),
                    })?
                    .into();

                let mut sql_multiaddresses = Vec::new();
                for multiaddress in multiaddresses {
                    if let Some(multiaddr) = convert_raw_multiaddress_to_multiaddr(multiaddress) {
                        sql_multiaddresses.push(
                            MultiAddress::create(conn, multiaddr.to_vec())
                                .await
                                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name: "BspRequestSignUpSuccess (create multiaddress)"
                                        .to_string(),
                                })?,
                        );
                    } else {
                        error!(target: LOG_TARGET, "Failed to parse multiaddr");
                    }
                }

                Bsp::create(
                    conn,
                    who.to_string(),
                    (*capacity).into(),
                    root.as_ref().to_vec(),
                    sql_multiaddresses,
                    OnchainBspId::new(*bsp_id),
                    stake,
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BspRequestSignUpSuccess (create BSP)".to_string(),
                })?;
            }
            pallet_storage_providers::Event::BspSignOffSuccess {
                who,
                bsp_id: _bsp_id,
            } => {
                Bsp::delete_by_account(conn, who.to_string())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "BspSignOffSuccess (delete BSP)".to_string(),
                    })?;
            }
            pallet_storage_providers::Event::CapacityChanged {
                who,
                new_capacity,
                provider_id,
                old_capacity: _old_capacity,
                next_block_when_change_allowed: _next_block_when_change_allowed,
            } => match provider_id {
                StorageProviderId::BackupStorageProvider(bsp_id) => {
                    Bsp::update_capacity(conn, who.to_string(), (*new_capacity).into())
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "CapacityChanged (update BSP capacity)".to_string(),
                        })?;

                    // update also the stake
                    let stake = self
                        .client
                        .runtime_api()
                        .get_bsp_stake(block_hash, bsp_id)
                        .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                            anyhow_error: anyhow!("get_bsp_stake runtime API error: {:?}", e),
                            block_number: block_number.saturated_into(),
                            event_name: "CapacityChanged (get BSP stake)".to_string(),
                        })?
                        .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                            anyhow_error: anyhow!("get_bsp_stake runtime API error: {:?}", e),
                            block_number: block_number.saturated_into(),
                            event_name: "CapacityChanged (get BSP stake)".to_string(),
                        })?
                        .into();

                    Bsp::update_stake(conn, OnchainBspId::from(*bsp_id), stake)
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "CapacityChanged (update BSP stake)".to_string(),
                        })?;
                }
                StorageProviderId::MainStorageProvider(_) => {
                    Bsp::update_capacity(conn, who.to_string(), (*new_capacity).into())
                        .await
                        .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                            database_error: e,
                            block_number: block_number.saturated_into(),
                            event_name: "CapacityChanged (update BSP capacity)".to_string(),
                        })?;
                }
            },
            pallet_storage_providers::Event::SignUpRequestCanceled { .. } => {}
            pallet_storage_providers::Event::MspRequestSignUpSuccess { .. } => {}
            pallet_storage_providers::Event::MspSignUpSuccess {
                who,
                msp_id,
                multiaddresses,
                capacity,
                value_prop,
            } => {
                let mut sql_multiaddresses = Vec::new();
                for multiaddress in multiaddresses {
                    if let Some(multiaddr) = convert_raw_multiaddress_to_multiaddr(multiaddress) {
                        sql_multiaddresses.push(
                            MultiAddress::create(conn, multiaddr.to_vec())
                                .await
                                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                                    database_error: e,
                                    block_number: block_number.saturated_into(),
                                    event_name: "MspRequestSignUpSuccess (create multiaddress)"
                                        .to_string(),
                                })?,
                        );
                    } else {
                        error!(target: LOG_TARGET, "Failed to parse multiaddr");
                    }
                }

                // TODO: update value prop after properly defined in runtime
                let value_prop = format!("{value_prop:?}");

                Msp::create(
                    conn,
                    who.to_string(),
                    (*capacity).into(),
                    value_prop,
                    sql_multiaddresses,
                    OnchainMspId::new(*msp_id),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "MspRequestSignUpSuccess (create MSP)".to_string(),
                })?;
            }
            pallet_storage_providers::Event::MspSignOffSuccess {
                who,
                msp_id: _msp_id,
            } => {
                Msp::delete_by_account(conn, who.to_string())
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspSignOffSuccess (delete MSP)".to_string(),
                    })?;
            }
            pallet_storage_providers::Event::BucketRootChanged {
                bucket_id,
                old_root: _,
                new_root,
            } => {
                Bucket::update_merkle_root(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    new_root.as_ref().to_vec(),
                )
                .await
                .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                    database_error: e,
                    block_number: block_number.saturated_into(),
                    event_name: "BucketRootChanged (update bucket merkle root)".to_string(),
                })?;
            }
            pallet_storage_providers::Event::Slashed { .. } => {}
            pallet_storage_providers::Event::AwaitingTopUp {
                provider_id,
                top_up_metadata: _top_up_metadata,
            } => {
                let stake = self
                    .client
                    .runtime_api()
                    .get_bsp_stake(block_hash, provider_id)
                    .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                        anyhow_error: anyhow!("get_bsp_stake runtime API error: {:?}", e),
                        block_number: block_number.saturated_into(),
                        event_name: "AwaitingTopUp (get BSP stake)".to_string(),
                    })?
                    .map_err(|e| IndexBlockError::EventIndexingRuntimeApiError {
                        anyhow_error: anyhow!("get_bsp_stake runtime API error: {:?}", e),
                        block_number: block_number.saturated_into(),
                        event_name: "AwaitingTopUp (get BSP stake)".to_string(),
                    })?
                    .into();

                Bsp::update_stake(conn, OnchainBspId::from(*provider_id), stake)
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "AwaitingTopUp (update BSP stake)".to_string(),
                    })?;
            }
            pallet_storage_providers::Event::TopUpFulfilled { .. } => {}
            pallet_storage_providers::Event::ValuePropAdded { .. } => {}
            pallet_storage_providers::Event::ValuePropUnavailable { .. } => {}
            pallet_storage_providers::Event::MultiAddressAdded { .. } => {}
            pallet_storage_providers::Event::MultiAddressRemoved { .. } => {}
            pallet_storage_providers::Event::ProviderInsolvent { .. } => {}
            pallet_storage_providers::Event::BucketsOfInsolventMsp { .. } => {
                // TODO: Should we index this? Since this buckets are all going to have moves requested
            }
            pallet_storage_providers::Event::MspDeleted { provider_id } => {
                Msp::delete(conn, OnchainMspId::from(*provider_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "MspDeleted (delete MSP)".to_string(),
                    })?;
            }
            pallet_storage_providers::Event::BspDeleted { provider_id } => {
                Bsp::delete(conn, OnchainBspId::from(*provider_id))
                    .await
                    .map_err(|e| IndexBlockError::EventIndexingDatabaseError {
                        database_error: e,
                        block_number: block_number.saturated_into(),
                        event_name: "BspDeleted (delete BSP)".to_string(),
                    })?;
            }
            pallet_storage_providers::Event::FailedToGetOwnerAccountOfInsolventProvider {
                ..
            } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_storage_providers::Event::FailedToSlashInsolventProvider { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_storage_providers::Event::FailedToStopAllCyclesForInsolventBsp { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_storage_providers::Event::FailedToInsertProviderTopUpExpiration { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_storage_providers::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_randomness_event<'a, 'b: 'a>(
        &'b self,
        _conn: &mut DbConnection<'a>,
        event: &pallet_randomness::Event<Runtime>,
        _block_number: NumberFor<Runtime::Block>,
    ) -> Result<(), IndexBlockError> {
        match event {
            pallet_randomness::Event::NewOneEpochAgoRandomnessAvailable { .. } => {}
            pallet_randomness::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }
}

// Define the EventLoop for IndexerService
pub struct IndexerServiceEventLoop<Runtime: StorageEnableRuntime> {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<IndexerServiceCommand>,
    actor: IndexerService<Runtime>,
}

enum MergedEventLoopMessage<Block>
where
    Block: sp_runtime::traits::Block,
{
    Command(IndexerServiceCommand),
    FinalityNotification(sc_client_api::FinalityNotification<Block>),
}

// Implement ActorEventLoop for IndexerServiceEventLoop
impl<Runtime: StorageEnableRuntime> ActorEventLoop<IndexerService<Runtime>>
    for IndexerServiceEventLoop<Runtime>
{
    fn new(
        actor: IndexerService<Runtime>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<IndexerServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "IndexerService starting up in {:?} mode!", self.actor.indexer_mode);

        let finality_notification_stream = self.actor.client.finality_notification_stream();

        let mut merged_stream = stream::select(
            self.receiver.map(MergedEventLoopMessage::Command),
            finality_notification_stream.map(MergedEventLoopMessage::FinalityNotification),
        );

        while let Some(message) = merged_stream.next().await {
            match message {
                MergedEventLoopMessage::Command(command) => {
                    self.actor.handle_message(command).await;
                }
                MergedEventLoopMessage::FinalityNotification(notification) => {
                    self.actor
                        .handle_finality_notification(notification)
                        .await
                        .unwrap_or_else(|e| {
                            error!(target: LOG_TARGET, "Failed to handle finality notification: {}", e);
                        });
                }
            }
        }

        info!(target: LOG_TARGET, "IndexerService shutting down.");
    }
}

#[derive(Error, Debug)]
pub enum IndexBlockError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Error indexing event [{event_name}] at block [{block_number}]. Database error: {database_error}")]
    EventIndexingDatabaseError {
        #[source]
        database_error: diesel::result::Error,
        block_number: u64,
        event_name: String,
    },
    #[error("Error indexing event [{event_name}] at block [{block_number}]. Runtime API error: {anyhow_error}")]
    EventIndexingRuntimeApiError {
        #[source]
        anyhow_error: anyhow::Error,
        block_number: u64,
        event_name: String,
    },
    #[error("Failed to retrieve or decode events: {0}")]
    EventsRetrievalError(#[from] EventsRetrievalError),
}

#[derive(Error, Debug)]
pub enum HandleFinalityNotificationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Block hash not found")]
    BlockHashNotFound,
    #[error("Index block error: {0}")]
    IndexBlockError(#[from] IndexBlockError),
    #[error("Client error: {0}")]
    ClientError(#[from] sp_blockchain::Error),
    #[error("Pool run error: {0}")]
    PoolRunError(#[from] diesel_async::pooled_connection::bb8::RunError),
}
