use diesel::prelude::*;
use diesel_async::AsyncConnection;
use futures::prelude::*;
use log::{error, info, trace, warn};
use shc_common::traits::{ReadOnlyKeystore, StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::{ProviderId, StorageProviderId};
use sp_runtime::AccountId32;
use std::sync::Arc;
use thiserror::Error;

use pallet_storage_providers_runtime_api::StorageProvidersApi;
use sc_client_api::{BlockBackend, BlockchainEvents};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::blockchain_utils::{
    convert_raw_multiaddress_to_multiaddr, get_provider_id_from_keystore, EventsRetrievalError,
    GetProviderIdError,
};
use shc_common::{
    blockchain_utils::get_events_at_block,
    types::{BlockNumber, ParachainClient},
};
use shc_indexer_db::{models::*, schema::msp, DbConnection, DbPool};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::Header;
use storage_hub_runtime::RuntimeEvent;

pub(crate) const LOG_TARGET: &str = "indexer-service";

// Since the indexed data should be used directly from the database,
// we don't need to implement commands.
#[derive(Debug)]
pub enum IndexerServiceCommand {}

// The IndexerService actor
pub struct IndexerService<RuntimeApi, K = Arc<dyn ReadOnlyKeystore>> {
    client: Arc<ParachainClient<RuntimeApi>>,
    db_pool: DbPool,
    indexer_mode: crate::IndexerMode,
    msp_id: Option<ProviderId>,
    keystore: K,
}

// Implement the Actor trait for IndexerService
impl<RuntimeApi, K> Actor for IndexerService<RuntimeApi, K>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    K: ReadOnlyKeystore + Send + Sync + 'static,
{
    type Message = IndexerServiceCommand;
    type EventLoop = IndexerServiceEventLoop<RuntimeApi, K>;
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
impl<RuntimeApi, K> IndexerService<RuntimeApi, K>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    K: ReadOnlyKeystore,
{
    pub fn new(
        client: Arc<ParachainClient<RuntimeApi>>,
        db_pool: DbPool,
        indexer_mode: crate::IndexerMode,
        keystore: K,
    ) -> Self {
        Self {
            client,
            db_pool,
            indexer_mode,
            msp_id: None,
            keystore,
        }
    }

    /// Synchronize the MSP ID from the keystore.
    ///
    /// This method detects the MSP ID based on the BCSV keys in the keystore
    /// and updates the `msp_id` field accordingly.
    fn sync_msp_id(&mut self, block_hash: &H256) {
        match get_provider_id_from_keystore(&self.client, &self.keystore, block_hash) {
            Ok(None) => {
                // No MSP ID found - expected for non-MSP nodes
                self.msp_id = None;
                info!(target: LOG_TARGET, "No MSP ID detected - running as non-MSP node");
            }
            Ok(Some(provider_id)) => {
                // Check if it's an MSP ID
                if let StorageProviderId::MainStorageProvider(msp_id) = provider_id {
                    self.msp_id = Some(msp_id);
                    info!(target: LOG_TARGET, "MSP ID detected: {:?}", provider_id);
                } else {
                    // It's a BSP ID, not an MSP
                    self.msp_id = None;
                    info!(target: LOG_TARGET, "BSP ID detected: {:?} - running as non-MSP node", provider_id);
                }
            }
            Err(GetProviderIdError::MultipleProviderIds) => {
                // Configuration issue - multiple provider IDs found
                error!(target: LOG_TARGET, "Multiple provider IDs found in keystore - this is a configuration issue");
                self.msp_id = None;
            }
            Err(e) => {
                // Runtime API error
                error!(target: LOG_TARGET, "Failed to get provider ID from keystore: {:?}", e);
                self.msp_id = None;
            }
        }
    }

    /// Check if a bucket belongs to the current MSP.
    ///
    /// Used in lite mode only for events requiring ownership filtering:
    /// - BucketPrivacyUpdated
    /// - BucketDeleted
    /// - BucketRootChanged
    async fn check_bucket_belongs_to_current_msp<'a>(
        &self,
        conn: &mut DbConnection<'a>,
        bucket_id: Vec<u8>,
        current_msp_id: &ProviderId,
    ) -> bool {
        match Bucket::get_by_onchain_bucket_id(conn, bucket_id).await {
            Ok(bucket) => {
                // Check if bucket has an MSP assigned
                if let Some(msp_id) = bucket.msp_id {
                    // Get the MSP for this bucket from DB by its database ID
                    match diesel_async::RunQueryDsl::first::<Msp>(
                        msp::table.filter(msp::id.eq(msp_id)),
                        conn,
                    )
                    .await
                    {
                        Ok(msp) => msp.onchain_msp_id == current_msp_id.to_string(),
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Check if a file belongs to a bucket managed by the current MSP.
    ///
    /// Used in lite mode only for events requiring ownership filtering:
    /// - ProofSubmittedForPendingFileDeletionRequest
    /// - MspAcceptedStorageRequest
    /// - StorageRequestRejected
    async fn check_file_belongs_to_current_msp<'a>(
        &self,
        conn: &mut DbConnection<'a>,
        file_key: Vec<u8>,
        current_msp_id: &ProviderId,
    ) -> bool {
        match File::get_by_file_key(conn, file_key).await {
            Ok(file) => {
                // Get the bucket for this file
                match Bucket::get_by_id(conn, file.bucket_id).await {
                    Ok(bucket) => {
                        // Check if bucket has an MSP assigned
                        if let Some(msp_id) = bucket.msp_id {
                            // Get the MSP for this bucket from DB by its database ID
                            match diesel_async::RunQueryDsl::first::<Msp>(
                                msp::table.filter(msp::id.eq(msp_id)),
                                conn,
                            )
                            .await
                            {
                                Ok(msp) => msp.onchain_msp_id == current_msp_id.to_string(),
                                Err(_) => false,
                            }
                        } else {
                            false
                        }
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    async fn handle_finality_notification<Block>(
        &mut self,
        notification: sc_client_api::FinalityNotification<Block>,
    ) -> Result<(), HandleFinalityNotificationError>
    where
        Block: sp_runtime::traits::Block<Hash = H256>,
        Block::Header: Header<Number = BlockNumber>,
    {
        let finalized_block_hash = notification.hash;
        let finalized_block_number = *notification.header.number();

        info!(target: LOG_TARGET, "Finality notification (#{}): {}", finalized_block_number, finalized_block_hash);

        // In Lite mode, sync MSP ID on each finality notification
        if self.indexer_mode == crate::IndexerMode::Lite {
            self.sync_msp_id(&finalized_block_hash);
        }

        let mut db_conn = self.db_pool.get().await?;

        let service_state = ServiceState::get(&mut db_conn).await?;

        // Collect block hashes first to avoid borrowing issues
        let mut blocks_to_index = Vec::new();
        for block_number in
            (service_state.last_processed_block as BlockNumber + 1)..=finalized_block_number
        {
            let block_hash = self
                .client
                .block_hash(block_number)?
                .ok_or(HandleFinalityNotificationError::BlockHashNotFound)?;
            blocks_to_index.push((block_number, block_hash));
        }

        // Now index the blocks
        for (block_number, block_hash) in blocks_to_index {
            self.index_block(&mut db_conn, block_number as BlockNumber, block_hash)
                .await?;
            
            // In lite mode, sync MSP ID after each block in case we just processed our MSP registration
            if self.indexer_mode == crate::IndexerMode::Lite && self.msp_id.is_none() {
                self.sync_msp_id(&block_hash);
                if self.msp_id.is_some() {
                    info!(target: LOG_TARGET, "MSP ID detected after processing block #{}: {:?}", block_number, self.msp_id);
                }
            }
        }

        Ok(())
    }

    async fn index_block<'a, 'b: 'a>(
        &'b mut self,
        conn: &mut DbConnection<'a>,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<(), IndexBlockError> {
        info!(target: LOG_TARGET, "Indexing block #{}: {}", block_number, block_hash);

        let block_events = get_events_at_block(&self.client, &block_hash)?;

        conn.transaction::<(), IndexBlockError, _>(move |conn| {
            Box::pin(async move {
                ServiceState::update(conn, block_number as i64).await?;

                for ev in block_events {
                    self.route_event(conn, &ev.event, block_hash).await?;
                }

                Ok(())
            })
        })
        .await?;

        Ok(())
    }

    async fn route_event<'a, 'b: 'a>(
        &'b mut self,
        conn: &mut DbConnection<'a>,
        event: &RuntimeEvent,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match self.indexer_mode {
            crate::IndexerMode::Full => self.index_event(conn, event, block_hash).await,
            crate::IndexerMode::Lite => self.index_event_lite(conn, event, block_hash).await,
        }
    }

    async fn index_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &RuntimeEvent,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match event {
            RuntimeEvent::BucketNfts(event) => self.index_bucket_nfts_event(conn, event).await?,
            RuntimeEvent::FileSystem(event) => self.index_file_system_event(conn, event).await?,
            RuntimeEvent::PaymentStreams(event) => {
                self.index_payment_streams_event(conn, event).await?
            }
            RuntimeEvent::ProofsDealer(event) => {
                self.index_proofs_dealer_event(conn, event).await?
            }
            RuntimeEvent::Providers(event) => {
                self.index_providers_event(conn, event, block_hash).await?
            }
            RuntimeEvent::Randomness(event) => self.index_randomness_event(conn, event).await?,
            // TODO: We have to index the events from the CrRandomness pallet when we integrate it to the runtime,
            // since they contain the information about the commit-reveal deadlines for Providers.
            // RuntimeEvent::CrRandomness(event) => self.index_cr_randomness_event(conn, event).await?,
            // Runtime events that we're not interested in.
            // We add them here instead of directly matching (_ => {})
            // to ensure the compiler will let us know to treat future events when added.
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
        }

        Ok(())
    }

    async fn index_event_lite<'a, 'b: 'a>(
        &'b mut self,
        conn: &mut DbConnection<'a>,
        event: &RuntimeEvent,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match event {
            RuntimeEvent::FileSystem(event) => {
                // Only process FileSystem events if we have an MSP ID
                if self.msp_id.is_none() {
                    trace!(target: LOG_TARGET, "No MSP ID configured, skipping FileSystem event in lite mode");
                    return Ok(());
                }
                self.index_file_system_event_lite(conn, event).await?
            }
            RuntimeEvent::Providers(event) => {
                // Always process Provider events - they might contain our MSP registration
                self.index_providers_event_lite(conn, event, block_hash)
                    .await?
            }
            // Explicitly ignore other pallets in lite mode
            RuntimeEvent::BucketNfts(_) => {
                trace!(target: LOG_TARGET, "Ignoring BucketNfts event in lite mode");
            }
            RuntimeEvent::PaymentStreams(_) => {
                trace!(target: LOG_TARGET, "Ignoring PaymentStreams event in lite mode");
            }
            RuntimeEvent::ProofsDealer(_) => {
                trace!(target: LOG_TARGET, "Ignoring ProofsDealer event in lite mode");
            }
            RuntimeEvent::Randomness(_) => {
                trace!(target: LOG_TARGET, "Ignoring Randomness event in lite mode");
            }
            // System pallets - explicitly list all to ensure compilation errors on new events
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
        }

        Ok(())
    }

    async fn index_bucket_nfts_event<'a, 'b: 'a>(
        &'b self,
        _conn: &mut DbConnection<'a>,
        event: &pallet_bucket_nfts::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
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
        event: &pallet_file_system::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            pallet_file_system::Event::NewBucket {
                who,
                msp_id,
                bucket_id,
                name,
                collection_id,
                private,
                value_prop_id: _,
                root,
            } => {
                // Get the existing MSP - it should have been created during MSP registration
                let msp = Some(Msp::get_by_onchain_msp_id(conn, msp_id.to_string()).await?);

                Bucket::create(
                    conn,
                    msp.map(|m| m.id),
                    who.to_string(),
                    bucket_id.as_ref().to_vec(),
                    name.to_vec(),
                    collection_id.map(|id| id.to_string()),
                    *private,
                    root.as_ref().to_vec(),
                )
                .await?;
            }
            pallet_file_system::Event::MoveBucketAccepted {
                old_msp_id: _,
                new_msp_id,
                bucket_id,
                value_prop_id: _,
            } => {
                // Get the existing MSP - it should have been created during MSP registration
                let new_msp = Msp::get_by_onchain_msp_id(conn, new_msp_id.to_string()).await?;
                Bucket::update_msp(conn, bucket_id.as_ref().to_vec(), new_msp.id).await?;
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
                .await?;
            }
            pallet_file_system::Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key: _,
                new_root,
            } => {
                // Ensure BSP exists before updating merkle root
                let _ = Bsp::get_by_onchain_bsp_id(conn, bsp_id.to_string()).await?;
                Bsp::update_merkle_root(conn, bsp_id.to_string(), new_root.as_ref().to_vec())
                    .await?;
            }
            pallet_file_system::Event::BspConfirmedStoring {
                who: _,
                bsp_id,
                confirmed_file_keys,
                skipped_file_keys: _,
                new_root,
            } => {
                // Get the existing BSP - it should have been created during BSP registration
                let bsp = Bsp::get_by_onchain_bsp_id(conn, bsp_id.to_string()).await?;

                // Update the merkle root for the BSP
                Bsp::update_merkle_root(conn, bsp_id.to_string(), new_root.as_ref().to_vec())
                    .await?;

                for file_key in confirmed_file_keys {
                    let file = File::get_by_file_key(conn, file_key.as_ref().to_vec()).await?;
                    BspFile::create(conn, bsp.id, file.id).await?;
                }
            }
            pallet_file_system::Event::NewStorageRequest {
                who,
                file_key,
                bucket_id,
                location,
                fingerprint,
                size,
                peer_ids,
                expires_at: _,
            } => {
                let bucket =
                    Bucket::get_by_onchain_bucket_id(conn, bucket_id.as_ref().to_vec()).await?;

                let mut sql_peer_ids = Vec::new();
                for peer_id in peer_ids {
                    sql_peer_ids.push(PeerId::create(conn, peer_id.to_vec()).await?);
                }

                File::create(
                    conn,
                    <AccountId32 as AsRef<[u8]>>::as_ref(who).to_vec(),
                    file_key.as_ref().to_vec(),
                    bucket.id,
                    location.to_vec(),
                    fingerprint.as_ref().to_vec(),
                    *size as i64,
                    FileStorageRequestStep::Requested,
                    sql_peer_ids,
                )
                .await?;
            }
            pallet_file_system::Event::MoveBucketRequested { .. } => {}
            pallet_file_system::Event::NewCollectionAndAssociation { .. } => {}
            pallet_file_system::Event::AcceptedBspVolunteer {
                bsp_id: _,
                bucket_id: _,
                location: _,
                fingerprint: _,
                multiaddresses: _,
                owner: _,
                size: _,
            } => {
                // TODO: Implement AcceptedBspVolunteer event handling logic
                // This event is indexed in lite mode but the implementation will be added later
            }
            pallet_file_system::Event::StorageRequestFulfilled { file_key } => {
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Stored,
                )
                .await?;
            }
            pallet_file_system::Event::StorageRequestExpired { file_key } => {
                File::update_step(
                    conn,
                    file_key.as_ref().to_vec(),
                    FileStorageRequestStep::Stored,
                )
                .await?;
            }
            pallet_file_system::Event::StorageRequestRevoked { file_key } => {
                File::delete(conn, file_key.as_ref().to_vec()).await?;
            }
            pallet_file_system::Event::MspAcceptedStorageRequest { .. } => {}
            pallet_file_system::Event::StorageRequestRejected { .. } => {}
            pallet_file_system::Event::BspRequestedToStopStoring { .. } => {}
            pallet_file_system::Event::PriorityChallengeForFileDeletionQueued { .. } => {}
            pallet_file_system::Event::MspStopStoringBucketInsolventUser { .. } => {
                // TODO: Index this
            }
            pallet_file_system::Event::SpStopStoringInsolventUser {
                sp_id,
                file_key,
                owner: _,
                location: _,
                new_root: _,
            } => {
                // We are now only deleting for BSP as BSP are associating with files
                // MSP will handle insolvent user at the level of buckets (an MSP will delete the full bucket for an insolvent user and it will produce a new kind of event)
                BspFile::delete(conn, file_key, sp_id.to_string()).await?;
            }
            pallet_file_system::Event::FailedToQueuePriorityChallenge { .. } => {}
            pallet_file_system::Event::FileDeletionRequest { .. } => {}
            pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest { .. } => {}
            pallet_file_system::Event::BspChallengeCycleInitialised { .. } => {}
            pallet_file_system::Event::MoveBucketRequestExpired { .. } => {}
            pallet_file_system::Event::MoveBucketRejected { .. } => {}
            pallet_file_system::Event::MspStoppedStoringBucket { .. } => {}
            pallet_file_system::Event::BucketDeleted {
                who: _,
                bucket_id,
                maybe_collection_id: _,
            } => {
                Bucket::delete(conn, bucket_id.as_ref().to_vec()).await?;
            }
            pallet_file_system::Event::FailedToGetMspOfBucket { .. } => {}
            pallet_file_system::Event::FailedToDecreaseMspUsedCapacity { .. } => {}
            pallet_file_system::Event::UsedCapacityShouldBeZero { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_file_system::Event::FailedToReleaseStorageRequestCreationDeposit { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_file_system::Event::FailedToTransferDepositFundsToBsp { .. } => {
                // In the future we should monitor for this to detect eventual bugs in the pallets
            }
            pallet_file_system::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_file_system_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_file_system::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
        // We can safely unwrap msp_id here since the caller already checked it
        let current_msp_id = self.msp_id.as_ref().unwrap();

        // Filter events based on MSP relevance
        let should_index = match event {
            pallet_file_system::Event::NewBucket { msp_id, .. } => {
                // Index ALL buckets regardless of MSP ownership
                true
            }
            pallet_file_system::Event::MoveBucketAccepted {
                bucket_id,
                new_msp_id,
                ..
            } => {
                // Index if bucket exists in DB (was previously with current MSP) OR new MSP is current MSP
                if new_msp_id == current_msp_id {
                    true
                } else {
                    // Check if bucket exists in DB (meaning it was previously with current MSP)
                    match Bucket::get_by_onchain_bucket_id(conn, bucket_id.as_ref().to_vec()).await
                    {
                        Ok(_) => true,   // Bucket exists, so it was previously with current MSP
                        Err(_) => false, // Bucket doesn't exist in DB
                    }
                }
            }
            pallet_file_system::Event::NewStorageRequest { bucket_id, .. } => {
                // Index ALL storage requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::StorageRequestFulfilled { file_key } => {
                // Index ALL fulfilled storage requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::StorageRequestExpired { file_key } => {
                // Index ALL expired storage requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::StorageRequestRevoked { file_key } => {
                // Index ALL revoked storage requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::BucketPrivacyUpdated { bucket_id, .. } => {
                // Only index if bucket belongs to current MSP
                self.check_bucket_belongs_to_current_msp(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            pallet_file_system::Event::BucketDeleted { bucket_id, .. } => {
                // Only index if bucket belongs to current MSP
                self.check_bucket_belongs_to_current_msp(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            pallet_file_system::Event::MoveBucketRequested { bucket_id, .. } => {
                // Index ALL move bucket requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::MoveBucketRejected { bucket_id, .. } => {
                // Index ALL move bucket rejections regardless of MSP ownership
                true
            }
            pallet_file_system::Event::MspStoppedStoringBucket { msp_id, .. } => {
                // Only index if it's the current MSP
                msp_id == current_msp_id
            }
            pallet_file_system::Event::MspStopStoringBucketInsolventUser { msp_id, .. } => {
                // Only index if it's the current MSP
                msp_id == current_msp_id
            }
            pallet_file_system::Event::FileDeletionRequest { file_key, .. } => {
                // Index ALL file deletion requests regardless of MSP ownership
                true
            }
            pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest {
                file_key,
                ..
            } => {
                // Check if file belongs to current MSP's bucket
                self.check_file_belongs_to_current_msp(
                    conn,
                    file_key.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            pallet_file_system::Event::MoveBucketRequestExpired { bucket_id, .. } => {
                // Index ALL move bucket request expirations regardless of MSP ownership
                true
            }
            pallet_file_system::Event::MspAcceptedStorageRequest { file_key, .. } => {
                // Check if file belongs to current MSP's bucket
                self.check_file_belongs_to_current_msp(
                    conn,
                    file_key.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            pallet_file_system::Event::StorageRequestRejected { file_key, .. } => {
                // Check if file belongs to current MSP's bucket
                self.check_file_belongs_to_current_msp(
                    conn,
                    file_key.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            // BSP-specific events and others remain filtered out
            pallet_file_system::Event::NewCollectionAndAssociation { .. } => false,
            pallet_file_system::Event::AcceptedBspVolunteer { .. } => true, // Enable BSP volunteering indexing
            pallet_file_system::Event::BspRequestedToStopStoring { .. } => false,
            pallet_file_system::Event::BspConfirmStoppedStoring { .. } => false,
            pallet_file_system::Event::BspConfirmedStoring { .. } => true, // Enable BSP confirmation indexing
            pallet_file_system::Event::PriorityChallengeForFileDeletionQueued { .. } => false,
            pallet_file_system::Event::SpStopStoringInsolventUser { .. } => false,
            pallet_file_system::Event::FailedToQueuePriorityChallenge { .. } => false,
            pallet_file_system::Event::BspChallengeCycleInitialised { .. } => false,
            pallet_file_system::Event::FailedToGetMspOfBucket { .. } => false,
            pallet_file_system::Event::FailedToDecreaseMspUsedCapacity { .. } => false,
            pallet_file_system::Event::UsedCapacityShouldBeZero { .. } => false,
            pallet_file_system::Event::FailedToReleaseStorageRequestCreationDeposit { .. } => false,
            pallet_file_system::Event::FailedToTransferDepositFundsToBsp { .. } => false,
            pallet_file_system::Event::__Ignore(_, _) => false,
        };

        if should_index {
            // Delegate to the original method
            self.index_file_system_event(conn, event).await
        } else {
            trace!(target: LOG_TARGET, "Filtered out FileSystem event in lite mode for MSP {:?}", current_msp_id);
            Ok(())
        }
    }

    async fn index_payment_streams_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_payment_streams::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            pallet_payment_streams::Event::DynamicRatePaymentStreamCreated {
                provider_id,
                user_account,
                amount_provided: _amount_provided,
            } => {
                PaymentStream::create(conn, user_account.to_string(), provider_id.to_string())
                    .await?;
            }
            pallet_payment_streams::Event::DynamicRatePaymentStreamUpdated { .. } => {
                // TODO: Currently we are not treating the info of dynamic rate update
            }
            pallet_payment_streams::Event::DynamicRatePaymentStreamDeleted { .. } => {}
            pallet_payment_streams::Event::FixedRatePaymentStreamCreated {
                provider_id,
                user_account,
                rate: _rate,
            } => {
                PaymentStream::create(conn, user_account.to_string(), provider_id.to_string())
                    .await?;
            }
            pallet_payment_streams::Event::FixedRatePaymentStreamUpdated { .. } => {
                // TODO: Currently we are not treating the info of fixed rate update
            }
            pallet_payment_streams::Event::FixedRatePaymentStreamDeleted { .. } => {}
            pallet_payment_streams::Event::PaymentStreamCharged {
                user_account,
                provider_id,
                amount,
                last_tick_charged,
                charged_at_tick,
            } => {
                // We want to handle this and update the payment stream total amount
                let ps =
                    PaymentStream::get(conn, user_account.to_string(), provider_id.to_string())
                        .await?;
                let new_total_amount = ps.total_amount_paid + amount;
                let last_tick_charged: i64 = (*last_tick_charged).into();
                let charged_at_tick: i64 = (*charged_at_tick).into();
                PaymentStream::update_total_amount(
                    conn,
                    ps.id,
                    new_total_amount,
                    last_tick_charged,
                    charged_at_tick,
                )
                .await?;
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
        event: &pallet_proofs_dealer::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            pallet_proofs_dealer::Event::MutationsAppliedForProvider { .. } => {}
            pallet_proofs_dealer::Event::MutationsApplied { .. } => {}
            pallet_proofs_dealer::Event::NewChallenge { .. } => {}
            pallet_proofs_dealer::Event::ProofAccepted {
                provider_id: provider,
                proof: _proof,
                last_tick_proven,
            } => {
                // Ensure BSP exists before updating last tick proven
                let _ = Bsp::get_by_onchain_bsp_id(conn, provider.to_string()).await?;
                Bsp::update_last_tick_proven(
                    conn,
                    provider.to_string(),
                    (*last_tick_proven).into(),
                )
                .await?;
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
        event: &pallet_storage_providers::Event<storage_hub_runtime::Runtime>,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
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
                    .expect("to have a stake")
                    .unwrap_or(Default::default())
                    .into();

                let mut sql_multiaddresses = Vec::new();
                for multiaddress in multiaddresses {
                    if let Some(multiaddr) = convert_raw_multiaddress_to_multiaddr(multiaddress) {
                        sql_multiaddresses
                            .push(MultiAddress::create(conn, multiaddr.to_vec()).await?);
                    } else {
                        error!(target: LOG_TARGET, "Failed to parse multiaddr");
                    }
                }

                Bsp::create(
                    conn,
                    who.to_string(),
                    capacity.into(),
                    root.as_ref().to_vec(),
                    sql_multiaddresses,
                    bsp_id.to_string(),
                    stake,
                )
                .await?;
            }
            pallet_storage_providers::Event::BspSignOffSuccess {
                who,
                bsp_id: _bsp_id,
            } => {
                Bsp::delete(conn, who.to_string()).await?;
            }
            pallet_storage_providers::Event::CapacityChanged {
                who,
                new_capacity,
                provider_id,
                old_capacity: _old_capacity,
                next_block_when_change_allowed: _next_block_when_change_allowed,
            } => match provider_id {
                StorageProviderId::BackupStorageProvider(bsp_id) => {
                    Bsp::update_capacity(conn, who.to_string(), new_capacity.into()).await?;

                    // update also the stake
                    let stake = self
                        .client
                        .runtime_api()
                        .get_bsp_stake(block_hash, bsp_id)
                        .expect("to have a stake")
                        .unwrap_or(Default::default())
                        .into();

                    Bsp::update_stake(conn, bsp_id.to_string(), stake).await?;
                }
                StorageProviderId::MainStorageProvider(_msp_id) => {
                    Bsp::update_capacity(conn, who.to_string(), new_capacity.into()).await?;
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
                        sql_multiaddresses
                            .push(MultiAddress::create(conn, multiaddr.to_vec()).await?);
                    } else {
                        error!(target: LOG_TARGET, "Failed to parse multiaddr");
                    }
                }

                // TODO: update value prop after properly defined in runtime
                let value_prop = format!("{value_prop:?}");

                Msp::create(
                    conn,
                    who.to_string(),
                    capacity.into(),
                    value_prop,
                    sql_multiaddresses,
                    msp_id.to_string(),
                )
                .await?;
            }
            pallet_storage_providers::Event::MspSignOffSuccess { who, msp_id: _ } => {
                Msp::delete(conn, who.to_string()).await?;
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
                .await?;
            }
            pallet_storage_providers::Event::Slashed { .. } => {}
            pallet_storage_providers::Event::AwaitingTopUp {
                provider_id,
                top_up_metadata: _top_up_metadata,
            } => {
                // Get the existing BSP before updating stake
                let _ = Bsp::get_by_onchain_bsp_id(conn, provider_id.to_string()).await?;

                let stake = self
                    .client
                    .runtime_api()
                    .get_bsp_stake(block_hash, provider_id)
                    .expect("to have a stake")
                    .unwrap_or(Default::default())
                    .into();

                Bsp::update_stake(conn, provider_id.to_string(), stake).await?;
            }
            pallet_storage_providers::Event::TopUpFulfilled { .. } => {}
            pallet_storage_providers::Event::ValuePropAdded { .. } => {}
            pallet_storage_providers::Event::ValuePropUnavailable { .. } => {}
            pallet_storage_providers::Event::MultiAddressAdded { provider_id: _, .. } => {
                // TODO: Handle multi address addition
            }
            pallet_storage_providers::Event::MultiAddressRemoved { provider_id: _, .. } => {
                // TODO: Handle multi address removal
            }
            pallet_storage_providers::Event::ProviderInsolvent { .. } => {}
            pallet_storage_providers::Event::BucketsOfInsolventMsp { .. } => {
                // TODO: Should we index this? Since this buckets are all going to have moves requested
            }
            pallet_storage_providers::Event::MspDeleted { provider_id } => {
                Msp::delete(conn, provider_id.to_string()).await?;
            }
            pallet_storage_providers::Event::BspDeleted { provider_id } => {
                Bsp::delete(conn, provider_id.to_string()).await?;
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

    async fn index_providers_event_lite<'a, 'b: 'a>(
        &'b mut self,
        conn: &mut DbConnection<'a>,
        event: &pallet_storage_providers::Event<storage_hub_runtime::Runtime>,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        // Always index provider registration events to ensure MSPs and BSPs exist
        match event {
            pallet_storage_providers::Event::MspSignUpSuccess { .. } => {
                info!(target: LOG_TARGET, "Indexing MspSignUpSuccess event in lite mode");
                return self.index_providers_event(conn, event, block_hash).await;
            }
            pallet_storage_providers::Event::BspSignUpSuccess { .. } => {
                info!(target: LOG_TARGET, "Indexing BspSignUpSuccess event in lite mode");
                return self.index_providers_event(conn, event, block_hash).await;
            }
            _ => {}
        }

        // For all other events, we need an MSP ID
        if self.msp_id.is_none() {
            trace!(target: LOG_TARGET, "No MSP ID configured, skipping Providers event in lite mode");
            return Ok(());
        }

        let current_msp_id = self.msp_id.as_ref().unwrap();

        // Filter events based on MSP relevance
        let should_index = match event {
            // MSP-specific events - only index if it's for our MSP
            pallet_storage_providers::Event::MspSignUpSuccess { msp_id, .. } => {
                *msp_id == *current_msp_id
            }
            pallet_storage_providers::Event::MspSignOffSuccess { msp_id, .. } => {
                *msp_id == *current_msp_id
            }
            pallet_storage_providers::Event::CapacityChanged { provider_id, .. } => {
                match provider_id {
                    StorageProviderId::MainStorageProvider(msp_id) => *msp_id == *current_msp_id,
                    StorageProviderId::BackupStorageProvider(_) => false,
                }
            }
            pallet_storage_providers::Event::MultiAddressAdded { provider_id, .. } => {
                *provider_id == *current_msp_id
            }
            pallet_storage_providers::Event::MultiAddressRemoved { provider_id, .. } => {
                *provider_id == *current_msp_id
            }
            // MSP deletion - only index if it's the current MSP
            pallet_storage_providers::Event::MspDeleted { provider_id, .. } => {
                *provider_id == *current_msp_id
            }
            // Provider insolvency - only index if it's the current MSP
            pallet_storage_providers::Event::ProviderInsolvent { provider_id, .. } => {
                *provider_id == *current_msp_id
            }
            // Buckets of insolvent MSP - only index if it's the current MSP
            pallet_storage_providers::Event::BucketsOfInsolventMsp { msp_id, .. } => {
                *msp_id == *current_msp_id
            }
            // Service offering events - only index for current MSP
            pallet_storage_providers::Event::ValuePropAdded { msp_id, .. } => {
                *msp_id == *current_msp_id
            }
            pallet_storage_providers::Event::ValuePropUnavailable { msp_id, .. } => {
                *msp_id == *current_msp_id
            }
            // Financial events - only index for current MSP
            pallet_storage_providers::Event::Slashed { provider_id, .. } => {
                *provider_id == *current_msp_id
            }
            pallet_storage_providers::Event::TopUpFulfilled { provider_id, .. } => {
                // Only index top-ups for current MSP
                *provider_id == *current_msp_id
            }
            pallet_storage_providers::Event::BucketRootChanged { bucket_id, .. } => {
                // Only index if bucket belongs to current MSP
                self.check_bucket_belongs_to_current_msp(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    current_msp_id,
                )
                .await
            }
            // MSP request sign up - can't determine if it's for current MSP from event data
            pallet_storage_providers::Event::MspRequestSignUpSuccess { .. } => {
                // TODO: The event only contains 'who' (AccountId), not MSP ID
                // We can't filter this properly without additional context
                false
            }
            // BSP-specific and other events remain filtered out
            pallet_storage_providers::Event::BspRequestSignUpSuccess { .. } => false,
            pallet_storage_providers::Event::BspSignUpSuccess { .. } => {
                // This is handled separately above, but we need to include it for exhaustiveness
                false
            }
            pallet_storage_providers::Event::BspSignOffSuccess { .. } => false,
            pallet_storage_providers::Event::SignUpRequestCanceled { .. } => false,
            pallet_storage_providers::Event::AwaitingTopUp { .. } => false,
            pallet_storage_providers::Event::BspDeleted { .. } => false,
            pallet_storage_providers::Event::FailedToGetOwnerAccountOfInsolventProvider {
                ..
            } => false,
            pallet_storage_providers::Event::FailedToSlashInsolventProvider { .. } => false,
            pallet_storage_providers::Event::FailedToStopAllCyclesForInsolventBsp { .. } => false,
            pallet_storage_providers::Event::FailedToInsertProviderTopUpExpiration { .. } => false,
            pallet_storage_providers::Event::__Ignore(_, _) => false,
        };

        if should_index {
            // Delegate to the original method
            self.index_providers_event(conn, event, block_hash).await
        } else {
            trace!(target: LOG_TARGET, "Filtered out Providers event in lite mode for MSP {:?}", current_msp_id);
            Ok(())
        }
    }

    async fn index_randomness_event<'a, 'b: 'a>(
        &'b self,
        _conn: &mut DbConnection<'a>,
        event: &pallet_randomness::Event<storage_hub_runtime::Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            pallet_randomness::Event::NewOneEpochAgoRandomnessAvailable { .. } => {}
            pallet_randomness::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }
}

// Define the EventLoop for IndexerService
pub struct IndexerServiceEventLoop<RuntimeApi, K> {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<IndexerServiceCommand>,
    actor: IndexerService<RuntimeApi, K>,
}

enum MergedEventLoopMessage<Block>
where
    Block: sp_runtime::traits::Block,
{
    Command(IndexerServiceCommand),
    FinalityNotification(sc_client_api::FinalityNotification<Block>),
}

// Implement ActorEventLoop for IndexerServiceEventLoop
impl<RuntimeApi, K> ActorEventLoop<IndexerService<RuntimeApi, K>>
    for IndexerServiceEventLoop<RuntimeApi, K>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    K: ReadOnlyKeystore + Send + Sync + 'static,
{
    fn new(
        actor: IndexerService<RuntimeApi, K>,
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
