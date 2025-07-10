# Lite Mode Indexed Events

The indexer in lite mode only processes events relevant to the configured MSP.

## ⚠️ IMPORTANT: Incomplete Filtering

The current lite mode filtering implementation is incomplete and requires further research. Many events that are currently filtered out may actually be relevant to MSP operations. A deeper analysis of the event relationships and their impact on MSP operations is needed to determine the complete set of events that should be indexed.

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

## Events Requiring Further Analysis

Based on initial research, the following FileSystem events appear to be MSP-related but are currently filtered out:

### Potentially Should Be Indexed:
- **StorageRequestFulfilled**: Marks successful completion of storage request the MSP accepted
- **StorageRequestExpired**: Storage request expired after MSP accepted but BSP replication target wasn't met
- **StorageRequestRevoked**: User revoked request, MSP must delete the file
- **BucketPrivacyUpdated**: Affects access control for MSP-stored files
- **BucketDeleted**: Empty bucket deletion, MSP no longer responsible
- **MoveBucketRequested**: Pending bucket transfer affecting MSP
- **MoveBucketRejected**: MSP rejected bucket transfer request
- **MspStoppedStoringBucket**: MSP stopped storing a bucket
- **MspStopStoringBucketInsolventUser**: MSP forcefully removed insolvent user's bucket
- **FileDeletionRequest**: Requires MSP to provide proof and delete file
- **ProofSubmittedForPendingFileDeletionRequest**: MSP's response to deletion request
- **MoveBucketRequestExpired**: Transfer request expired without acceptance

### Analysis Needed:
Each of these events affects MSP operations by:
1. Changing storage capacity allocation
2. Modifying bucket ownership or properties
3. Requiring MSP action (deletions, proofs)
4. Tracking lifecycle of accepted storage requests
5. Documenting changes to MSP-managed buckets

A comprehensive review of the runtime implementation is needed to confirm which events should be indexed for complete MSP operational visibility.