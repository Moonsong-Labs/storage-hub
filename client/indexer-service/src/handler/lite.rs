//! Lite mode event handlers for the indexer service.
//!
//! This module contains all lite mode event handlers that process events
//! without MSP-specific filtering. All events are indexed in lite mode
//! for future filtering logic implementation.

use anyhow::Result;
use log::trace;
use shc_common::{traits::StorageEnableRuntime, types::StorageEnableEvents};
use shc_indexer_db::DbConnection;

use super::IndexerService;

use pallet_bucket_nfts;
use pallet_file_system;
use pallet_payment_streams;
use pallet_proofs_dealer;
use pallet_randomness;
use pallet_storage_providers;

const LOG_TARGET: &str = "indexer-service::lite_handlers";

impl<Runtime: StorageEnableRuntime> IndexerService<Runtime> {
    pub(super) async fn index_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &StorageEnableEvents<Runtime>,
        block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        match event {
            StorageEnableEvents::FileSystem(event) => {
                self.index_file_system_event_lite(conn, event).await?
            }
            StorageEnableEvents::StorageProviders(event) => {
                self.index_providers_event_lite(conn, event, block_hash)
                    .await?
            }
            StorageEnableEvents::BucketNfts(event) => {
                self.index_bucket_nfts_event_lite(conn, event, block_hash)
                    .await?
            }
            StorageEnableEvents::PaymentStreams(event) => {
                self.index_payment_streams_event_lite(conn, event, block_hash)
                    .await?
            }
            StorageEnableEvents::ProofsDealer(event) => {
                self.index_proofs_dealer_event_lite(conn, event, block_hash)
                    .await?
            }
            StorageEnableEvents::Randomness(event) => {
                self.index_randomness_event_lite(conn, event, block_hash)
                    .await?
            }
            // System pallets - explicitly list all to ensure compilation errors on new events
            StorageEnableEvents::System(_) => {}
            StorageEnableEvents::Balances(_) => {}
            StorageEnableEvents::TransactionPayment(_) => {}
            StorageEnableEvents::Other(_) => {}
        }

