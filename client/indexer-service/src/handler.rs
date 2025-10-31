use bigdecimal::BigDecimal;
use codec::Encode;
use diesel_async::AsyncConnection;
use futures::prelude::*;
use log::{error, info};
use std::sync::Arc;
use thiserror::Error;

use pallet_storage_providers_runtime_api::StorageProvidersApi;
use sc_client_api::{BlockBackend, BlockchainEvents};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::{
        convert_raw_multiaddress_to_multiaddr, get_events_at_block, EventsRetrievalError,
    },
    traits::StorageEnableRuntime,
    types::{ParachainClient, StorageEnableEvents, StorageProviderId},
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
    client: Arc<ParachainClient<Runtime::RuntimeApi>>,
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
        client: Arc<ParachainClient<Runtime::RuntimeApi>>,
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

        conn.transaction::<(), IndexBlockError, _>(move |conn| {
            Box::pin(async move {
                let block_number_u64: u64 = block_number.saturated_into();
                let block_number_i64: i64 = block_number_u64 as i64;
                ServiceState::update(conn, block_number_i64).await?;

                for ev in block_events {
                    self.route_event(conn, &ev.event.into(), block_hash).await?;
                }

                Ok(())
            })
        })
        .await?;

        Ok(())
    }

    async fn route_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match self.indexer_mode {
            crate::IndexerMode::Full => self.index_event(conn, event, block_hash).await,
            crate::IndexerMode::Lite => self.index_event_lite(conn, event, block_hash).await,
            crate::IndexerMode::Fishing => self.index_event_fishing(conn, event, block_hash).await,
        }
    }

    async fn index_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: H256,
    ) -> Result<(), diesel::result::Error> {
        match event {
            StorageEnableEvents::BucketNfts(event) => {
                self.index_bucket_nfts_event(conn, event).await?
            }
            StorageEnableEvents::FileSystem(event) => {
                self.index_file_system_event(conn, event).await?
            }
            StorageEnableEvents::PaymentStreams(event) => {
                self.index_payment_streams_event(conn, event).await?
            }
            StorageEnableEvents::ProofsDealer(event) => {
                self.index_proofs_dealer_event(conn, event).await?
            }
            StorageEnableEvents::StorageProviders(event) => {
                self.index_providers_event(conn, event, block_hash).await?
            }
            StorageEnableEvents::Randomness(event) => {
                self.index_randomness_event(conn, event).await?
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
        event: &pallet_file_system::Event<Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
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
                let msp =
                    Some(Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id)).await?);

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
                .await?;
            }
            pallet_file_system::Event::MoveBucketAccepted {
                old_msp_id,
                new_msp_id,
                bucket_id,
                value_prop_id: _,
            } => {
                let old_msp = if let Some(id) = old_msp_id {
                    Some(Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*id)).await?)
                } else {
                    None
                };
                let new_msp =
                    Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*new_msp_id)).await?;

                // Handle MSP-file associations based on whether old_msp exists
                if let Some(old_msp) = old_msp {
                    // Update existing associations from old to new MSP
                    MspFile::update_msp_for_bucket(
                        conn,
                        bucket_id.as_ref(),
                        old_msp.id,
                        new_msp.id,
                    )
                    .await?;
                } else {
                    // Create new associations for all files in the bucket
                    MspFile::create_for_bucket(conn, bucket_id.as_ref(), new_msp.id).await?;
                }

                // Update bucket's MSP reference
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
                file_key,
                new_root,
            } => {
                Bsp::update_merkle_root(
                    conn,
                    OnchainBspId::from(*bsp_id),
                    new_root.as_ref().to_vec(),
                )
                .await?;
                BspFile::delete_for_bsp(conn, file_key.as_ref(), OnchainBspId::from(*bsp_id))
                    .await?;
            }
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
                .await?;

                let bsp = Bsp::get_by_onchain_bsp_id(conn, OnchainBspId::from(*bsp_id)).await?;
                for (file_key, _file_metadata) in confirmed_file_keys {
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

                let size: u64 = (*size).saturated_into();
                let size: i64 = size.saturated_into();
                let who = who.as_ref().to_vec();
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
                )
                .await?;
            }
            pallet_file_system::Event::MoveBucketRequested { .. } => {}
            pallet_file_system::Event::NewCollectionAndAssociation { .. } => {}
            pallet_file_system::Event::AcceptedBspVolunteer { .. } => {}
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
                    FileStorageRequestStep::Expired,
                )
                .await?;
            }
            pallet_file_system::Event::StorageRequestRevoked { file_key } => {
                // Check if file has any provider associations
                let has_msp = File::has_msp_associations(conn, file_key.as_ref()).await?;
                let has_bsp = File::has_bsp_associations(conn, file_key.as_ref()).await?;

                if !has_msp && !has_bsp {
                    // No associations, safe to delete immediately
                    // This happens when storage request is revoked before any BSPs or MSP confirms
                    File::delete(conn, file_key.as_ref().to_vec()).await?;
                    log::debug!(
                        "Storage request revoked for file {:?} with no associations, deleted immediately",
                        file_key
                    );
                }
                // If the file has associations, the `IncompleteStorageRequest` event will handle it
            }
            pallet_file_system::Event::StorageRequestRejected { file_key, reason } => {
                // Check if the file has any BSP associations (it will not have MSP ones since the MSP did not accept it)
                let has_bsp = File::has_bsp_associations(conn, file_key.as_ref()).await?;
                if has_bsp {
                    // If the file has BSP associations, the `IncompleteStorageRequest` event will handle it
                    return Ok(());
                }
                // If the file does not have BSP associations, it's safe to delete immediately
                File::delete(conn, file_key.as_ref().to_vec()).await?;
                log::debug!(
                    "Storage request rejected for file {:?} with reason {:?}, deleted immediately",
                    file_key,
                    reason
                );
            }
            pallet_file_system::Event::MspAcceptedStorageRequest {
                file_key,
                file_metadata: _,
            } => {
                let file = File::get_by_file_key(conn, file_key.as_ref().to_vec()).await?;
                let bucket = Bucket::get_by_id(conn, file.bucket_id).await?;
                if let Some(msp_id) = bucket.msp_id {
                    MspFile::create(conn, msp_id, file.id).await?;
                }
            }
            pallet_file_system::Event::BspRequestedToStopStoring { .. } => {}
            pallet_file_system::Event::PriorityChallengeForFileDeletionQueued { .. } => {}
            pallet_file_system::Event::MspStopStoringBucketInsolventUser {
                msp_id,
                owner: _,
                bucket_id,
            } => {
                let msp = Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id)).await?;
                MspFile::delete_by_bucket(conn, bucket_id.as_ref(), msp.id).await?;
                Bucket::unset_msp(conn, bucket_id.as_ref().to_vec()).await?;
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
                BspFile::delete_for_bsp(conn, file_key, OnchainBspId::from(*sp_id)).await?;
            }
            pallet_file_system::Event::FailedToQueuePriorityChallenge { .. } => {}
            pallet_file_system::Event::FileDeletionRequest { .. } => {}
            pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest { .. } => {}
            pallet_file_system::Event::BspChallengeCycleInitialised { .. } => {}
            pallet_file_system::Event::MoveBucketRequestExpired { .. } => {}
            pallet_file_system::Event::MoveBucketRejected { .. } => {}
            pallet_file_system::Event::MspStoppedStoringBucket {
                msp_id,
                owner: _,
                bucket_id,
            } => {
                let msp = Msp::get_by_onchain_msp_id(conn, OnchainMspId::from(*msp_id)).await?;
                MspFile::delete_by_bucket(conn, bucket_id.as_ref(), msp.id).await?;
                Bucket::unset_msp(conn, bucket_id.as_ref().to_vec()).await?;
            }
            pallet_file_system::Event::BucketDeleted {
                who: _,
                bucket_id,
                maybe_collection_id: _,
            } => {
                Bucket::delete(conn, bucket_id.as_ref().to_vec()).await?;
            }
            pallet_file_system::Event::FileDeletionRequested {
                signed_delete_intention,
                signature,
            } => {
                // Mark file for deletion with user signature
                let file_key = &signed_delete_intention.file_key;
                let signature_bytes = signature.encode();
                File::update_deletion_status(
                    conn,
                    file_key.as_ref(),
                    FileDeletionStatus::InProgress,
                    Some(signature_bytes),
                )
                .await?;
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
                            .await?;
                    }
                }

                // Check if files should be deleted (no more associations)
                for file_key in file_keys.iter() {
                    let deleted = File::delete_if_orphaned(conn, file_key.as_ref()).await?;

                    if deleted {
                        log::trace!("Deleted orphaned file after MSP deletion: {:?}", file_key);
                    }
                }

                // Update bucket merkle root
                Bucket::update_merkle_root(
                    conn,
                    bucket_id.as_ref().to_vec(),
                    new_root.as_ref().to_vec(),
                )
                .await?;
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
                        .await?;
                }

                // Check if files should be deleted (no more associations)
                for file_key in file_keys.iter() {
                    let deleted = File::delete_if_orphaned(conn, file_key.as_ref()).await?;

                    if deleted {
                        log::trace!("Deleted orphaned file after BSP deletion: {:?}", file_key);
                    }
                }

                // Update BSP merkle root
                Bsp::update_merkle_root(
                    conn,
                    OnchainBspId::from(*bsp_id),
                    new_root.as_ref().to_vec(),
                )
                .await?;
            }
            // This event covers all scenarios where a storage request was unfulfilled while there were BSPs and/or the MSP who have confirmed to store the file
            // and necessitates a fisherman to delete this file.
            pallet_file_system::Event::IncompleteStorageRequest { file_key } => {
                // Check if file has any provider associations
                let has_msp = File::has_msp_associations(conn, file_key.as_ref()).await?;
                let has_bsp = File::has_bsp_associations(conn, file_key.as_ref()).await?;

                if has_msp || has_bsp {
                    // File has associations, mark for deletion by fisherman
                    File::update_deletion_status(
                        conn,
                        file_key.as_ref(),
                        FileDeletionStatus::InProgress,
                        None,
                    )
                    .await?;

                    log::debug!(
                        "Incomplete storage request for file {:?} with existing associations (MSP: {}, BSP: {}), marked for deletion without signature",
                        file_key, has_msp, has_bsp
                    );
                } else {
                    // No associations, safe to delete immediately
                    File::delete(conn, file_key.as_ref().to_vec()).await?;
                    log::debug!(
                        "Incomplete storage request for file {:?} with no associations, deleted immediately",
                        file_key
                    );
                }
            }
            pallet_file_system::Event::__Ignore(_, _) => {}
        }
        Ok(())
    }

    async fn index_payment_streams_event<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_payment_streams::Event<Runtime>,
    ) -> Result<(), diesel::result::Error> {
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
                .await?;
            }
            pallet_payment_streams::Event::DynamicRatePaymentStreamUpdated {
                provider_id,
                user_account,
                new_amount_provided,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id).await?;

                PaymentStream::update_dynamic_rate(
                    conn,
                    ps.id,
                    (*new_amount_provided).into().into(),
                )
                .await?;
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
                .await?;
            }
            pallet_payment_streams::Event::FixedRatePaymentStreamUpdated {
                provider_id,
                user_account,
                new_rate,
            } => {
                // Using .to_string() leads to truncated provider_id
                let provider_id = format!("{:#?}", provider_id);

                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id).await?;
                PaymentStream::update_fixed_rate(conn, ps.id, (*new_rate).into()).await?;
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
                let ps = PaymentStream::get(conn, user_account.to_string(), provider_id).await?;
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
        event: &pallet_proofs_dealer::Event<Runtime>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            pallet_proofs_dealer::Event::MutationsAppliedForProvider { .. } => {}
            pallet_proofs_dealer::Event::MutationsApplied { .. } => {}
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
        event: &pallet_storage_providers::Event<Runtime>,
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
                    (*capacity).into(),
                    root.as_ref().to_vec(),
                    sql_multiaddresses,
                    OnchainBspId::new(*bsp_id),
                    stake,
                )
                .await?;
            }
            pallet_storage_providers::Event::BspSignOffSuccess {
                who,
                bsp_id: _bsp_id,
            } => {
                Bsp::delete_by_account(conn, who.to_string()).await?;
            }
            pallet_storage_providers::Event::CapacityChanged {
                who,
                new_capacity,
                provider_id,
                old_capacity: _old_capacity,
                next_block_when_change_allowed: _next_block_when_change_allowed,
            } => match provider_id {
                StorageProviderId::BackupStorageProvider(bsp_id) => {
                    Bsp::update_capacity(conn, who.to_string(), (*new_capacity).into()).await?;

                    // update also the stake
                    let stake = self
                        .client
                        .runtime_api()
                        .get_bsp_stake(block_hash, bsp_id)
                        .expect("to have a stake")
                        .unwrap_or(Default::default())
                        .into();

                    Bsp::update_stake(conn, OnchainBspId::from(*bsp_id), stake).await?;
                }
                StorageProviderId::MainStorageProvider(_) => {
                    Bsp::update_capacity(conn, who.to_string(), (*new_capacity).into()).await?;
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
                    (*capacity).into(),
                    value_prop,
                    sql_multiaddresses,
                    OnchainMspId::new(*msp_id),
                )
                .await?;
            }
            pallet_storage_providers::Event::MspSignOffSuccess {
                who,
                msp_id: _msp_id,
            } => {
                Msp::delete_by_account(conn, who.to_string()).await?;
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
                let stake = self
                    .client
                    .runtime_api()
                    .get_bsp_stake(block_hash, provider_id)
                    .expect("to have a stake")
                    .unwrap_or(Default::default())
                    .into();

                Bsp::update_stake(conn, OnchainBspId::from(*provider_id), stake).await?;
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
                Msp::delete(conn, OnchainMspId::from(*provider_id)).await?;
            }
            pallet_storage_providers::Event::BspDeleted { provider_id } => {
                Bsp::delete(conn, OnchainBspId::from(*provider_id)).await?;
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
    ) -> Result<(), diesel::result::Error> {
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
