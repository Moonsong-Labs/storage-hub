// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/api-base/types/errors";

import type { ApiTypes, AugmentedError } from "@polkadot/api-base/types";

export type __AugmentedError<ApiType extends ApiTypes> = AugmentedError<ApiType>;

declare module "@polkadot/api-base/types/errors" {
  interface AugmentedErrors<ApiType extends ApiTypes> {
    babe: {
      /**
       * A given equivocation report is valid but already previously reported.
       **/
      DuplicateOffenceReport: AugmentedError<ApiType>;
      /**
       * Submitted configuration is invalid.
       **/
      InvalidConfiguration: AugmentedError<ApiType>;
      /**
       * An equivocation proof provided as part of an equivocation report is invalid.
       **/
      InvalidEquivocationProof: AugmentedError<ApiType>;
      /**
       * A key ownership proof provided as part of an equivocation report is invalid.
       **/
      InvalidKeyOwnershipProof: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    balances: {
      /**
       * Beneficiary account must pre-exist.
       **/
      DeadAccount: AugmentedError<ApiType>;
      /**
       * The delta cannot be zero.
       **/
      DeltaZero: AugmentedError<ApiType>;
      /**
       * Value too low to create account due to existential deposit.
       **/
      ExistentialDeposit: AugmentedError<ApiType>;
      /**
       * A vesting schedule already exists for this account.
       **/
      ExistingVestingSchedule: AugmentedError<ApiType>;
      /**
       * Transfer/payment would kill account.
       **/
      Expendability: AugmentedError<ApiType>;
      /**
       * Balance too low to send value.
       **/
      InsufficientBalance: AugmentedError<ApiType>;
      /**
       * The issuance cannot be modified since it is already deactivated.
       **/
      IssuanceDeactivated: AugmentedError<ApiType>;
      /**
       * Account liquidity restrictions prevent withdrawal.
       **/
      LiquidityRestrictions: AugmentedError<ApiType>;
      /**
       * Number of freezes exceed `MaxFreezes`.
       **/
      TooManyFreezes: AugmentedError<ApiType>;
      /**
       * Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`.
       **/
      TooManyHolds: AugmentedError<ApiType>;
      /**
       * Number of named reserves exceed `MaxReserves`.
       **/
      TooManyReserves: AugmentedError<ApiType>;
      /**
       * Vesting balance too high to send value.
       **/
      VestingBalance: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    bucketNfts: {
      /**
       * Bucket is not private. Call `update_bucket_privacy` from the file system pallet to make it private.
       **/
      BucketIsNotPrivate: AugmentedError<ApiType>;
      /**
       * Failed to convert bytes to `BoundedVec`
       **/
      ConvertBytesToBoundedVec: AugmentedError<ApiType>;
      /**
       * No collection corresponding to the bucket. Call `update_bucket_privacy` from the file system pallet to make it private.
       **/
      NoCorrespondingCollection: AugmentedError<ApiType>;
      /**
       * Account is not the owner of the bucket.
       **/
      NotBucketOwner: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    ethereum: {
      /**
       * Signature is invalid.
       **/
      InvalidSignature: AugmentedError<ApiType>;
      /**
       * Pre-log is present, therefore transact is not allowed.
       **/
      PreLogExists: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    evm: {
      /**
       * Not enough balance to perform action
       **/
      BalanceLow: AugmentedError<ApiType>;
      /**
       * Calculating total fee overflowed
       **/
      FeeOverflow: AugmentedError<ApiType>;
      /**
       * Gas limit is too high.
       **/
      GasLimitTooHigh: AugmentedError<ApiType>;
      /**
       * Gas limit is too low.
       **/
      GasLimitTooLow: AugmentedError<ApiType>;
      /**
       * Gas price is too low.
       **/
      GasPriceTooLow: AugmentedError<ApiType>;
      /**
       * The chain id is invalid.
       **/
      InvalidChainId: AugmentedError<ApiType>;
      /**
       * Nonce is invalid
       **/
      InvalidNonce: AugmentedError<ApiType>;
      /**
       * the signature is invalid.
       **/
      InvalidSignature: AugmentedError<ApiType>;
      /**
       * Calculating total payment overflowed
       **/
      PaymentOverflow: AugmentedError<ApiType>;
      /**
       * EVM reentrancy
       **/
      Reentrancy: AugmentedError<ApiType>;
      /**
       * EIP-3607,
       **/
      TransactionMustComeFromEOA: AugmentedError<ApiType>;
      /**
       * Undefined error.
       **/
      Undefined: AugmentedError<ApiType>;
      /**
       * Withdraw fee failed
       **/
      WithdrawFailed: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    fileSystem: {
      /**
       * Batch file deletion must contain files from a single bucket only.
       **/
      BatchFileDeletionMustContainSingleBucket: AugmentedError<ApiType>;
      /**
       * BSP has already confirmed storing the given file.
       **/
      BspAlreadyConfirmed: AugmentedError<ApiType>;
      /**
       * BSP already volunteered to store the given file.
       **/
      BspAlreadyVolunteered: AugmentedError<ApiType>;
      /**
       * BSP has not confirmed storing the given file.
       **/
      BspNotConfirmed: AugmentedError<ApiType>;
      /**
       * BSP cannot volunteer at this current tick.
       **/
      BspNotEligibleToVolunteer: AugmentedError<ApiType>;
      /**
       * BSP has not volunteered to store the given file.
       **/
      BspNotVolunteered: AugmentedError<ApiType>;
      /**
       * Action not allowed while the bucket is being moved.
       **/
      BucketIsBeingMoved: AugmentedError<ApiType>;
      /**
       * Bucket is not empty.
       **/
      BucketNotEmpty: AugmentedError<ApiType>;
      /**
       * Bucket does not exist
       **/
      BucketNotFound: AugmentedError<ApiType>;
      /**
       * Cannot hold the required deposit from the user
       **/
      CannotHoldDeposit: AugmentedError<ApiType>;
      /**
       * Collection ID was not found.
       **/
      CollectionNotFound: AugmentedError<ApiType>;
      /**
       * Duplicate file key detected within the same batch deletion request.
       **/
      DuplicateFileKeyInBatchFileDeletion: AugmentedError<ApiType>;
      /**
       * Failed to fetch the dynamic-rate payment stream.
       **/
      DynamicRatePaymentStreamNotFound: AugmentedError<ApiType>;
      /**
       * Failed to verify proof: required to provide a proof of inclusion.
       **/
      ExpectedInclusionProof: AugmentedError<ApiType>;
      /**
       * Failed to verify proof: required to provide a proof of non-inclusion.
       **/
      ExpectedNonInclusionProof: AugmentedError<ApiType>;
      /**
       * Failed to compute file key
       **/
      FailedToComputeFileKey: AugmentedError<ApiType>;
      /**
       * Failed to create file metadata
       **/
      FailedToCreateFileMetadata: AugmentedError<ApiType>;
      /**
       * Failed to get owner account of ID of provider
       **/
      FailedToGetOwnerAccount: AugmentedError<ApiType>;
      /**
       * Failed to get the payment account of the provider.
       **/
      FailedToGetPaymentAccount: AugmentedError<ApiType>;
      /**
       * Failed to push file key to bounded vector during BSP file deletion
       **/
      FailedToPushFileKeyToBspDeletionVector: AugmentedError<ApiType>;
      /**
       * Failed to push file key to bounded vector during bucket file deletion
       **/
      FailedToPushFileKeyToBucketDeletionVector: AugmentedError<ApiType>;
      /**
       * Failed to push user to bounded vector during BSP file deletion
       **/
      FailedToPushUserToBspDeletionVector: AugmentedError<ApiType>;
      /**
       * Failed to query earliest volunteer tick
       **/
      FailedToQueryEarliestFileVolunteerTick: AugmentedError<ApiType>;
      /**
       * File has an active storage request and as such is not eligible for deletion.
       * The user should use the `revoke_storage_request` extrinsic to revoke it first.
       **/
      FileHasActiveStorageRequest: AugmentedError<ApiType>;
      /**
       * File has an `IncompleteStorageRequest` associated with it and as such is not eligible for a new storage request
       **/
      FileHasIncompleteStorageRequest: AugmentedError<ApiType>;
      /**
       * The bounded vector that holds file metadata to process it is full but there's still more to process.
       **/
      FileMetadataProcessingQueueFull: AugmentedError<ApiType>;
      /**
       * File size cannot be zero.
       **/
      FileSizeCannotBeZero: AugmentedError<ApiType>;
      /**
       * Failed to fetch the rate for the payment stream.
       **/
      FixedRatePaymentStreamNotFound: AugmentedError<ApiType>;
      /**
       * Failed to get value when just checked it existed.
       **/
      ImpossibleFailedToGetValue: AugmentedError<ApiType>;
      /**
       * Incomplete storage request not found.
       **/
      IncompleteStorageRequestNotFound: AugmentedError<ApiType>;
      /**
       * SP does not have enough storage capacity to store the file.
       **/
      InsufficientAvailableCapacity: AugmentedError<ApiType>;
      /**
       * Bucket id and file key pair is invalid.
       **/
      InvalidBucketIdFileKeyPair: AugmentedError<ApiType>;
      /**
       * Metadata does not correspond to expected file key.
       **/
      InvalidFileKeyMetadata: AugmentedError<ApiType>;
      /**
       * Invalid provider ID provided.
       **/
      InvalidProviderID: AugmentedError<ApiType>;
      /**
       * Invalid signature provided for file operation
       **/
      InvalidSignature: AugmentedError<ApiType>;
      /**
       * Invalid signed operation provided.
       **/
      InvalidSignedOperation: AugmentedError<ApiType>;
      /**
       * Error created in 2024. If you see this, you are well beyond the singularity and should
       * probably stop using this pallet.
       **/
      MaxTickNumberReached: AugmentedError<ApiType>;
      /**
       * Minimum amount of blocks between the request opening and being able to confirm it not reached.
       **/
      MinWaitForStopStoringNotReached: AugmentedError<ApiType>;
      /**
       * Move bucket request not found in storage.
       **/
      MoveBucketRequestNotFound: AugmentedError<ApiType>;
      /**
       * The MSP is trying to confirm to store a file from a storage request that it has already confirmed to store.
       **/
      MspAlreadyConfirmed: AugmentedError<ApiType>;
      /**
       * The MSP is already storing the bucket.
       **/
      MspAlreadyStoringBucket: AugmentedError<ApiType>;
      /**
       * Unauthorized operation, signer is not an MSP of the bucket id.
       **/
      MspNotStoringBucket: AugmentedError<ApiType>;
      /**
       * No BSP reputation weight set.
       **/
      NoBspReputationWeightSet: AugmentedError<ApiType>;
      /**
       * No file keys to confirm storing
       **/
      NoFileKeysToConfirm: AugmentedError<ApiType>;
      /**
       * Requires at least 1 file key to be deleted.
       **/
      NoFileKeysToDelete: AugmentedError<ApiType>;
      /**
       * No global reputation weight set.
       **/
      NoGlobalReputationWeightSet: AugmentedError<ApiType>;
      /**
       * Account is not a BSP.
       **/
      NotABsp: AugmentedError<ApiType>;
      /**
       * Account is not a MSP.
       **/
      NotAMsp: AugmentedError<ApiType>;
      /**
       * Account is not a SP.
       **/
      NotASp: AugmentedError<ApiType>;
      /**
       * Operation failed because the account is not the owner of the bucket.
       **/
      NotBucketOwner: AugmentedError<ApiType>;
      /**
       * The MSP is trying to confirm to store a file from a storage request is not the one selected to store it.
       **/
      NotSelectedMsp: AugmentedError<ApiType>;
      /**
       * Operations not allowed for insolvent provider
       **/
      OperationNotAllowedForInsolventProvider: AugmentedError<ApiType>;
      /**
       * Certain operations (such as issuing new storage requests) are not allowed when interacting with insolvent users.
       **/
      OperationNotAllowedWithInsolventUser: AugmentedError<ApiType>;
      /**
       * Pending stop storing request already exists.
       **/
      PendingStopStoringRequestAlreadyExists: AugmentedError<ApiType>;
      /**
       * Pending stop storing request not found.
       **/
      PendingStopStoringRequestNotFound: AugmentedError<ApiType>;
      /**
       * Provider is not storing the file.
       **/
      ProviderNotStoringFile: AugmentedError<ApiType>;
      /**
       * Replication target cannot be zero.
       **/
      ReplicationTargetCannotBeZero: AugmentedError<ApiType>;
      /**
       * BSPs required for storage request cannot exceed the maximum allowed.
       **/
      ReplicationTargetExceedsMaximum: AugmentedError<ApiType>;
      /**
       * The MSP is trying to confirm to store a file from a storage request that does not have a MSP assigned.
       **/
      RequestWithoutMsp: AugmentedError<ApiType>;
      /**
       * Root was not updated after applying delta
       **/
      RootNotUpdated: AugmentedError<ApiType>;
      /**
       * Storage request already registered for the given file.
       **/
      StorageRequestAlreadyRegistered: AugmentedError<ApiType>;
      /**
       * Number of BSPs required for storage request has been reached.
       **/
      StorageRequestBspsRequiredFulfilled: AugmentedError<ApiType>;
      /**
       * Operation not allowed while the storage request exists.
       **/
      StorageRequestExists: AugmentedError<ApiType>;
      /**
       * Not authorized to delete the storage request.
       **/
      StorageRequestNotAuthorized: AugmentedError<ApiType>;
      /**
       * Storage request not registered for the given file.
       **/
      StorageRequestNotFound: AugmentedError<ApiType>;
      /**
       * Arithmetic error in threshold calculation.
       **/
      ThresholdArithmeticError: AugmentedError<ApiType>;
      /**
       * Too many storage request responses.
       **/
      TooManyStorageRequestResponses: AugmentedError<ApiType>;
      /**
       * A SP tried to stop storing files from a user that was supposedly insolvent, but the user is not insolvent.
       **/
      UserNotInsolvent: AugmentedError<ApiType>;
      /**
       * The selected value proposition is not available in the MSP.
       **/
      ValuePropositionNotAvailable: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    grandpa: {
      /**
       * Attempt to signal GRANDPA change with one already pending.
       **/
      ChangePending: AugmentedError<ApiType>;
      /**
       * A given equivocation report is valid but already previously reported.
       **/
      DuplicateOffenceReport: AugmentedError<ApiType>;
      /**
       * An equivocation proof provided as part of an equivocation report is invalid.
       **/
      InvalidEquivocationProof: AugmentedError<ApiType>;
      /**
       * A key ownership proof provided as part of an equivocation report is invalid.
       **/
      InvalidKeyOwnershipProof: AugmentedError<ApiType>;
      /**
       * Attempt to signal GRANDPA pause when the authority set isn't live
       * (either paused or already pending pause).
       **/
      PauseFailed: AugmentedError<ApiType>;
      /**
       * Attempt to signal GRANDPA resume when the authority set isn't paused
       * (either live or already pending resume).
       **/
      ResumeFailed: AugmentedError<ApiType>;
      /**
       * Cannot signal forced change so soon after last.
       **/
      TooSoon: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    nfts: {
      /**
       * The provided Item was already used for claiming.
       **/
      AlreadyClaimed: AugmentedError<ApiType>;
      /**
       * The item ID has already been used for an item.
       **/
      AlreadyExists: AugmentedError<ApiType>;
      /**
       * The approval had a deadline that expired, so the approval isn't valid anymore.
       **/
      ApprovalExpired: AugmentedError<ApiType>;
      /**
       * The provided attribute can't be found.
       **/
      AttributeNotFound: AugmentedError<ApiType>;
      /**
       * The witness data given does not match the current state of the chain.
       **/
      BadWitness: AugmentedError<ApiType>;
      /**
       * The provided bid is too low.
       **/
      BidTooLow: AugmentedError<ApiType>;
      /**
       * Collection ID is already taken.
       **/
      CollectionIdInUse: AugmentedError<ApiType>;
      /**
       * Can't delete non-empty collections.
       **/
      CollectionNotEmpty: AugmentedError<ApiType>;
      /**
       * The deadline has already expired.
       **/
      DeadlineExpired: AugmentedError<ApiType>;
      /**
       * Item's config already exists and should be equal to the provided one.
       **/
      InconsistentItemConfig: AugmentedError<ApiType>;
      /**
       * The provided data is incorrect.
       **/
      IncorrectData: AugmentedError<ApiType>;
      /**
       * The provided metadata might be too long.
       **/
      IncorrectMetadata: AugmentedError<ApiType>;
      /**
       * The item is locked (non-transferable).
       **/
      ItemLocked: AugmentedError<ApiType>;
      /**
       * Items within that collection are non-transferable.
       **/
      ItemsNonTransferable: AugmentedError<ApiType>;
      /**
       * Collection's attributes are locked.
       **/
      LockedCollectionAttributes: AugmentedError<ApiType>;
      /**
       * Collection's metadata is locked.
       **/
      LockedCollectionMetadata: AugmentedError<ApiType>;
      /**
       * Item's attributes are locked.
       **/
      LockedItemAttributes: AugmentedError<ApiType>;
      /**
       * Item's metadata is locked.
       **/
      LockedItemMetadata: AugmentedError<ApiType>;
      /**
       * Can't set more attributes per one call.
       **/
      MaxAttributesLimitReached: AugmentedError<ApiType>;
      /**
       * The max supply is locked and can't be changed.
       **/
      MaxSupplyLocked: AugmentedError<ApiType>;
      /**
       * All items have been minted.
       **/
      MaxSupplyReached: AugmentedError<ApiType>;
      /**
       * The provided max supply is less than the number of items a collection already has.
       **/
      MaxSupplyTooSmall: AugmentedError<ApiType>;
      /**
       * The given item has no metadata set.
       **/
      MetadataNotFound: AugmentedError<ApiType>;
      /**
       * The method is disabled by system settings.
       **/
      MethodDisabled: AugmentedError<ApiType>;
      /**
       * Mint has already ended.
       **/
      MintEnded: AugmentedError<ApiType>;
      /**
       * Mint has not started yet.
       **/
      MintNotStarted: AugmentedError<ApiType>;
      /**
       * Config for a collection or an item can't be found.
       **/
      NoConfig: AugmentedError<ApiType>;
      /**
       * The signing account has no permission to do the operation.
       **/
      NoPermission: AugmentedError<ApiType>;
      /**
       * The provided account is not a delegate.
       **/
      NotDelegate: AugmentedError<ApiType>;
      /**
       * Item is not for sale.
       **/
      NotForSale: AugmentedError<ApiType>;
      /**
       * The item has reached its approval limit.
       **/
      ReachedApprovalLimit: AugmentedError<ApiType>;
      /**
       * Some roles were not cleared.
       **/
      RolesNotCleared: AugmentedError<ApiType>;
      /**
       * The named owner has not signed ownership acceptance of the collection.
       **/
      Unaccepted: AugmentedError<ApiType>;
      /**
       * No approval exists that would allow the transfer.
       **/
      Unapproved: AugmentedError<ApiType>;
      /**
       * The given item ID is unknown.
       **/
      UnknownCollection: AugmentedError<ApiType>;
      /**
       * The given item ID is unknown.
       **/
      UnknownItem: AugmentedError<ApiType>;
      /**
       * Swap doesn't exist.
       **/
      UnknownSwap: AugmentedError<ApiType>;
      /**
       * The witness data should be provided.
       **/
      WitnessRequired: AugmentedError<ApiType>;
      /**
       * The delegate turned out to be different to what was expected.
       **/
      WrongDelegate: AugmentedError<ApiType>;
      /**
       * The duration provided should be less than or equal to `MaxDeadlineDuration`.
       **/
      WrongDuration: AugmentedError<ApiType>;
      /**
       * The provided namespace isn't supported in this call.
       **/
      WrongNamespace: AugmentedError<ApiType>;
      /**
       * The extrinsic was sent by the wrong origin.
       **/
      WrongOrigin: AugmentedError<ApiType>;
      /**
       * The owner turned out to be different to what was expected.
       **/
      WrongOwner: AugmentedError<ApiType>;
      /**
       * The provided setting can't be set.
       **/
      WrongSetting: AugmentedError<ApiType>;
      /**
       * The provided signature is incorrect.
       **/
      WrongSignature: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    paymentStreams: {
      /**
       * Error thrown when trying to create a new dynamic-rate payment stream with amount provided 0 or update the amount provided of an existing one to 0 (should use remove_dynamic_rate_payment_stream instead)
       **/
      AmountProvidedCantBeZero: AugmentedError<ApiType>;
      /**
       * Error thrown when the system can't hold funds from the User as a deposit for creating a new payment stream
       **/
      CannotHoldDeposit: AugmentedError<ApiType>;
      /**
       * Error thrown when charging a payment stream would result in an overflow of the balance type
       **/
      ChargeOverflow: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to clear the flag of being without funds before the cooldown period has passed
       **/
      CooldownPeriodNotPassed: AugmentedError<ApiType>;
      /**
       * Error thrown when the new last chargeable tick number that is trying to be set is greater than the current tick number or smaller than the previous last chargeable tick number
       **/
      InvalidLastChargeableBlockNumber: AugmentedError<ApiType>;
      /**
       * Error thrown when the new last chargeable price index that is trying to be set is greater than the current price index or smaller than the previous last chargeable price index
       **/
      InvalidLastChargeablePriceIndex: AugmentedError<ApiType>;
      /**
       * Error thrown when the tick number of when the payment stream was last charged is greater than the tick number of the last chargeable tick
       **/
      LastChargedGreaterThanLastChargeable: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to charge a payment stream and it's not a registered Provider
       **/
      NotAProvider: AugmentedError<ApiType>;
      /**
       * Error thrown when a user of this pallet tries to add a payment stream that already exists.
       **/
      PaymentStreamAlreadyExists: AugmentedError<ApiType>;
      /**
       * Error thrown when a user of this pallet tries to update, remove or charge a payment stream that does not exist.
       **/
      PaymentStreamNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when failing to get the payment account of a registered Provider
       **/
      ProviderInconsistencyError: AugmentedError<ApiType>;
      /**
       * Error thrown when a charge is attempted when the provider is marked as insolvent
       **/
      ProviderInsolvent: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to create a new fixed-rate payment stream with rate 0 or update the rate of an existing one to 0 (should use remove_fixed_rate_payment_stream instead)
       **/
      RateCantBeZero: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to update the amount provided of a dynamic-rate payment stream to the same amount as before
       **/
      UpdateAmountToSameAmount: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to update the rate of a fixed-rate payment stream to the same rate as before
       **/
      UpdateRateToSameRate: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to clear the flag of being without funds before paying all its remaining debt
       **/
      UserHasRemainingDebt: AugmentedError<ApiType>;
      /**
       * Error thrown when a user that has not been flagged as without funds tries to use the extrinsic to pay its outstanding debt
       **/
      UserNotFlaggedAsWithoutFunds: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to operate when the User has been flagged for not having enough funds.
       **/
      UserWithoutFunds: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    proofsDealer: {
      /**
       * `challenge` extrinsic errors
       * The ChallengesQueue is full. No more manual challenges can be made
       * until some of the challenges in the queue are dispatched.
       **/
      ChallengesQueueOverflow: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof for a tick in the future.
       **/
      ChallengesTickNotReached: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof for a tick too late, i.e. that the challenges tick
       * is greater or equal than `challenges_tick` + `T::ChallengeTicksTolerance::get()`.
       **/
      ChallengesTickTooLate: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof for a tick before the last tick this pallet registers
       * challenges for.
       **/
      ChallengesTickTooOld: AugmentedError<ApiType>;
      /**
       * Checkpoint challenges not found in block.
       * This should only be possible if `TickToCheckpointChallenges` is dereferenced for a tick
       * that is not a checkpoint tick.
       **/
      CheckpointChallengesNotFound: AugmentedError<ApiType>;
      /**
       * `submit_proof` extrinsic errors
       * There are no key proofs submitted.
       **/
      EmptyKeyProofs: AugmentedError<ApiType>;
      /**
       * Failed to apply delta to the forest proof partial trie.
       **/
      FailedToApplyDelta: AugmentedError<ApiType>;
      /**
       * Failed to update the provider after a key removal mutation.
       **/
      FailedToUpdateProviderAfterKeyRemoval: AugmentedError<ApiType>;
      /**
       * The fee for submitting a challenge could not be charged.
       **/
      FeeChargeFailed: AugmentedError<ApiType>;
      /**
       * The forest proof submitted by the Provider is invalid.
       * This could be because the proof is not valid for the root, or because the proof is
       * not sufficient for the challenges made.
       **/
      ForestProofVerificationFailed: AugmentedError<ApiType>;
      /**
       * The number of key proofs submitted does not match the number of keys proven in the forest proof.
       **/
      IncorrectNumberOfKeyProofs: AugmentedError<ApiType>;
      /**
       * There is at least one key proven in the forest proof, that does not have a corresponding
       * key proof.
       **/
      KeyProofNotFound: AugmentedError<ApiType>;
      /**
       * A key proof submitted by the Provider is invalid.
       * This could be because the proof is not valid for the root of that key, or because the proof
       * is not sufficient for the challenges made.
       **/
      KeyProofVerificationFailed: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof but there is no record of the last tick they
       * submitted a proof for.
       * Providers who are required to submit proofs should always have a record of the
       * last tick they submitted a proof for, otherwise it means they haven't started
       * providing service for any user yet.
       **/
      NoRecordOfLastSubmittedProof: AugmentedError<ApiType>;
      /**
       * General errors
       * The proof submitter is not a registered Provider.
       **/
      NotProvider: AugmentedError<ApiType>;
      /**
       * The PriorityChallengesQueue is full. No more priority challenges can be made
       * until some of the challenges in the queue are dispatched.
       **/
      PriorityChallengesQueueOverflow: AugmentedError<ApiType>;
      /**
       * The root for the Provider could not be found.
       **/
      ProviderRootNotFound: AugmentedError<ApiType>;
      /**
       * The provider stake could not be found.
       **/
      ProviderStakeNotFound: AugmentedError<ApiType>;
      /**
       * The seed for the tick could not be found.
       * This should not be possible for a tick within the `ChallengeHistoryLength` range, as
       * seeds are generated for all ticks, and stored within this range.
       **/
      SeedNotFound: AugmentedError<ApiType>;
      /**
       * The staked balance of the Provider could not be converted to `u128`.
       * This should not be possible, as the `Balance` type should be an unsigned integer type.
       **/
      StakeCouldNotBeConverted: AugmentedError<ApiType>;
      /**
       * The limit of Providers that can submit a proof in a single tick has been reached.
       **/
      TooManyValidProofSubmitters: AugmentedError<ApiType>;
      /**
       * After successfully applying delta for a set of mutations, the number of mutated keys is
       * not the same as the number of mutations expected to have been applied.
       **/
      UnexpectedNumberOfRemoveMutations: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof when they have a zero root.
       * Providers with zero roots are not providing any service, so they should not be
       * submitting proofs.
       **/
      ZeroRoot: AugmentedError<ApiType>;
      /**
       * Provider is submitting a proof but their stake is zero.
       **/
      ZeroStake: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    providers: {
      /**
       * Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
       **/
      AlreadyRegistered: AugmentedError<ApiType>;
      /**
       * Error thrown when a bucket ID could not be added to the list of buckets of a MSP.
       **/
      AppendBucketToMspFailed: AugmentedError<ApiType>;
      /**
       * An operation dedicated to BSPs only
       **/
      BspOnlyOperation: AugmentedError<ApiType>;
      /**
       * Error thrown when a bucket ID already exists in storage.
       **/
      BucketAlreadyExists: AugmentedError<ApiType>;
      /**
       * Error thrown when a bucket has no value proposition.
       **/
      BucketHasNoValueProposition: AugmentedError<ApiType>;
      /**
       * Error thrown when an operation requires an MSP to be storing the bucket.
       **/
      BucketMustHaveMspForOperation: AugmentedError<ApiType>;
      /**
       * Bucket cannot be deleted because it is not empty.
       **/
      BucketNotEmpty: AugmentedError<ApiType>;
      /**
       * Error thrown when a bucket ID is not found in storage.
       **/
      BucketNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when a user exceeded the bucket data limit based on the associated value proposition.
       **/
      BucketSizeExceedsLimit: AugmentedError<ApiType>;
      /**
       * Error thrown when, after moving all buckets of a MSP when removing it from the system, the amount doesn't match the expected value.
       **/
      BucketsMovedAmountMismatch: AugmentedError<ApiType>;
      /**
       * Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP or change its capacity.
       **/
      CannotHoldDeposit: AugmentedError<ApiType>;
      /**
       * Cannot stop BSP cycles without a default root
       **/
      CannotStopCycleWithNonDefaultRoot: AugmentedError<ApiType>;
      /**
       * Error thrown when a MSP tries to deactivate its last value proposition.
       **/
      CantDeactivateLastValueProp: AugmentedError<ApiType>;
      /**
       * Failed to delete a provider due to conditions not being met.
       *
       * Call `can_delete_provider` runtime API to check if the provider can be deleted.
       **/
      DeleteProviderConditionsNotMet: AugmentedError<ApiType>;
      /**
       * Deposit too low to determine capacity.
       **/
      DepositTooLow: AugmentedError<ApiType>;
      /**
       * Error thrown when a fixed payment stream is not found.
       **/
      FixedRatePaymentStreamNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when failing to decode the owner Account ID from the received metadata.
       **/
      InvalidEncodedAccountId: AugmentedError<ApiType>;
      /**
       * Error thrown when failing to decode the metadata from a received trie value that was removed.
       **/
      InvalidEncodedFileMetadata: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid.
       **/
      InvalidMultiAddress: AugmentedError<ApiType>;
      /**
       * Error thrown when a Provider tries to remove the last MultiAddress from its account.
       **/
      LastMultiAddressCantBeRemoved: AugmentedError<ApiType>;
      /**
       * Congratulations, you either lived long enough or were born late enough to see this error.
       **/
      MaxBlockNumberReached: AugmentedError<ApiType>;
      /**
       * Error thrown when changing the MSP of a bucket to the same assigned MSP.
       **/
      MspAlreadyAssignedToBucket: AugmentedError<ApiType>;
      /**
       * An operation dedicated to MSPs only
       **/
      MspOnlyOperation: AugmentedError<ApiType>;
      /**
       * Error thrown when a Provider tries to add a new MultiAddress to its account but it already exists.
       **/
      MultiAddressAlreadyExists: AugmentedError<ApiType>;
      /**
       * Error thrown when a Provider tries to add a new MultiAddress to its account but it already has the maximum amount of multiaddresses.
       **/
      MultiAddressesMaxAmountReached: AugmentedError<ApiType>;
      /**
       * Error thrown when a Provider tries to delete a MultiAddress from its account but it does not have that MultiAddress.
       **/
      MultiAddressNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to change its capacity to zero (there are specific extrinsics to sign off as a SP).
       **/
      NewCapacityCantBeZero: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to change its capacity to the same value it already has.
       **/
      NewCapacityEqualsCurrentCapacity: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to change its capacity to less than its used storage.
       **/
      NewCapacityLessThanUsedStorage: AugmentedError<ApiType>;
      /**
       * Error thrown when a SP tries to change its capacity but the new capacity is not enough to store the used storage.
       **/
      NewUsedCapacityExceedsStorageCapacity: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to get a root from a MSP without passing a Bucket ID.
       **/
      NoBucketId: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up without any multiaddress.
       **/
      NoMultiAddress: AugmentedError<ApiType>;
      /**
       * Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its capacity.
       **/
      NotEnoughBalance: AugmentedError<ApiType>;
      /**
       * Error thrown when a SP tries to change its capacity but it has not been enough time since the last time it changed it.
       **/
      NotEnoughTimePassed: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to interact as a SP but is not registered as a MSP or BSP.
       **/
      NotRegistered: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to get a root from a MSP without passing a User ID.
       **/
      NoUserId: AugmentedError<ApiType>;
      /**
       * Operation not allowed for insolvent provider
       **/
      OperationNotAllowedForInsolventProvider: AugmentedError<ApiType>;
      /**
       * Error thrown when trying to update a payment stream that does not exist.
       **/
      PaymentStreamNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when an attempt was made to slash an unslashable Storage Provider.
       **/
      ProviderNotSlashable: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to confirm a sign up but the randomness is too fresh to be used yet.
       **/
      RandomnessNotValidYet: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign off as a BSP but the sign off period has not passed yet.
       **/
      SignOffPeriodNotPassed: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to confirm a sign up that was not requested previously.
       **/
      SignUpNotRequested: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to confirm a sign up but too much time has passed since the request.
       **/
      SignUpRequestExpired: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to request to sign up when it already has a sign up request pending.
       **/
      SignUpRequestPending: AugmentedError<ApiType>;
      /**
       * Error thrown when a user has a SP ID assigned to it but the SP data does not exist in storage (Inconsistency error).
       **/
      SpRegisteredButDataNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign off as a SP but still has used storage.
       **/
      StorageStillInUse: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up or change its capacity to store less storage than the minimum required by the runtime.
       **/
      StorageTooLow: AugmentedError<ApiType>;
      /**
       * Error thrown when a provider attempts to top up their deposit when not required.
       **/
      TopUpNotRequired: AugmentedError<ApiType>;
      /**
       * Error thrown when value proposition under a given id already exists.
       **/
      ValuePropositionAlreadyExists: AugmentedError<ApiType>;
      /**
       * Error thrown when a value proposition is not available.
       **/
      ValuePropositionNotAvailable: AugmentedError<ApiType>;
      /**
       * Error thrown when the value proposition id is not found.
       **/
      ValuePropositionNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when, after deleting all value propositions of a MSP when removing it from the system, the amount doesn't match the expected value.
       **/
      ValuePropositionsDeletedAmountMismatch: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    session: {
      /**
       * Registered duplicate key.
       **/
      DuplicatedKey: AugmentedError<ApiType>;
      /**
       * Invalid ownership proof.
       **/
      InvalidProof: AugmentedError<ApiType>;
      /**
       * Key setting account is not live, so it's impossible to associate keys.
       **/
      NoAccount: AugmentedError<ApiType>;
      /**
       * No associated validator ID for account.
       **/
      NoAssociatedValidatorId: AugmentedError<ApiType>;
      /**
       * No keys are associated with this account.
       **/
      NoKeys: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    sudo: {
      /**
       * Sender must be the Sudo account.
       **/
      RequireSudo: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    system: {
      /**
       * The origin filter prevent the call to be dispatched.
       **/
      CallFiltered: AugmentedError<ApiType>;
      /**
       * Failed to extract the runtime version from the new runtime.
       *
       * Either calling `Core_version` or decoding `RuntimeVersion` failed.
       **/
      FailedToExtractRuntimeVersion: AugmentedError<ApiType>;
      /**
       * The name of specification does not match between the current runtime
       * and the new runtime.
       **/
      InvalidSpecName: AugmentedError<ApiType>;
      /**
       * A multi-block migration is ongoing and prevents the current code from being replaced.
       **/
      MultiBlockMigrationsOngoing: AugmentedError<ApiType>;
      /**
       * Suicide called when the account has non-default composite data.
       **/
      NonDefaultComposite: AugmentedError<ApiType>;
      /**
       * There is a non-zero reference count preventing the account from being purged.
       **/
      NonZeroRefCount: AugmentedError<ApiType>;
      /**
       * No upgrade authorized.
       **/
      NothingAuthorized: AugmentedError<ApiType>;
      /**
       * The specification version is not allowed to decrease between the current runtime
       * and the new runtime.
       **/
      SpecVersionNeedsToIncrease: AugmentedError<ApiType>;
      /**
       * The submitted code is not authorized.
       **/
      Unauthorized: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
  } // AugmentedErrors
} // declare module
