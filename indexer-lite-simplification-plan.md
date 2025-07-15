## Implementation Plan: Simplify Indexer Lite Mode

### Overview

Simplify the indexer lite mode by removing MSP ID synchronization and filtering logic while preserving the event routing architecture for future implementation. The lite mode will temporarily process all events until new filtering logic is added later.

### Prerequisites

- [ ] Access to `/client/indexer-service/src/` directory
- [ ] Understanding of existing event routing structure
- [ ] Cargo and test infrastructure available

### Steps

1. **Remove MSP ID Synchronization from Handler**

   - File: `/client/indexer-service/src/handler.rs`
   - Operation: Remove MSP ID detection and passing
   - Details:
     - Remove lines 207-210 (MSP ID detection in finality notification handler)
     - Modify line 263: Change `self.route_event(event, storage, msp_id, block_hash).await?;` to `self.route_event(event, storage, block_hash).await?;`
     - Update `route_event` function signature (line 256): Remove `msp_id: Option<ProviderId>` parameter
     - Update line 269: Change `self.index_event_lite(event, storage, msp_id, block_hash).await` to `self.index_event_lite(event, storage, block_hash).await`
     - Keep `detect_msp_id` function (lines 93-104) as requested
   - Success: Code compiles without MSP ID being passed through event routing

2. **Simplify Lite Mode Event Handlers to Process All Events**

   - File: `/client/indexer-service/src/handler.rs`
   - Operation: Remove MSP filtering logic from lite event handlers
   - Details:
     
     a) Update `index_event_lite` function (line 271):
        - Remove `msp_id: Option<ProviderId>` parameter
        - Remove lines 291-295 (MSP ID sync logic)
        - Update all sub-handler calls to remove `msp_id` parameter

     b) Simplify `index_file_system_event_lite` (line 568):
        - Remove `msp_id: Option<ProviderId>` parameter
        - Update the match statement to always return `true` for all events (list all variants explicitly):
        ```rust
        let should_index = match event {
            // All events return true for now - ready for future filtering logic
            FileSystemEvent::NewBucket { .. }
            | FileSystemEvent::MoveBucketAccepted { .. }
            | FileSystemEvent::NewStorageRequest { .. }
            | FileSystemEvent::StorageRequestFulfilled { .. }
            | FileSystemEvent::StorageRequestExpired { .. }
            | FileSystemEvent::StorageRequestRevoked { .. }
            | FileSystemEvent::BucketPrivacyUpdated { .. }
            | FileSystemEvent::BucketDeleted { .. }
            | FileSystemEvent::MoveBucketRequested { .. }
            | FileSystemEvent::MoveBucketRejected { .. }
            | FileSystemEvent::MspStoppedStoringBucket { .. }
            | FileSystemEvent::MspStopStoringBucketInsolventUser { .. }
            | FileSystemEvent::FileDeletionRequest { .. }
            | FileSystemEvent::ProofSubmittedForPendingFileDeletionRequest { .. }
            | FileSystemEvent::MoveBucketRequestExpired { .. }
            | FileSystemEvent::MspAcceptedStorageRequest { .. }
            | FileSystemEvent::StorageRequestRejected { .. }
            | FileSystemEvent::AcceptedBspVolunteer { .. }
            | FileSystemEvent::BspConfirmedStoring { .. }
            | FileSystemEvent::BspStoppedStoring { .. }
            | FileSystemEvent::ItemAdded { .. }
            | FileSystemEvent::PrivateItemAdded { .. }
            | FileSystemEvent::ItemDeleted { .. }
            | FileSystemEvent::SpIncreaseCapacity { .. }
            | FileSystemEvent::SpDecreaseCapacity { .. }
            | FileSystemEvent::FailedToQueuePriorityChallenge { .. }
            | FileSystemEvent::ChallengeCycleInitialised { .. }
            | FileSystemEvent::OpenCollectionModified { .. }
            | FileSystemEvent::BspRequestVolunteer { .. }
            | FileSystemEvent::PriorityChallenge { .. }
            | FileSystemEvent::__Ignore { .. } => true,
        };
        ```

     c) Simplify `index_providers_event_lite` (line 815):
        - Remove `msp_id: Option<ProviderId>` parameter
        - Remove lines 823-829 (MSP ID sync after MspSignUpSuccess)
        - Update the match statement with all provider event variants explicitly

   - Success: All events are processed in lite mode without filtering

