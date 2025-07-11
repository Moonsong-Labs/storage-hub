# Lite Mode Indexed Events

> **Note**: This document describes the **Enhanced Lite Mode** behavior, which extends the original lite mode to support complete bucket transfers and full network visibility.

The indexer in enhanced lite mode processes all buckets and files while maintaining MSP-specific event filtering for provider operations.

## FileSystem Pallet

### Indexed Events:
- **NewBucket**: ALL buckets are indexed (not just MSP-owned)
- **MoveBucketAccepted**: ALL bucket moves are tracked for complete bucket transfer support
- **NewStorageRequest**: ALL storage requests are indexed
- **StorageRequestFulfilled**: ALL fulfillments are tracked
- **StorageRequestExpired**: ALL expirations are tracked
- **StorageRequestRevoked**: ALL revocations are tracked
- **BucketPrivacyUpdated**: ALL privacy updates are tracked
- **BucketDeleted**: ALL bucket deletions are tracked
- **MoveBucketRequested**: ALL move requests are tracked
- **MoveBucketRejected**: ALL move rejections are tracked
- **MspStoppedStoringBucket**: When current MSP stops storing a bucket
- **MspStopStoringBucketInsolventUser**: When current MSP removes insolvent user's bucket
- **FileDeletionRequest**: ALL file deletion requests are tracked
- **ProofSubmittedForPendingFileDeletionRequest**: ALL proof submissions are tracked
- **MoveBucketRequestExpired**: ALL move request expirations are tracked
- **MspAcceptedStorageRequest**: When current MSP accepts a storage request
- **StorageRequestRejected**: When current MSP rejects a request or request expires
- **AcceptedBspVolunteer**: BSP volunteer acceptances are tracked
- **BspConfirmedStoring**: BSP confirmations are tracked

### Ignored Events:
- BSP-specific operational events:
  - BspConfirmStoppedStoring
  - BspRequestedToStopStoring
  - SpStopStoringInsolventUser
  - BspChallengeCycleInitialised
- NFT and collection events:
  - NewCollectionAndAssociation
- Challenge queue events:
  - PriorityChallengeForFileDeletionQueued
  - FailedToQueuePriorityChallenge
- Error events:
  - FailedToGetMspOfBucket
  - FailedToDecreaseMspUsedCapacity
  - UsedCapacityShouldBeZero
  - FailedToReleaseStorageRequestCreationDeposit
  - FailedToTransferDepositFundsToBsp

## Providers Pallet

### Indexed Events:
- **MspSignUpSuccess**: When current MSP signs up
- **MspSignOffSuccess**: When current MSP signs off
- **CapacityChanged**: When current MSP's capacity changes
- **MultiAddressAdded**: When current MSP adds multiaddress
- **MultiAddressRemoved**: When current MSP removes multiaddress
- **MspDeleted**: When current MSP is deleted
- **ProviderInsolvent**: When current MSP becomes insolvent
- **BucketsOfInsolventMsp**: When current MSP's buckets are listed as insolvent
- **Slashed**: When current MSP is slashed
- **TopUpFulfilled**: When current MSP's top-up is fulfilled
- **BucketRootChanged**: When root changes for buckets owned by current MSP
- **ValuePropAdded**: When current MSP adds a value proposition
- **ValuePropUnavailable**: When current MSP's value proposition becomes unavailable

### Ignored Events:
- All BSP events:
  - BspRequestSignUpSuccess
  - BspSignUpSuccess
  - BspSignOffSuccess
  - BspDeleted
  - AwaitingTopUp
- General provider events:
  - SignUpRequestCanceled
  - MspRequestSignUpSuccess (only contains AccountId, not MSP ID)
- Error events:
  - FailedToGetOwnerAccountOfInsolventProvider
  - FailedToSlashInsolventProvider
  - FailedToStopAllCyclesForInsolventBsp
  - FailedToInsertProviderTopUpExpiration

## Completely Ignored Pallets
- **BucketNfts**: All events ignored (access control not relevant to MSP operations)
- **PaymentStreams**: All events ignored (payment tracking handled separately)
- **ProofsDealer**: All events ignored (BSP-specific proof system)
- **Randomness**: All events ignored (not needed for MSP operations)

## Implementation Details

### Enhanced Lite Mode Behavior

In enhanced lite mode, the indexer provides expanded functionality:

1. **Complete Data Indexing**: ALL buckets and files are indexed, not just those owned by the current MSP
   - Enables full visibility of the storage network state
   - Supports bucket transfers between any MSPs
   - Tracks BSP assignments for all files

2. **Selective Event Processing**: Events are still routed through lite-specific handlers
   - Routes events through `index_event_lite` instead of `index_event`
   - Calls specialized handlers (`index_file_system_event_lite`, `index_providers_event_lite`)
   - MSP-specific events (capacity changes, sign-ups) remain filtered to current MSP only

3. **Bucket Transfer Support**: Full support for bucket transfers requires:
   - Tracking all buckets regardless of owner
   - Monitoring BSP assignments for proper file availability
   - Maintaining complete file metadata for transfer validation

4. **MSP Identity Synchronization**: The MSP ID is synchronized from the keystore on each finality notification, ensuring MSP-specific operations always use the current identity.

### Key Differences from Original Lite Mode

- **Buckets**: Previously only MSP-owned buckets were indexed; now ALL buckets are indexed
- **Files**: Previously only files in MSP-owned buckets were tracked; now ALL files are tracked
- **BSP Events**: Key BSP events (AcceptedBspVolunteer, BspConfirmedStoring) are now indexed
- **Transfers**: Complete bucket transfer support between any MSPs, not just to/from current MSP