// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/api-base/types/errors";

import type { ApiTypes, AugmentedError } from "@polkadot/api-base/types";

export type __AugmentedError<ApiType extends ApiTypes> = AugmentedError<ApiType>;

declare module "@polkadot/api-base/types/errors" {
  interface AugmentedErrors<ApiType extends ApiTypes> {
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
    collatorSelection: {
      /**
       * Account is already a candidate.
       **/
      AlreadyCandidate: AugmentedError<ApiType>;
      /**
       * Account is already an Invulnerable.
       **/
      AlreadyInvulnerable: AugmentedError<ApiType>;
      /**
       * New deposit amount would be below the minimum candidacy bond.
       **/
      DepositTooLow: AugmentedError<ApiType>;
      /**
       * The updated deposit amount is equal to the amount already reserved.
       **/
      IdenticalDeposit: AugmentedError<ApiType>;
      /**
       * Could not insert in the candidate list.
       **/
      InsertToCandidateListFailed: AugmentedError<ApiType>;
      /**
       * Deposit amount is too low to take the target's slot in the candidate list.
       **/
      InsufficientBond: AugmentedError<ApiType>;
      /**
       * Cannot lower candidacy bond while occupying a future collator slot in the list.
       **/
      InvalidUnreserve: AugmentedError<ApiType>;
      /**
       * Account has no associated validator ID.
       **/
      NoAssociatedValidatorId: AugmentedError<ApiType>;
      /**
       * Account is not a candidate.
       **/
      NotCandidate: AugmentedError<ApiType>;
      /**
       * Account is not an Invulnerable.
       **/
      NotInvulnerable: AugmentedError<ApiType>;
      /**
       * Could not remove from the candidate list.
       **/
      RemoveFromCandidateListFailed: AugmentedError<ApiType>;
      /**
       * The target account to be replaced in the candidate list is not a candidate.
       **/
      TargetIsNotCandidate: AugmentedError<ApiType>;
      /**
       * Leaving would result in too few candidates.
       **/
      TooFewEligibleCollators: AugmentedError<ApiType>;
      /**
       * The pallet has too many candidates.
       **/
      TooManyCandidates: AugmentedError<ApiType>;
      /**
       * There are too many Invulnerables.
       **/
      TooManyInvulnerables: AugmentedError<ApiType>;
      /**
       * Could not update the candidate list.
       **/
      UpdateCandidateListFailed: AugmentedError<ApiType>;
      /**
       * Validator ID is not yet registered.
       **/
      ValidatorNotRegistered: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    fileSystem: {
      /**
       * BSP did not succeed threshold check.
       **/
      AboveThreshold: AugmentedError<ApiType>;
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
       * BSP has not volunteered to store the given file.
       **/
      BspNotVolunteered: AugmentedError<ApiType>;
      /**
       * BSPs required for storage request cannot be 0.
       **/
      BspsRequiredCannotBeZero: AugmentedError<ApiType>;
      /**
       * BSPs required for storage request cannot exceed the maximum allowed.
       **/
      BspsRequiredExceedsMax: AugmentedError<ApiType>;
      /**
       * Bucket is not private. Call `update_bucket_privacy` to make it private.
       **/
      BucketIsNotPrivate: AugmentedError<ApiType>;
      /**
       * Bucket does not exist
       **/
      BucketNotFound: AugmentedError<ApiType>;
      /**
       * Divided by 0
       **/
      DividedByZero: AugmentedError<ApiType>;
      /**
       * Failed to verify proof: required to provide a proof of inclusion.
       **/
      ExpectedInclusionProof: AugmentedError<ApiType>;
      /**
       * Failed to verify proof: required to provide a proof of non-inclusion.
       **/
      ExpectedNonInclusionProof: AugmentedError<ApiType>;
      /**
       * Failed to add file key to pending deletion requests.
       **/
      FailedToAddFileKeyToPendingDeletionRequests: AugmentedError<ApiType>;
      /**
       * Failed to convert block number to threshold.
       **/
      FailedToConvertBlockNumber: AugmentedError<ApiType>;
      /**
       * Failed to decode threshold.
       **/
      FailedToDecodeThreshold: AugmentedError<ApiType>;
      /**
       * Failed to encode BSP id as slice.
       **/
      FailedToEncodeBsp: AugmentedError<ApiType>;
      /**
       * Failed to encode fingerprint as slice.
       **/
      FailedToEncodeFingerprint: AugmentedError<ApiType>;
      /**
       * Failed to convert to primitive type.
       **/
      FailedTypeConversion: AugmentedError<ApiType>;
      /**
       * File key already pending deletion.
       **/
      FileKeyAlreadyPendingDeletion: AugmentedError<ApiType>;
      /**
       * Failed to get value when just checked it existed.
       **/
      ImpossibleFailedToGetValue: AugmentedError<ApiType>;
      /**
       * Metadata does not correspond to expected file key.
       **/
      InvalidFileKeyMetadata: AugmentedError<ApiType>;
      /**
       * Error created in 2024. If you see this, you are well beyond the singularity and should
       * probably stop using this pallet.
       **/
      MaxBlockNumberReached: AugmentedError<ApiType>;
      /**
       * Unauthorized operation, signer is not an MSP of the bucket id.
       **/
      MspNotStoringBucket: AugmentedError<ApiType>;
      /**
       * Account is not a BSP.
       **/
      NotABsp: AugmentedError<ApiType>;
      /**
       * Account is not a MSP.
       **/
      NotAMsp: AugmentedError<ApiType>;
      /**
       * Operation failed because the account is not the owner of the bucket.
       **/
      NotBucketOwner: AugmentedError<ApiType>;
      /**
       * Unauthorized operation, signer does not own the file.
       **/
      NotFileOwner: AugmentedError<ApiType>;
      /**
       * Root of the provider not found.
       **/
      ProviderRootNotFound: AugmentedError<ApiType>;
      /**
       * Storage request already registered for the given file.
       **/
      StorageRequestAlreadyRegistered: AugmentedError<ApiType>;
      /**
       * Number of BSPs required for storage request has been reached.
       **/
      StorageRequestBspsRequiredFulfilled: AugmentedError<ApiType>;
      /**
       * No slot available found in blocks to insert storage request expiration time.
       **/
      StorageRequestExpiredNoSlotAvailable: AugmentedError<ApiType>;
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
       * BSPs assignment threshold cannot be below asymptote.
       **/
      ThresholdBelowAsymptote: AugmentedError<ApiType>;
      /**
       * Number of removed BSPs volunteered from storage request prefix did not match the expected number.
       **/
      UnexpectedNumberOfRemovedVolunteeredBsps: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    messageQueue: {
      /**
       * The message was already processed and cannot be processed again.
       **/
      AlreadyProcessed: AugmentedError<ApiType>;
      /**
       * There is temporarily not enough weight to continue servicing messages.
       **/
      InsufficientWeight: AugmentedError<ApiType>;
      /**
       * The referenced message could not be found.
       **/
      NoMessage: AugmentedError<ApiType>;
      /**
       * Page to be reaped does not exist.
       **/
      NoPage: AugmentedError<ApiType>;
      /**
       * Page is not reapable because it has items remaining to be processed and is not old
       * enough.
       **/
      NotReapable: AugmentedError<ApiType>;
      /**
       * The message is queued for future execution.
       **/
      Queued: AugmentedError<ApiType>;
      /**
       * The queue is paused and no message can be executed from it.
       *
       * This can change at any time and may resolve in the future by re-trying.
       **/
      QueuePaused: AugmentedError<ApiType>;
      /**
       * Another call is in progress and needs to finish before this call can happen.
       **/
      RecursiveDisallowed: AugmentedError<ApiType>;
      /**
       * This message is temporarily unprocessable.
       *
       * Such errors are expected, but not guaranteed, to resolve themselves eventually through
       * retrying.
       **/
      TemporarilyUnprocessable: AugmentedError<ApiType>;
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
    parachainSystem: {
      /**
       * The inherent which supplies the host configuration did not run this block.
       **/
      HostConfigurationNotAvailable: AugmentedError<ApiType>;
      /**
       * No code upgrade has been authorized.
       **/
      NothingAuthorized: AugmentedError<ApiType>;
      /**
       * No validation function upgrade is currently scheduled.
       **/
      NotScheduled: AugmentedError<ApiType>;
      /**
       * Attempt to upgrade validation function while existing upgrade pending.
       **/
      OverlappingUpgrades: AugmentedError<ApiType>;
      /**
       * Polkadot currently prohibits this parachain from upgrading its validation function.
       **/
      ProhibitedByPolkadot: AugmentedError<ApiType>;
      /**
       * The supplied validation function has compiled into a blob larger than Polkadot is
       * willing to run.
       **/
      TooBig: AugmentedError<ApiType>;
      /**
       * The given code upgrade has not been authorized.
       **/
      Unauthorized: AugmentedError<ApiType>;
      /**
       * The inherent which supplies the validation data did not run this block.
       **/
      ValidationDataNotAvailable: AugmentedError<ApiType>;
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
       * Error thrown when charging a payment stream would result in an overflow of the balance type (TODO: maybe we should use saturating arithmetic instead)
       **/
      ChargeOverflow: AugmentedError<ApiType>;
      /**
       * Error thrown when the new last chargeable block number that is trying to be set by the PaymentManager is greater than the current block number or smaller than the previous last chargeable block number
       **/
      InvalidLastChargeableBlockNumber: AugmentedError<ApiType>;
      /**
       * Error thrown when the new last chargeable price index that is trying to be set by the PaymentManager is greater than the current price index or smaller than the previous last chargeable price index
       **/
      InvalidLastChargeablePriceIndex: AugmentedError<ApiType>;
      /**
       * Error thrown when the block number of when the payment stream was last charged is greater than the block number of the last chargeable block
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
       * Error thrown when trying to operate when the User has been flagged for not having enough funds.
       **/
      UserWithoutFunds: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
    polkadotXcm: {
      /**
       * The given account is not an identifiable sovereign account for any location.
       **/
      AccountNotSovereign: AugmentedError<ApiType>;
      /**
       * The location is invalid since it already has a subscription from us.
       **/
      AlreadySubscribed: AugmentedError<ApiType>;
      /**
       * The given location could not be used (e.g. because it cannot be expressed in the
       * desired version of XCM).
       **/
      BadLocation: AugmentedError<ApiType>;
      /**
       * The version of the `Versioned` value used is not able to be interpreted.
       **/
      BadVersion: AugmentedError<ApiType>;
      /**
       * Could not check-out the assets for teleportation to the destination chain.
       **/
      CannotCheckOutTeleport: AugmentedError<ApiType>;
      /**
       * Could not re-anchor the assets to declare the fees for the destination chain.
       **/
      CannotReanchor: AugmentedError<ApiType>;
      /**
       * The destination `Location` provided cannot be inverted.
       **/
      DestinationNotInvertible: AugmentedError<ApiType>;
      /**
       * The assets to be sent are empty.
       **/
      Empty: AugmentedError<ApiType>;
      /**
       * The operation required fees to be paid which the initiator could not meet.
       **/
      FeesNotMet: AugmentedError<ApiType>;
      /**
       * The message execution fails the filter.
       **/
      Filtered: AugmentedError<ApiType>;
      /**
       * The unlock operation cannot succeed because there are still consumers of the lock.
       **/
      InUse: AugmentedError<ApiType>;
      /**
       * Invalid non-concrete asset.
       **/
      InvalidAssetNotConcrete: AugmentedError<ApiType>;
      /**
       * Invalid asset, reserve chain could not be determined for it.
       **/
      InvalidAssetUnknownReserve: AugmentedError<ApiType>;
      /**
       * Invalid asset, do not support remote asset reserves with different fees reserves.
       **/
      InvalidAssetUnsupportedReserve: AugmentedError<ApiType>;
      /**
       * Origin is invalid for sending.
       **/
      InvalidOrigin: AugmentedError<ApiType>;
      /**
       * Local XCM execution incomplete.
       **/
      LocalExecutionIncomplete: AugmentedError<ApiType>;
      /**
       * A remote lock with the corresponding data could not be found.
       **/
      LockNotFound: AugmentedError<ApiType>;
      /**
       * The owner does not own (all) of the asset that they wish to do the operation on.
       **/
      LowBalance: AugmentedError<ApiType>;
      /**
       * The referenced subscription could not be found.
       **/
      NoSubscription: AugmentedError<ApiType>;
      /**
       * There was some other issue (i.e. not to do with routing) in sending the message.
       * Perhaps a lack of space for buffering the message.
       **/
      SendFailure: AugmentedError<ApiType>;
      /**
       * Too many assets have been attempted for transfer.
       **/
      TooManyAssets: AugmentedError<ApiType>;
      /**
       * The asset owner has too many locks on the asset.
       **/
      TooManyLocks: AugmentedError<ApiType>;
      /**
       * Too many assets with different reserve locations have been attempted for transfer.
       **/
      TooManyReserves: AugmentedError<ApiType>;
      /**
       * The desired destination was unreachable, generally because there is a no way of routing
       * to it.
       **/
      Unreachable: AugmentedError<ApiType>;
      /**
       * The message's weight could not be determined.
       **/
      UnweighableMessage: AugmentedError<ApiType>;
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
       * Error thrown when a bucket ID already exists in storage.
       **/
      BucketAlreadyExists: AugmentedError<ApiType>;
      /**
       * Error thrown when a bucket ID is not found in storage.
       **/
      BucketNotFound: AugmentedError<ApiType>;
      /**
       * Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP or change its capacity.
       **/
      CannotHoldDeposit: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid.
       **/
      InvalidMultiAddress: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up as a BSP but the maximum amount of BSPs has been reached.
       **/
      MaxBspsReached: AugmentedError<ApiType>;
      /**
       * Error thrown when a user tries to sign up as a MSP but the maximum amount of MSPs has been reached.
       **/
      MaxMspsReached: AugmentedError<ApiType>;
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
       * Error thrown when a user tries to confirm a sign up but the randomness is too fresh to be used yet.
       **/
      RandomnessNotValidYet: AugmentedError<ApiType>;
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
    xcmpQueue: {
      /**
       * The execution is already resumed.
       **/
      AlreadyResumed: AugmentedError<ApiType>;
      /**
       * The execution is already suspended.
       **/
      AlreadySuspended: AugmentedError<ApiType>;
      /**
       * Setting the queue config failed since one of its values was invalid.
       **/
      BadQueueConfig: AugmentedError<ApiType>;
      /**
       * Generic error
       **/
      [key: string]: AugmentedError<ApiType>;
    };
  } // AugmentedErrors
} // declare module