3. **Create Missing Lite Mode Event Handlers with Match Structure**

   - File: `/client/indexer-service/src/handler.rs`
   - Operation: Add lite mode handlers for missing pallets
   - Details:
     
     a) Add after line 950:
     ```rust
     async fn index_bucket_nfts_event_lite(
         &self,
         event: &BucketNftsEvent,
         storage: &Storage,
         block_hash: &Hash,
     ) -> Result<(), anyhow::Error> {
         let should_index = match event {
             // All events return true for now - ready for future filtering logic
             BucketNftsEvent::AccessShared { .. }
             | BucketNftsEvent::ItemReadAccessUpdated { .. }
             | BucketNftsEvent::ItemBurned { .. }
             | BucketNftsEvent::__Ignore { .. } => true,
         };

         if should_index {
             self.index_bucket_nfts_event(event, storage, block_hash).await?;
         }

         Ok(())
     }

     async fn index_payment_streams_event_lite(
         &self,
         event: &PaymentStreamsEvent,
         storage: &Storage,
         block_hash: &Hash,
     ) -> Result<(), anyhow::Error> {
         let should_index = match event {
             // All events return true for now - ready for future filtering logic
             PaymentStreamsEvent::PaymentStreamCreated { .. }
             | PaymentStreamsEvent::PaymentStreamUpdated { .. }
             | PaymentStreamsEvent::PaymentStreamCharged { .. }
             | PaymentStreamsEvent::PaymentStreamClosed { .. }
             | PaymentStreamsEvent::DynamicRatePaymentStreamUpdated { .. }
             | PaymentStreamsEvent::DynamicRatePaymentStreamDeleted { .. }
             | PaymentStreamsEvent::ProviderChargeableInfoUpdated { .. }
             | PaymentStreamsEvent::UserChargeableInfoUpdated { .. }
             | PaymentStreamsEvent::LastChargeableInfoUpdated { .. }
             | PaymentStreamsEvent::LastChargeableInfoRemoved { .. }
             | PaymentStreamsEvent::ProviderInsolvent { .. }
             | PaymentStreamsEvent::UserPaidDebts { .. }
             | PaymentStreamsEvent::UserSolvent { .. }
             | PaymentStreamsEvent::ChargeError { .. }
             | PaymentStreamsEvent::__Ignore { .. } => true,
         };

         if should_index {
             self.index_payment_streams_event(event, storage, block_hash).await?;
         }

         Ok(())
     }

     async fn index_proofs_dealer_event_lite(
         &self,
         event: &ProofsDealerEvent,
         storage: &Storage,
         block_hash: &Hash,
     ) -> Result<(), anyhow::Error> {
         let should_index = match event {
             // All events return true for now - ready for future filtering logic
             ProofsDealerEvent::ChallengeInitialised { .. }
             | ProofsDealerEvent::ProofAccepted { .. }
             | ProofsDealerEvent::NewChallengeSeed { .. }
             | ProofsDealerEvent::MutationsApplied { .. }
             | ProofsDealerEvent::NewChallengeCycleInitialised { .. }
             | ProofsDealerEvent::SlashableProvider { .. }
             | ProofsDealerEvent::ChallengesTickResult { .. }
             | ProofsDealerEvent::ChallengesFailed { .. }
             | ProofsDealerEvent::CheckpointChallengesFailed { .. }
             | ProofsDealerEvent::ChallengePrioritiesSet { .. }
             | ProofsDealerEvent::__Ignore { .. } => true,
         };

         if should_index {
             self.index_proofs_dealer_event(event, storage, block_hash).await?;
         }

         Ok(())
     }

     async fn index_randomness_event_lite(
         &self,
         event: &RandomnessEvent,
         storage: &Storage,
         block_hash: &Hash,
     ) -> Result<(), anyhow::Error> {
         let should_index = match event {
             // All events return true for now - ready for future filtering logic
             RandomnessEvent::NewOneEpochAgoRandomnessAvailable { .. }
             | RandomnessEvent::__Ignore { .. } => true,
         };

         if should_index {
             self.index_randomness_event(event, storage, block_hash).await?;
         }

         Ok(())
     }
     ```

     b) Update `index_event_lite` to call new handlers (around line 280):
     ```rust
     RuntimeEvent::BucketNfts(event) => {
         self.index_bucket_nfts_event_lite(event, storage, block_hash).await?;
     }
     RuntimeEvent::PaymentStreams(event) => {
         self.index_payment_streams_event_lite(event, storage, block_hash).await?;
     }
     RuntimeEvent::ProofsDealer(event) => {
         self.index_proofs_dealer_event_lite(event, storage, block_hash).await?;
     }
     RuntimeEvent::Randomness(event) => {
         self.index_randomness_event_lite(event, storage, block_hash).await?;
     }
     ```

   - Success: All pallets have lite mode handlers with exhaustive match patterns

