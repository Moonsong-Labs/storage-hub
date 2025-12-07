//! Fishing mode event handlers for the indexer service.
//!
//! This module implements the fishing mode which indexes only file creation/deletion
//! events and BSP-file associations needed for fisherman monitoring. This mode is
//! automatically selected when running a fisherman-only node to minimize database
//! load while maintaining file availability tracking.

use anyhow::Result;
use log::trace;
use shc_common::{traits::StorageEnableRuntime, types::StorageEnableEvents};
use shc_indexer_db::DbConnection;
use sp_runtime::traits::NumberFor;

use super::IndexerService;

use pallet_file_system;
use pallet_proofs_dealer;
use pallet_storage_providers;

const LOG_TARGET: &str = "indexer-service::fishing_handlers";

impl<Runtime> IndexerService<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    pub(super) async fn index_event_fishing<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: Runtime::Hash,
        block_number: NumberFor<Runtime::Block>,
        evm_tx_hash: Option<Runtime::Hash>,
    ) -> Result<(), diesel::result::Error> {
        match event {
            StorageEnableEvents::FileSystem(fs_event) => match fs_event {
                pallet_file_system::Event::NewStorageRequest { .. } => {
                    trace!(target: LOG_TARGET, "Indexing NewStorageRequest event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::StorageRequestRevoked { .. }
                | pallet_file_system::Event::SpStopStoringInsolventUser { .. } => {
                    trace!(target: LOG_TARGET, "Indexing file deletion event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::BspConfirmedStoring { .. }
                | pallet_file_system::Event::BspConfirmStoppedStoring { .. } => {
                    trace!(target: LOG_TARGET, "Indexing BSP-file association event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::NewBucket { .. }
                | pallet_file_system::Event::BucketDeleted { .. } => {
                    trace!(target: LOG_TARGET, "Indexing bucket event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::MspAcceptedStorageRequest { .. }
                | pallet_file_system::Event::StorageRequestFulfilled { .. } => {
                    trace!(target: LOG_TARGET, "Indexing MSP-file association event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::MspStopStoringBucketInsolventUser { .. }
                | pallet_file_system::Event::MspStoppedStoringBucket { .. } => {
                    trace!(target: LOG_TARGET, "Indexing MSP bucket removal event");
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::StorageRequestExpired { file_key } => {
                    trace!(target: LOG_TARGET, "Indexing expired storage request event for file key: {:?}", file_key);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::MoveBucketAccepted {
                    bucket_id,
                    old_msp_id,
                    new_msp_id,
                    value_prop_id,
                } => {
                    trace!(target: LOG_TARGET, "Indexing move bucket accepted event for bucket ID: {}, old MSP ID: {:?}, new MSP ID: {:?}, value prop ID: {:?}",
                        bucket_id, old_msp_id, new_msp_id, value_prop_id);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::BucketFileDeletionsCompleted {
                    user,
                    file_keys,
                    bucket_id,
                    msp_id,
                    old_root,
                    new_root,
                } => {
                    trace!(target: LOG_TARGET, "Indexing MSP file deletion completed event for user: {:?}, file keys: {:?}, bucket ID: {:?}, MSP ID: {:?}, old root: {:?}, new root: {:?}",
                        user, file_keys, bucket_id, msp_id, old_root, new_root);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::BspFileDeletionsCompleted {
                    users,
                    file_keys,
                    bsp_id,
                    old_root,
                    new_root,
                } => {
                    trace!(target: LOG_TARGET, "Indexing BSP file deletion completed event for users: {:?}, file keys: {:?}, BSP ID: {:?}, old root: {:?}, new root: {:?}",
                        users, file_keys, bsp_id, old_root, new_root);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::FileDeletionRequested {
                    signed_delete_intention,
                    signature: _,
                } => {
                    trace!(target: LOG_TARGET, "Indexing file deletion requested event for file key: {:?}",
                        signed_delete_intention.file_key);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::IncompleteStorageRequest { file_key } => {
                    trace!(target: LOG_TARGET, "Indexing incomplete storage request event for file key: {:?}", file_key);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::IncompleteStorageRequestCleanedUp { file_key } => {
                    trace!(target: LOG_TARGET, "Indexing incomplete storage request cleaned up event for file key: {:?}", file_key);
                    self.index_file_system_event(
                        conn,
                        fs_event,
                        block_hash,
                        block_number,
                        evm_tx_hash,
                    )
                    .await?
                }
                pallet_file_system::Event::BucketPrivacyUpdated { .. }
                | pallet_file_system::Event::MoveBucketRequested { .. }
                | pallet_file_system::Event::NewCollectionAndAssociation { .. }
                | pallet_file_system::Event::AcceptedBspVolunteer { .. }
                | pallet_file_system::Event::StorageRequestRejected { .. }
                | pallet_file_system::Event::BspRequestedToStopStoring { .. }
                | pallet_file_system::Event::PriorityChallengeForFileDeletionQueued { .. }
                | pallet_file_system::Event::FailedToQueuePriorityChallenge { .. }
                | pallet_file_system::Event::FileDeletionRequest { .. }
                | pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest {
                    ..
                }
                | pallet_file_system::Event::BspChallengeCycleInitialised { .. }
                | pallet_file_system::Event::MoveBucketRequestExpired { .. }
                | pallet_file_system::Event::MoveBucketRejected { .. }
                | pallet_file_system::Event::FailedToGetMspOfBucket { .. }
                | pallet_file_system::Event::FailedToDecreaseMspUsedCapacity { .. }
                | pallet_file_system::Event::UsedCapacityShouldBeZero { .. }
                | pallet_file_system::Event::FailedToReleaseStorageRequestCreationDeposit {
                    ..
                }
                | pallet_file_system::Event::FailedToTransferDepositFundsToBsp { .. }
                | pallet_file_system::Event::__Ignore(_, _) => {
                    trace!(target: LOG_TARGET, "Ignoring non-essential FileSystem event in fishing mode");
                }
            },
            StorageEnableEvents::StorageProviders(provider_event) => match provider_event {
                pallet_storage_providers::Event::BspSignUpSuccess { .. }
                | pallet_storage_providers::Event::BspSignOffSuccess { .. }
                | pallet_storage_providers::Event::BspDeleted { .. } => {
                    trace!(target: LOG_TARGET, "Indexing BSP provider event");
                    self.index_providers_event(conn, provider_event, block_hash)
                        .await?
                }
                pallet_storage_providers::Event::MspSignUpSuccess { .. }
                | pallet_storage_providers::Event::MspSignOffSuccess { .. }
                | pallet_storage_providers::Event::MspDeleted { .. } => {
                    trace!(target: LOG_TARGET, "Indexing MSP provider event");
                    self.index_providers_event(conn, provider_event, block_hash)
                        .await?
                }
                pallet_storage_providers::Event::BspRequestSignUpSuccess { .. }
                | pallet_storage_providers::Event::MspRequestSignUpSuccess { .. }
                | pallet_storage_providers::Event::SignUpRequestCanceled { .. }
                | pallet_storage_providers::Event::CapacityChanged { .. }
                | pallet_storage_providers::Event::BucketRootChanged { .. }
                | pallet_storage_providers::Event::Slashed { .. }
                | pallet_storage_providers::Event::AwaitingTopUp { .. }
                | pallet_storage_providers::Event::TopUpFulfilled { .. }
                | pallet_storage_providers::Event::ValuePropAdded { .. }
                | pallet_storage_providers::Event::ValuePropUnavailable { .. }
                | pallet_storage_providers::Event::MultiAddressAdded { .. }
                | pallet_storage_providers::Event::MultiAddressRemoved { .. }
                | pallet_storage_providers::Event::ProviderInsolvent { .. }
                | pallet_storage_providers::Event::BucketsOfInsolventMsp { .. }
                | pallet_storage_providers::Event::FailedToGetOwnerAccountOfInsolventProvider {
                    ..
                }
                | pallet_storage_providers::Event::FailedToSlashInsolventProvider { .. }
                | pallet_storage_providers::Event::FailedToStopAllCyclesForInsolventBsp {
                    ..
                }
                | pallet_storage_providers::Event::FailedToInsertProviderTopUpExpiration {
                    ..
                }
                | pallet_storage_providers::Event::__Ignore(_, _) => {
                    trace!(target: LOG_TARGET, "Ignoring non-essential provider event in fishing mode");
                }
            },
            StorageEnableEvents::ProofsDealer(proofs_dealer_event) => match proofs_dealer_event {
                pallet_proofs_dealer::Event::MutationsApplied { .. } => {
                    trace!(target: LOG_TARGET, "Indexing MutationsApplied event");
                    self.index_proofs_dealer_event(conn, proofs_dealer_event, block_hash)
                        .await?
                }
                pallet_proofs_dealer::Event::MutationsAppliedForProvider { .. }
                | pallet_proofs_dealer::Event::NewChallenge { .. }
                | pallet_proofs_dealer::Event::NewPriorityChallenge { .. }
                | pallet_proofs_dealer::Event::ProofAccepted { .. }
                | pallet_proofs_dealer::Event::NewChallengeSeed { .. }
                | pallet_proofs_dealer::Event::NewCheckpointChallenge { .. }
                | pallet_proofs_dealer::Event::SlashableProvider { .. }
                | pallet_proofs_dealer::Event::NoRecordOfLastSubmittedProof { .. }
                | pallet_proofs_dealer::Event::NewChallengeCycleInitialised { .. }
                | pallet_proofs_dealer::Event::ChallengesTickerSet { .. }
                | pallet_proofs_dealer::Event::__Ignore(_, _) => {
                    trace!(target: LOG_TARGET, "Ignoring non-essential ProofsDealer event in fishing mode");
                }
            },
            // Explicitly list all other runtime events to ensure compilation errors on new events
            StorageEnableEvents::BucketNfts(_) => {}
            StorageEnableEvents::PaymentStreams(_) => {}
            StorageEnableEvents::Randomness(_) => {}
            StorageEnableEvents::System(_) => {}
            StorageEnableEvents::Balances(_) => {}
            StorageEnableEvents::TransactionPayment(_) => {}
            StorageEnableEvents::Other(_) => {}
        }
        Ok(())
    }
}