        Ok(())
    }

    pub(crate) async fn index_file_system_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_file_system::Event<Runtime>,
    ) -> Result<(), diesel::result::Error> {
        // In lite mode without MSP filtering, index all events
        let should_index = match event {
            pallet_file_system::Event::NewBucket { .. } => true,
            pallet_file_system::Event::MoveBucketAccepted { .. } => true,
            pallet_file_system::Event::NewStorageRequest { .. } => true,
            pallet_file_system::Event::StorageRequestFulfilled { .. } => true,
            pallet_file_system::Event::StorageRequestExpired { .. } => true,
            pallet_file_system::Event::StorageRequestRevoked { .. } => true,
            pallet_file_system::Event::BucketPrivacyUpdated { .. } => true,
            pallet_file_system::Event::BucketDeleted { .. } => true,
            pallet_file_system::Event::MoveBucketRequested { .. } => true,
            pallet_file_system::Event::MoveBucketRejected { .. } => true,
            pallet_file_system::Event::MspStoppedStoringBucket { .. } => true,
            pallet_file_system::Event::MspStopStoringBucketInsolventUser { .. } => true,
            pallet_file_system::Event::FileDeletionRequest { .. } => true,
            pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest { .. } => true,
            pallet_file_system::Event::MoveBucketRequestExpired { .. } => true,
            pallet_file_system::Event::MspAcceptedStorageRequest { .. } => true,
            pallet_file_system::Event::StorageRequestRejected { .. } => true,
            pallet_file_system::Event::NewCollectionAndAssociation { .. } => true,
            pallet_file_system::Event::AcceptedBspVolunteer { .. } => true,
            pallet_file_system::Event::BspRequestedToStopStoring { .. } => true,
            pallet_file_system::Event::BspConfirmStoppedStoring { .. } => true,
            pallet_file_system::Event::BspConfirmedStoring { .. } => true,
            pallet_file_system::Event::PriorityChallengeForFileDeletionQueued { .. } => true,
            pallet_file_system::Event::SpStopStoringInsolventUser { .. } => true,
            pallet_file_system::Event::FailedToQueuePriorityChallenge { .. } => true,
            pallet_file_system::Event::BspChallengeCycleInitialised { .. } => true,
            pallet_file_system::Event::FailedToGetMspOfBucket { .. } => true,
            pallet_file_system::Event::FailedToDecreaseMspUsedCapacity { .. } => true,
            pallet_file_system::Event::UsedCapacityShouldBeZero { .. } => true,
            pallet_file_system::Event::FailedToReleaseStorageRequestCreationDeposit { .. } => true,
            pallet_file_system::Event::FailedToTransferDepositFundsToBsp { .. } => true,
            pallet_file_system::Event::FileDeletionRequested { .. } => true,
            pallet_file_system::Event::MspFileDeletionCompleted { .. } => true,
            pallet_file_system::Event::BspFileDeletionCompleted { .. } => true,
            pallet_file_system::Event::FileDeletedFromIncompleteStorageRequest { .. } => true,
            pallet_file_system::Event::IncompleteStorageRequest { .. } => true,
            pallet_file_system::Event::__Ignore(_, _) => true,
        };

        if should_index {
            // Delegate to the original method
            self.index_file_system_event(conn, event).await
        } else {
            trace!(target: LOG_TARGET, "Filtered out FileSystem event in lite mode");
            Ok(())
        }
    }

    pub(crate) async fn index_providers_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_storage_providers::Event<Runtime>,
        block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        // In lite mode without MSP filtering, index all provider events
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            pallet_storage_providers::Event::MspRequestSignUpSuccess { .. }
            | pallet_storage_providers::Event::MspSignUpSuccess { .. }
            | pallet_storage_providers::Event::MspSignOffSuccess { .. }
            | pallet_storage_providers::Event::BspRequestSignUpSuccess { .. }
            | pallet_storage_providers::Event::BspSignUpSuccess { .. }
            | pallet_storage_providers::Event::BspSignOffSuccess { .. }
            | pallet_storage_providers::Event::SignUpRequestCanceled { .. }
            | pallet_storage_providers::Event::CapacityChanged { .. }
            | pallet_storage_providers::Event::Slashed { .. }
            | pallet_storage_providers::Event::AwaitingTopUp { .. }
            | pallet_storage_providers::Event::TopUpFulfilled { .. }
            | pallet_storage_providers::Event::MspDeleted { .. }
            | pallet_storage_providers::Event::BspDeleted { .. }
            | pallet_storage_providers::Event::ProviderInsolvent { .. }
            | pallet_storage_providers::Event::BucketsOfInsolventMsp { .. }
            | pallet_storage_providers::Event::MultiAddressAdded { .. }
            | pallet_storage_providers::Event::MultiAddressRemoved { .. }
            | pallet_storage_providers::Event::BucketRootChanged { .. }
            | pallet_storage_providers::Event::ValuePropAdded { .. }
            | pallet_storage_providers::Event::ValuePropUnavailable { .. }
            | pallet_storage_providers::Event::FailedToGetOwnerAccountOfInsolventProvider {
                ..
            }
            | pallet_storage_providers::Event::FailedToSlashInsolventProvider { .. }
            | pallet_storage_providers::Event::FailedToStopAllCyclesForInsolventBsp { .. }
            | pallet_storage_providers::Event::FailedToInsertProviderTopUpExpiration { .. }
            | pallet_storage_providers::Event::__Ignore { .. } => true,
        };

        if should_index {
            // Delegate to the original method
            self.index_providers_event(conn, event, block_hash).await
        } else {
            trace!(target: LOG_TARGET, "Filtered out Providers event in lite mode");
            Ok(())
        }
    }

    pub(crate) async fn index_bucket_nfts_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_bucket_nfts::Event<Runtime>,
        _block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            pallet_bucket_nfts::Event::AccessShared { .. }
            | pallet_bucket_nfts::Event::ItemReadAccessUpdated { .. }
            | pallet_bucket_nfts::Event::ItemBurned { .. }
            | pallet_bucket_nfts::Event::__Ignore { .. } => true,
        };

        if should_index {
            self.index_bucket_nfts_event(conn, event).await?;
        }

        Ok(())
    }

    pub(crate) async fn index_payment_streams_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_payment_streams::Event<Runtime>,
        _block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            pallet_payment_streams::Event::FixedRatePaymentStreamCreated { .. }
            | pallet_payment_streams::Event::FixedRatePaymentStreamUpdated { .. }
            | pallet_payment_streams::Event::FixedRatePaymentStreamDeleted { .. }
            | pallet_payment_streams::Event::DynamicRatePaymentStreamCreated { .. }
            | pallet_payment_streams::Event::DynamicRatePaymentStreamUpdated { .. }
            | pallet_payment_streams::Event::DynamicRatePaymentStreamDeleted { .. }
            | pallet_payment_streams::Event::PaymentStreamCharged { .. }
            | pallet_payment_streams::Event::UsersCharged { .. }
            | pallet_payment_streams::Event::LastChargeableInfoUpdated { .. }
            | pallet_payment_streams::Event::UserWithoutFunds { .. }
            | pallet_payment_streams::Event::UserPaidAllDebts { .. }
            | pallet_payment_streams::Event::UserPaidSomeDebts { .. }
            | pallet_payment_streams::Event::UserSolvent { .. }
            | pallet_payment_streams::Event::InconsistentTickProcessing { .. }
            | pallet_payment_streams::Event::__Ignore { .. } => true,
        };

        if should_index {
            self.index_payment_streams_event(conn, event).await?;
        }

        Ok(())
    }

    pub(crate) async fn index_proofs_dealer_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_proofs_dealer::Event<Runtime>,
        _block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            pallet_proofs_dealer::Event::NewChallenge { .. }
            | pallet_proofs_dealer::Event::ProofAccepted { .. }
            | pallet_proofs_dealer::Event::NewChallengeSeed { .. }
            | pallet_proofs_dealer::Event::NewCheckpointChallenge { .. }
            | pallet_proofs_dealer::Event::SlashableProvider { .. }
            | pallet_proofs_dealer::Event::NoRecordOfLastSubmittedProof { .. }
            | pallet_proofs_dealer::Event::NewChallengeCycleInitialised { .. }
            | pallet_proofs_dealer::Event::MutationsAppliedForProvider { .. }
            | pallet_proofs_dealer::Event::MutationsApplied { .. }
            | pallet_proofs_dealer::Event::ChallengesTickerSet { .. }
            | pallet_proofs_dealer::Event::NewPriorityChallenge { .. }
            | pallet_proofs_dealer::Event::__Ignore { .. } => true,
        };

        if should_index {
            self.index_proofs_dealer_event(conn, event).await?;
        }

        Ok(())
    }

    pub(crate) async fn index_randomness_event_lite<'a, 'b: 'a>(
        &'b self,
        conn: &mut DbConnection<'a>,
        event: &pallet_randomness::Event<Runtime>,
        _block_hash: Runtime::Hash,
    ) -> Result<(), diesel::result::Error> {
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            pallet_randomness::Event::NewOneEpochAgoRandomnessAvailable { .. }
            | pallet_randomness::Event::__Ignore { .. } => true,
        };

        if should_index {
            self.index_randomness_event(conn, event).await?;
        }

        Ok(())
    }
}