4. **Remove MSP Ownership Check Functions**

   - File: `/client/indexer-service/src/handler.rs`
   - Operation: Remove helper functions no longer needed
   - Details:
     - Remove `check_bucket_belongs_to_current_msp` function (lines 952-970)
     - Remove `check_file_belongs_to_current_msp` function (lines 972-990)
   - Success: Unused code removed

5. **Remove All Lite Mode Tests**

   - Files: Multiple test files in `/test/suites/integration/msp/`
   - Operation: Delete all lite mode test files
   - Details:
     - Delete `/test/suites/integration/msp/indexer-lite-core.test.ts`
     - Delete `/test/suites/integration/msp/indexer-lite-mode.test.ts`
     - Delete `/test/suites/integration/msp/indexer-lite-msp-registration.test.ts`
     - Search for and remove any other test files containing "lite" in their name or testing lite mode functionality
   - Success: No lite mode tests remain

6. **Refactor Event Handlers into Separate Module**

   - File: Create `/client/indexer-service/src/lite_handlers.rs`
   - Operation: Move all lite mode handlers to new module
   - Details:
     
     a) Create new file `/client/indexer-service/src/lite_handlers.rs`:
     ```rust
     use super::*;
     use crate::handler::Handler;
     use anyhow::Result;
     use sc_client_db::hash_to_hex;
     use sh_rust_storage_client::StorageClient as Storage;
     use storage_hub_runtime::{Hash, RuntimeEvent};
     
     impl Handler {
         pub(crate) async fn index_event_lite(
             &self,
             event: &RuntimeEvent,
             storage: &Storage,
             block_hash: &Hash,
         ) -> Result<()> {
             // Move index_event_lite implementation here with exhaustive match
         }
         
         pub(crate) async fn index_file_system_event_lite(
             &self,
             event: &FileSystemEvent,
             storage: &Storage,
             block_hash: &Hash,
         ) -> Result<()> {
             // Move implementation here with exhaustive match
         }
         
         // Move all other *_event_lite functions here
     }
     ```

     b) In `/client/indexer-service/src/handler.rs`:
     - Add `mod lite_handlers;` after other module declarations
     - Remove all `*_event_lite` function implementations (keep only the main handlers)
     - Keep `route_event` function that calls into lite handlers

     c) In `/client/indexer-service/src/lib.rs`:
     - No changes needed as lite_handlers is internal to handler module

   - Success: handler.rs file size significantly reduced, lite mode logic isolated

### Testing Strategy

- [ ] Run `cargo build --release` to verify compilation
- [ ] Run `cargo clippy` to check for warnings
- [ ] Run `cargo fmt` to ensure formatting
- [ ] Manually test indexer in lite mode to verify it processes all events
- [ ] Verify database contains all event data (no filtering)

### Rollback Plan

This is a significant simplification that removes functionality. To rollback:
1. Restore MSP ID detection and passing in handler.rs
2. Restore filtering logic in lite event handlers
3. Restore ownership check functions
4. Restore lite mode tests
5. Remove newly added lite handlers for other pallets
6. Move lite handlers back from separate module to handler.rs