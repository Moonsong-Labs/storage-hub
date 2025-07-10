# Lite Mode Indexed Events

The indexer in lite mode only processes events relevant to the configured MSP.

## FileSystem Pallet

### Indexed Events:
- **NewBucket**: When bucket is assigned to current MSP
- **MoveBucketAccepted**: When bucket moves to/from current MSP  
- **NewStorageRequest**: When request is for bucket managed by current MSP
- **StorageRequestFulfilled**: When storage request in MSP's bucket is fulfilled
- **StorageRequestExpired**: When storage request in MSP's bucket expires
- **StorageRequestRevoked**: When storage request in MSP's bucket is revoked
- **BucketPrivacyUpdated**: When privacy settings change for MSP's bucket
- **BucketDeleted**: When MSP's bucket is deleted
- **MoveBucketRequested**: When move is requested for MSP's bucket
- **MoveBucketRejected**: When move is rejected for MSP's bucket
- **MspStoppedStoringBucket**: When current MSP stops storing a bucket
- **MspStopStoringBucketInsolventUser**: When current MSP removes insolvent user's bucket
- **FileDeletionRequest**: When deletion is requested for file in MSP's bucket
- **ProofSubmittedForPendingFileDeletionRequest**: When proof is submitted for file deletion in MSP's bucket
- **MoveBucketRequestExpired**: When move request expires for MSP's bucket
- **MspAcceptedStorageRequest**: When current MSP accepts a storage request (before BSP fulfillment)
- **StorageRequestRejected**: When current MSP rejects a request or request expires

### Ignored Events:
- All BSP-related events:
  - BspConfirmStoppedStoring
  - BspConfirmedStoring
  - AcceptedBspVolunteer
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
- **MspRequestSignUpSuccess**: When current MSP's sign-up request is successful
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

### Ignored Events:
- All BSP events:
  - BspRequestSignUpSuccess
  - BspSignUpSuccess
  - BspSignOffSuccess
  - BspDeleted
  - AwaitingTopUp
- General provider events:
  - SignUpRequestCanceled
  - ValuePropAdded (can't determine ownership)
  - ValuePropUnavailable (can't determine ownership)
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

In lite mode, the indexer uses a separate event routing mechanism that:
1. Routes events through `index_event_lite` instead of `index_event`
2. Calls specialized handlers (`index_file_system_event_lite`, `index_providers_event_lite`)
3. These handlers filter events before delegating to the original handlers
4. This ensures clean separation between Full and Lite modes without performance overhead in Full mode

The MSP ID is synchronized from the keystore on each finality notification in Lite mode, ensuring the indexer always filters based on the current MSP identity.