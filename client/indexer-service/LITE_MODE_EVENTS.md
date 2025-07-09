# Lite Mode Indexed Events

The indexer in lite mode only processes events relevant to the configured MSP.

## FileSystem Pallet

### Indexed Events:
- **NewBucket**: When bucket is assigned to current MSP
- **MoveBucketAccepted**: When bucket moves to/from current MSP  
- **NewStorageRequest**: When request is for bucket managed by current MSP

### Ignored Events:
- All BSP-related events:
  - BspConfirmStoppedStoring
  - BspConfirmedStoring
  - AcceptedBspVolunteer
  - BspRequestedToStopStoring
  - SpStopStoringInsolventUser
  - BspChallengeCycleInitialised
- General bucket operations:
  - BucketPrivacyUpdated
  - BucketDeleted
  - MoveBucketRequested
  - MoveBucketRequestExpired
  - MoveBucketRejected
  - NewCollectionAndAssociation
- File lifecycle events:
  - StorageRequestFulfilled
  - StorageRequestExpired
  - StorageRequestRevoked
  - MspAcceptedStorageRequest
  - StorageRequestRejected
  - FileDeletionRequest
  - ProofSubmittedForPendingFileDeletionRequest
- Provider stopping events:
  - MspStopStoringBucketInsolventUser
  - MspStoppedStoringBucket
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

### Ignored Events:
- All BSP events:
  - BspRequestSignUpSuccess
  - BspSignUpSuccess
  - BspSignOffSuccess
  - AwaitingTopUp
- General provider events:
  - SignUpRequestCanceled
  - MspRequestSignUpSuccess
  - MspDeleted
  - BspDeleted
  - BucketRootChanged
  - Slashed
  - TopUpFulfilled
  - ValuePropAdded
  - ValuePropUnavailable
  - ProviderInsolvent
  - BucketsOfInsolventMsp
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