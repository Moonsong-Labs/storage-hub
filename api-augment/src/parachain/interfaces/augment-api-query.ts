// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/api-base/types/storage";

import type { ApiTypes, AugmentedQuery, QueryableStorageEntry } from "@polkadot/api-base/types";
import type {
  BTreeMap,
  BTreeSet,
  Bytes,
  Null,
  Option,
  Struct,
  Vec,
  bool,
  u128,
  u16,
  u32,
  u64,
  u8
} from "@polkadot/types-codec";
import type { AnyNumber, ITuple } from "@polkadot/types-codec/types";
import type { AccountId32, H256 } from "@polkadot/types/interfaces/runtime";
import type {
  CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot,
  CumulusPalletParachainSystemUnincludedSegmentAncestor,
  CumulusPalletParachainSystemUnincludedSegmentSegmentTracker,
  CumulusPalletXcmpQueueOutboundChannelDetails,
  CumulusPalletXcmpQueueQueueConfigData,
  CumulusPrimitivesCoreAggregateMessageOrigin,
  FrameSupportDispatchPerDispatchClassWeight,
  FrameSupportTokensMiscIdAmount,
  FrameSystemAccountInfo,
  FrameSystemCodeUpgradeAuthorization,
  FrameSystemEventRecord,
  FrameSystemLastRuntimeUpgradeInfo,
  FrameSystemPhase,
  PalletBalancesAccountData,
  PalletBalancesBalanceLock,
  PalletBalancesReserveData,
  PalletCollatorSelectionCandidateInfo,
  PalletFileSystemIncompleteStorageRequestMetadata,
  PalletFileSystemMoveBucketRequestMetadata,
  PalletFileSystemPendingFileDeletionRequest,
  PalletFileSystemPendingStopStoringRequest,
  PalletFileSystemStorageRequestBspsMetadata,
  PalletFileSystemStorageRequestMetadata,
  PalletMessageQueueBookState,
  PalletMessageQueuePage,
  PalletNftsAttributeDeposit,
  PalletNftsAttributeNamespace,
  PalletNftsCollectionConfig,
  PalletNftsCollectionDetails,
  PalletNftsCollectionMetadata,
  PalletNftsItemConfig,
  PalletNftsItemDetails,
  PalletNftsItemMetadata,
  PalletNftsPendingSwap,
  PalletPaymentStreamsDynamicRatePaymentStream,
  PalletPaymentStreamsFixedRatePaymentStream,
  PalletPaymentStreamsProviderLastChargeableInfo,
  PalletProofsDealerCustomChallenge,
  PalletProofsDealerProofSubmissionRecord,
  PalletStorageProvidersBackupStorageProvider,
  PalletStorageProvidersBucket,
  PalletStorageProvidersMainStorageProvider,
  PalletStorageProvidersSignUpRequest,
  PalletStorageProvidersStorageProviderId,
  PalletStorageProvidersTopUpMetadata,
  PalletStorageProvidersValueProposition,
  PalletTransactionPaymentReleases,
  PalletXcmQueryStatus,
  PalletXcmRemoteLockedFungibleRecord,
  PalletXcmVersionMigrationStage,
  PolkadotCorePrimitivesOutboundHrmpMessage,
  PolkadotPrimitivesV8AbridgedHostConfiguration,
  PolkadotPrimitivesV8PersistedValidationData,
  PolkadotPrimitivesV8UpgradeGoAhead,
  PolkadotPrimitivesV8UpgradeRestriction,
  ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey,
  ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue,
  ShParachainRuntimeRuntimeHoldReason,
  ShParachainRuntimeSessionKeys,
  SpConsensusAuraSr25519AppSr25519Public,
  SpCoreCryptoKeyTypeId,
  SpRuntimeDigest,
  SpTrieStorageProof,
  SpWeightsWeightV2Weight,
  StagingXcmV5Instruction,
  XcmVersionedAssetId,
  XcmVersionedLocation
} from "@polkadot/types/lookup";
import type { Observable } from "@polkadot/types/types";

export type __AugmentedQuery<ApiType extends ApiTypes> = AugmentedQuery<ApiType, () => unknown>;
export type __QueryableStorageEntry<ApiType extends ApiTypes> = QueryableStorageEntry<ApiType>;

declare module "@polkadot/api-base/types/storage" {
  interface AugmentedQueries<ApiType extends ApiTypes> {
    aura: {
      /**
       * The current authority set.
       **/
      authorities: AugmentedQuery<
        ApiType,
        () => Observable<Vec<SpConsensusAuraSr25519AppSr25519Public>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The current slot of this block.
       *
       * This will be set in `on_initialize`.
       **/
      currentSlot: AugmentedQuery<ApiType, () => Observable<u64>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    auraExt: {
      /**
       * Serves as cache for the authorities.
       *
       * The authorities in AuRa are overwritten in `on_initialize` when we switch to a new session,
       * but we require the old authorities to verify the seal when validating a PoV. This will
       * always be updated to the latest AuRa authorities in `on_finalize`.
       **/
      authorities: AugmentedQuery<
        ApiType,
        () => Observable<Vec<SpConsensusAuraSr25519AppSr25519Public>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Current slot paired with a number of authored blocks.
       *
       * Updated on each block initialization.
       **/
      slotInfo: AugmentedQuery<ApiType, () => Observable<Option<ITuple<[u64, u32]>>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    authorship: {
      /**
       * Author of current block.
       **/
      author: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    balances: {
      /**
       * The Balances pallet example of storing the balance of an account.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
       * }
       * ```
       *
       * You can also store the balance of an account in the `System` pallet.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = System
       * }
       * ```
       *
       * But this comes with tradeoffs, storing account balances in the system pallet stores
       * `frame_system` data alongside the account data contrary to storing account balances in the
       * `Balances` pallet, which uses a `StorageMap` to store balances data only.
       * NOTE: This is only used in the case that this pallet is used to store balances.
       **/
      account: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<PalletBalancesAccountData>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Freeze locks on account balances.
       **/
      freezes: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<FrameSupportTokensMiscIdAmount>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Holds on account balances.
       **/
      holds: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<
          Vec<
            {
              readonly id: ShParachainRuntimeRuntimeHoldReason;
              readonly amount: u128;
            } & Struct
          >
        >,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The total units of outstanding deactivated balance in the system.
       **/
      inactiveIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Any liquidity locks on some account balances.
       * NOTE: Should only be accessed when setting, changing and freeing a lock.
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      locks: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesBalanceLock>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Named reserves on some account balances.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      reserves: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesReserveData>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The total units issued in the system.
       **/
      totalIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    collatorSelection: {
      /**
       * Fixed amount to deposit to become a collator.
       *
       * When a collator calls `leave_intent` they immediately receive the deposit back.
       **/
      candidacyBond: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The (community, limited) collation candidates. `Candidates` and `Invulnerables` should be
       * mutually exclusive.
       *
       * This list is sorted in ascending order by deposit and when the deposits are equal, the least
       * recently updated is considered greater.
       **/
      candidateList: AugmentedQuery<
        ApiType,
        () => Observable<Vec<PalletCollatorSelectionCandidateInfo>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Desired number of candidates.
       *
       * This should ideally always be less than [`Config::MaxCandidates`] for weights to be correct.
       **/
      desiredCandidates: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The invulnerable, permissioned collators. This list must be sorted.
       **/
      invulnerables: AugmentedQuery<ApiType, () => Observable<Vec<AccountId32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Last block authored by collator.
       **/
      lastAuthoredBlock: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<u32>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    fileSystem: {
      /**
       * Bookkeeping of the buckets containing open storage requests.
       **/
      bucketsWithStorageRequests: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<Null>>,
        [H256, H256]
      > &
        QueryableStorageEntry<ApiType, [H256, H256]>;
      /**
       * Incomplete storage requests with pending provider file removal.
       *
       * This mapping tracks storage requests that have been expired or rejected but still have
       * confirmed providers storing files. Each entry tracks which providers still need to remove
       * their files. Once all providers have removed their files, the entry is automatically cleaned up.
       **/
      incompleteStorageRequests: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletFileSystemIncompleteStorageRequestMetadata>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * A map of ticks to expired move bucket requests.
       **/
      moveBucketRequestExpirations: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<H256>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * Mapping from MSPs to the amount of pending file deletion requests they have.
       *
       * This is used to keep track of the amount of pending file deletion requests each MSP has, so that MSPs are removed
       * from the privileged providers list if they have at least one, and are added back if they have none.
       * This is to ensure that MSPs are correctly incentivised to submit the required proofs for file deletions.
       **/
      mspsAmountOfPendingFileDeletionRequests: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<u32>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * A pointer to the earliest available tick to insert a new move bucket request expiration.
       *
       * This should always be greater or equal than current tick + [`Config::MoveBucketRequestTtl`].
       **/
      nextAvailableMoveBucketRequestExpirationTick: AugmentedQuery<
        ApiType,
        () => Observable<u32>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A pointer to the earliest available tick to insert a new storage request expiration.
       *
       * This should always be greater or equal than current tick + [`Config::StorageRequestTtl`].
       **/
      nextAvailableStorageRequestExpirationTick: AugmentedQuery<
        ApiType,
        () => Observable<u32>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A pointer to the starting tick to clean up expired items.
       *
       * If this tick is behind the current tick number, the cleanup algorithm in `on_idle` will
       * attempt to advance this tick pointer as close to or up to the current tick number. This
       * will execute provided that there is enough remaining weight to do so.
       **/
      nextStartingTickToCleanUp: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Pending file deletion requests.
       *
       * A mapping from a user Account ID to a list of pending file deletion requests (which have the file information).
       **/
      pendingFileDeletionRequests: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array
        ) => Observable<Vec<PalletFileSystemPendingFileDeletionRequest>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Pending move bucket requests.
       *
       * A mapping from Bucket ID to their move bucket request metadata, which includes the new MSP
       * and value propositions that this bucket would take if accepted.
       **/
      pendingMoveBucketRequests: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletFileSystemMoveBucketRequestMetadata>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * Pending file stop storing requests.
       *
       * A double mapping from BSP IDs to a list of file keys pending stop storing requests to the block in which those requests were opened,
       * the proven size of the file and the owner of the file.
       * The block number is used to avoid BSPs being able to stop storing files immediately which would allow them to avoid challenges
       * of missing files. The size is to be able to decrease their used capacity when they confirm to stop storing the file.
       * The owner is to be able to update the payment stream between the user and the BSP.
       **/
      pendingStopStoringRequests: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<PalletFileSystemPendingStopStoringRequest>>,
        [H256, H256]
      > &
        QueryableStorageEntry<ApiType, [H256, H256]>;
      /**
       * A double map from file key to the BSP IDs of the BSPs that volunteered to store the file to whether that BSP has confirmed storing it.
       *
       * Any BSP under a file key prefix is considered to be a volunteer and can be removed at any time.
       * Once a BSP submits a valid proof via the `bsp_confirm_storing` extrinsic, the `confirmed` field in [`StorageRequestBspsMetadata`] will be set to `true`.
       *
       * When a storage request is expired or removed, the corresponding file key prefix in this map is removed.
       **/
      storageRequestBsps: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<PalletFileSystemStorageRequestBspsMetadata>>,
        [H256, H256]
      > &
        QueryableStorageEntry<ApiType, [H256, H256]>;
      /**
       * A map of ticks to expired storage requests.
       **/
      storageRequestExpirations: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<H256>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      storageRequests: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletFileSystemStorageRequestMetadata>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    messageQueue: {
      /**
       * The index of the first and last (non-empty) pages.
       **/
      bookStateFor: AugmentedQuery<
        ApiType,
        (
          arg:
            | CumulusPrimitivesCoreAggregateMessageOrigin
            | { Here: any }
            | { Parent: any }
            | { Sibling: any }
            | string
            | Uint8Array
        ) => Observable<PalletMessageQueueBookState>,
        [CumulusPrimitivesCoreAggregateMessageOrigin]
      > &
        QueryableStorageEntry<ApiType, [CumulusPrimitivesCoreAggregateMessageOrigin]>;
      /**
       * The map of page indices to pages.
       **/
      pages: AugmentedQuery<
        ApiType,
        (
          arg1:
            | CumulusPrimitivesCoreAggregateMessageOrigin
            | { Here: any }
            | { Parent: any }
            | { Sibling: any }
            | string
            | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<PalletMessageQueuePage>>,
        [CumulusPrimitivesCoreAggregateMessageOrigin, u32]
      > &
        QueryableStorageEntry<ApiType, [CumulusPrimitivesCoreAggregateMessageOrigin, u32]>;
      /**
       * The origin at which we should begin servicing.
       **/
      serviceHead: AugmentedQuery<
        ApiType,
        () => Observable<Option<CumulusPrimitivesCoreAggregateMessageOrigin>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    nfts: {
      /**
       * The items held by any given account; set out this way so that items owned by a single
       * account can be enumerated.
       **/
      account: AugmentedQuery<
        ApiType,
        (
          arg1: AccountId32 | string | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array,
          arg3: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<Null>>,
        [AccountId32, u32, u32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32, u32, u32]>;
      /**
       * Attributes of a collection.
       **/
      attribute: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: Option<u32> | null | Uint8Array | u32 | AnyNumber,
          arg3:
            | PalletNftsAttributeNamespace
            | { Pallet: any }
            | { CollectionOwner: any }
            | { ItemOwner: any }
            | { Account: any }
            | string
            | Uint8Array,
          arg4: Bytes | string | Uint8Array
        ) => Observable<Option<ITuple<[Bytes, PalletNftsAttributeDeposit]>>>,
        [u32, Option<u32>, PalletNftsAttributeNamespace, Bytes]
      > &
        QueryableStorageEntry<ApiType, [u32, Option<u32>, PalletNftsAttributeNamespace, Bytes]>;
      /**
       * Details of a collection.
       **/
      collection: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<PalletNftsCollectionDetails>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The collections owned by any given account; set out this way so that collections owned by
       * a single account can be enumerated.
       **/
      collectionAccount: AugmentedQuery<
        ApiType,
        (
          arg1: AccountId32 | string | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<Null>>,
        [AccountId32, u32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32, u32]>;
      /**
       * Config of a collection.
       **/
      collectionConfigOf: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<PalletNftsCollectionConfig>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * Metadata of a collection.
       **/
      collectionMetadataOf: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<PalletNftsCollectionMetadata>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The items in existence and their ownership details.
       * Stores collection roles as per account.
       **/
      collectionRoleOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: AccountId32 | string | Uint8Array
        ) => Observable<Option<u8>>,
        [u32, AccountId32]
      > &
        QueryableStorageEntry<ApiType, [u32, AccountId32]>;
      /**
       * The items in existence and their ownership details.
       **/
      item: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<PalletNftsItemDetails>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * Item attribute approvals.
       **/
      itemAttributesApprovalsOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<BTreeSet<AccountId32>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * Config of an item.
       **/
      itemConfigOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<PalletNftsItemConfig>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * Metadata of an item.
       **/
      itemMetadataOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<PalletNftsItemMetadata>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * A price of an item.
       **/
      itemPriceOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<ITuple<[u128, Option<AccountId32>]>>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * Stores the `CollectionId` that is going to be used for the next collection.
       * This gets incremented whenever a new collection is created.
       **/
      nextCollectionId: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The collection, if any, of which an account is willing to take ownership.
       **/
      ownershipAcceptance: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Option<u32>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Handles all the pending swaps.
       **/
      pendingSwapOf: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<PalletNftsPendingSwap>>,
        [u32, u32]
      > &
        QueryableStorageEntry<ApiType, [u32, u32]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    parachainInfo: {
      parachainId: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    parachainSystem: {
      /**
       * Storage field that keeps track of bandwidth used by the unincluded segment along with the
       * latest HRMP watermark. Used for limiting the acceptance of new blocks with
       * respect to relay chain constraints.
       **/
      aggregatedUnincludedSegment: AugmentedQuery<
        ApiType,
        () => Observable<Option<CumulusPalletParachainSystemUnincludedSegmentSegmentTracker>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The number of HRMP messages we observed in `on_initialize` and thus used that number for
       * announcing the weight of `on_initialize` and `on_finalize`.
       **/
      announcedHrmpMessagesPerCandidate: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A custom head data that should be returned as result of `validate_block`.
       *
       * See `Pallet::set_custom_validation_head_data` for more information.
       **/
      customValidationHeadData: AugmentedQuery<ApiType, () => Observable<Option<Bytes>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Were the validation data set to notify the relay chain?
       **/
      didSetValidationCode: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The parachain host configuration that was obtained from the relay parent.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       **/
      hostConfiguration: AugmentedQuery<
        ApiType,
        () => Observable<Option<PolkadotPrimitivesV8AbridgedHostConfiguration>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * HRMP messages that were sent in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       **/
      hrmpOutboundMessages: AugmentedQuery<
        ApiType,
        () => Observable<Vec<PolkadotCorePrimitivesOutboundHrmpMessage>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * HRMP watermark that was set in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       **/
      hrmpWatermark: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The last downward message queue chain head we have observed.
       *
       * This value is loaded before and saved after processing inbound downward messages carried
       * by the system inherent.
       **/
      lastDmqMqcHead: AugmentedQuery<ApiType, () => Observable<H256>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The message queue chain heads we have observed per each channel incoming channel.
       *
       * This value is loaded before and saved after processing inbound downward messages carried
       * by the system inherent.
       **/
      lastHrmpMqcHeads: AugmentedQuery<ApiType, () => Observable<BTreeMap<u32, H256>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The relay chain block number associated with the last parachain block.
       *
       * This is updated in `on_finalize`.
       **/
      lastRelayChainBlockNumber: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Validation code that is set by the parachain and is to be communicated to collator and
       * consequently the relay-chain.
       *
       * This will be cleared in `on_initialize` of each new block if no other pallet already set
       * the value.
       **/
      newValidationCode: AugmentedQuery<ApiType, () => Observable<Option<Bytes>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Upward messages that are still pending and not yet send to the relay chain.
       **/
      pendingUpwardMessages: AugmentedQuery<ApiType, () => Observable<Vec<Bytes>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * In case of a scheduled upgrade, this storage field contains the validation code to be
       * applied.
       *
       * As soon as the relay chain gives us the go-ahead signal, we will overwrite the
       * [`:code`][sp_core::storage::well_known_keys::CODE] which will result the next block process
       * with the new validation code. This concludes the upgrade process.
       **/
      pendingValidationCode: AugmentedQuery<ApiType, () => Observable<Bytes>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Number of downward messages processed in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       **/
      processedDownwardMessages: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The state proof for the last relay parent block.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       **/
      relayStateProof: AugmentedQuery<ApiType, () => Observable<Option<SpTrieStorageProof>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The snapshot of some state related to messaging relevant to the current parachain as per
       * the relay parent.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       **/
      relevantMessagingState: AugmentedQuery<
        ApiType,
        () => Observable<
          Option<CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot>
        >,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The weight we reserve at the beginning of the block for processing DMP messages. This
       * overrides the amount set in the Config trait.
       **/
      reservedDmpWeightOverride: AugmentedQuery<
        ApiType,
        () => Observable<Option<SpWeightsWeightV2Weight>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The weight we reserve at the beginning of the block for processing XCMP messages. This
       * overrides the amount set in the Config trait.
       **/
      reservedXcmpWeightOverride: AugmentedQuery<
        ApiType,
        () => Observable<Option<SpWeightsWeightV2Weight>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Latest included block descendants the runtime accepted. In other words, these are
       * ancestors of the currently executing block which have not been included in the observed
       * relay-chain state.
       *
       * The segment length is limited by the capacity returned from the [`ConsensusHook`] configured
       * in the pallet.
       **/
      unincludedSegment: AugmentedQuery<
        ApiType,
        () => Observable<Vec<CumulusPalletParachainSystemUnincludedSegmentAncestor>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Optional upgrade go-ahead signal from the relay-chain.
       *
       * This storage item is a mirror of the corresponding value for the current parachain from the
       * relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is
       * set after the inherent.
       **/
      upgradeGoAhead: AugmentedQuery<
        ApiType,
        () => Observable<Option<PolkadotPrimitivesV8UpgradeGoAhead>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * An option which indicates if the relay-chain restricts signalling a validation code upgrade.
       * In other words, if this is `Some` and [`NewValidationCode`] is `Some` then the produced
       * candidate will be invalid.
       *
       * This storage item is a mirror of the corresponding value for the current parachain from the
       * relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is
       * set after the inherent.
       **/
      upgradeRestrictionSignal: AugmentedQuery<
        ApiType,
        () => Observable<Option<PolkadotPrimitivesV8UpgradeRestriction>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The factor to multiply the base delivery fee by for UMP.
       **/
      upwardDeliveryFeeFactor: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Upward messages that were sent in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       **/
      upwardMessages: AugmentedQuery<ApiType, () => Observable<Vec<Bytes>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The [`PersistedValidationData`] set for this block.
       * This value is expected to be set only once per block and it's never stored
       * in the trie.
       **/
      validationData: AugmentedQuery<
        ApiType,
        () => Observable<Option<PolkadotPrimitivesV8PersistedValidationData>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    parameters: {
      /**
       * Stored parameters.
       **/
      parameters: AugmentedQuery<
        ApiType,
        (
          arg:
            | ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey
            | { RuntimeConfig: any }
            | string
            | Uint8Array
        ) => Observable<Option<ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue>>,
        [ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey]
      > &
        QueryableStorageEntry<
          ApiType,
          [ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey]
        >;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    paymentStreams: {
      /**
       * The accumulated price index since genesis, used to calculate the amount to charge for dynamic-rate payment streams.
       *
       * This is equivalent to what it would have cost to provide one unit of the provided service since the beginning of the network.
       * We use this to calculate the amount to charge for dynamic-rate payment streams, by checking out the difference between the index
       * when the payment stream was last charged, and the index at the last chargeable tick.
       *
       * This storage is updated in:
       * - [do_update_price_index](crate::utils::do_update_price_index), which updates the accumulated price index, adding to it the current price.
       **/
      accumulatedPriceIndex: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The current price per gigaunit per tick of the provided service, used to calculate the amount to charge for dynamic-rate payment streams.
       *
       * This can be updated each tick by the system manager.
       *
       * It is in giga-units to allow for a more granular price per unit considering the limitations in decimal places that the Balance type might have.
       **/
      currentPricePerGigaUnitPerTick: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The double mapping from a Provider, to its provided Users, to their dynamic-rate payment streams.
       *
       * This is used to store and manage dynamic-rate payment streams between Users and Providers.
       *
       * This storage is updated in:
       * - [create_dynamic_rate_payment_stream](crate::dispatchables::create_dynamic_rate_payment_stream), which adds a new entry to the map.
       * - [delete_dynamic_rate_payment_stream](crate::dispatchables::delete_dynamic_rate_payment_stream), which removes the corresponding entry from the map.
       * - [update_dynamic_rate_payment_stream](crate::dispatchables::update_dynamic_rate_payment_stream), which updates the entry's `amount_provided`.
       * - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which updates the entry's `price_index_when_last_charged`.
       **/
      dynamicRatePaymentStreams: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: AccountId32 | string | Uint8Array
        ) => Observable<Option<PalletPaymentStreamsDynamicRatePaymentStream>>,
        [H256, AccountId32]
      > &
        QueryableStorageEntry<ApiType, [H256, AccountId32]>;
      /**
       * The double mapping from a Provider, to its provided Users, to their fixed-rate payment streams.
       *
       * This is used to store and manage fixed-rate payment streams between Users and Providers.
       *
       * This storage is updated in:
       * - [create_fixed_rate_payment_stream](crate::dispatchables::create_fixed_rate_payment_stream), which adds a new entry to the map.
       * - [delete_fixed_rate_payment_stream](crate::dispatchables::delete_fixed_rate_payment_stream), which removes the corresponding entry from the map.
       * - [update_fixed_rate_payment_stream](crate::dispatchables::update_fixed_rate_payment_stream), which updates the entry's `rate`.
       * - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which updates the entry's `last_charged_tick`.
       **/
      fixedRatePaymentStreams: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: AccountId32 | string | Uint8Array
        ) => Observable<Option<PalletPaymentStreamsFixedRatePaymentStream>>,
        [H256, AccountId32]
      > &
        QueryableStorageEntry<ApiType, [H256, AccountId32]>;
      /**
       * The mapping from a Provider to its last chargeable price index (for dynamic-rate payment streams) and last chargeable tick (for fixed-rate payment streams).
       *
       * This is used to keep track of the last chargeable price index and tick number for each Provider, so this pallet can charge the payment streams correctly.
       *
       * This storage is updated in:
       * - [update_last_chargeable_info](crate::PaymentManager::update_last_chargeable_info), which updates the entry's `last_chargeable_tick` and `price_index`.
       **/
      lastChargeableInfo: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<PalletPaymentStreamsProviderLastChargeableInfo>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The last tick that was processed by this pallet from the Proof Submitters interface.
       *
       * This is used to keep track of the last tick processed by this pallet from the pallet that implements the from the ProvidersProofSubmitters interface.
       * This is done to know the last tick for which this pallet has registered the Providers that submitted a valid proof and updated their last chargeable info.
       * In the next `on_poll` hook execution, this pallet will update the last chargeable info of the Providers that submitted a valid proof in the tick that
       * follows the one saved in this storage element.
       **/
      lastSubmittersTickRegistered: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A counter of blocks for which Providers can charge their streams.
       *
       * This counter is not necessarily the same as the block number, as the last chargeable info of Providers
       * (and the global price index) are updated in the `on_poll` hook, which happens at the beginning of every block,
       * so long as the block is not part of a [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
       * During MBMs, the block number increases, but `OnPollTicker` does not.
       **/
      onPollTicker: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Mapping of Privileged Providers.
       *
       * Privileged Providers are those who are allowed to charge up to the current tick in
       * fixed rate payment streams, regardless of their [`LastChargeableInfo`].
       **/
      privilegedProviders: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<Null>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The mapping from a user to if it has been registered to the network and the amount of payment streams it has.
       *
       * Since users have to provide a deposit to be able to open each payment stream, this is used to keep track of the amount of payment streams
       * that a user has and it is also useful to check if a user has registered to the network.
       *
       * This storage is updated in:
       * - [create_fixed_rate_payment_stream](crate::dispatchables::create_fixed_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
       * - [create_dynamic_rate_payment_stream](crate::dispatchables::create_dynamic_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
       * - [remove_fixed_rate_payment_stream](crate::dispatchables::remove_fixed_rate_payment_stream), which removes one from this storage and releases the deposit.
       * - [remove_dynamic_rate_payment_stream](crate::dispatchables::remove_dynamic_rate_payment_stream), which removes one from this storage and releases the deposit.
       **/
      registeredUsers: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<u32>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The mapping from a user to if it has been flagged for not having enough funds to pay for its requested services.
       *
       * This is used to flag users that do not have enough funds to pay for their requested services, so other Providers
       * can stop providing services to them.
       *
       * This storage is updated in:
       * - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which emits a `UserWithoutFunds` event and sets the user's entry in this map
       * to that moment's tick number if it does not have enough funds.
       * - [clear_insolvent_flag](crate::utils::clear_insolvent_flag), which clears the user's entry in this map if the cooldown period has passed and the user has paid all its outstanding debt.
       **/
      usersWithoutFunds: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Option<u32>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    polkadotXcm: {
      /**
       * The existing asset traps.
       *
       * Key is the blake2 256 hash of (origin, versioned `Assets`) pair. Value is the number of
       * times this pair has been trapped (usually just 1 if it exists at all).
       **/
      assetTraps: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<u32>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The current migration's stage, if any.
       **/
      currentMigration: AugmentedQuery<
        ApiType,
        () => Observable<Option<PalletXcmVersionMigrationStage>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Fungible assets which we know are locked on this chain.
       **/
      lockedFungibles: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array
        ) => Observable<Option<Vec<ITuple<[u128, XcmVersionedLocation]>>>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The ongoing queries.
       **/
      queries: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Option<PalletXcmQueryStatus>>,
        [u64]
      > &
        QueryableStorageEntry<ApiType, [u64]>;
      /**
       * The latest available query index.
       **/
      queryCounter: AugmentedQuery<ApiType, () => Observable<u64>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * If [`ShouldRecordXcm`] is set to true, then the last XCM program executed locally
       * will be stored here.
       * Runtime APIs can fetch the XCM that was executed by accessing this value.
       *
       * Only relevant if this pallet is being used as the [`xcm_executor::traits::RecordXcm`]
       * implementation in the XCM executor configuration.
       **/
      recordedXcm: AugmentedQuery<
        ApiType,
        () => Observable<Option<Vec<StagingXcmV5Instruction>>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Fungible assets which we know are locked on a remote chain.
       **/
      remoteLockedFungibles: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: AccountId32 | string | Uint8Array,
          arg3: XcmVersionedAssetId | { V3: any } | { V4: any } | { V5: any } | string | Uint8Array
        ) => Observable<Option<PalletXcmRemoteLockedFungibleRecord>>,
        [u32, AccountId32, XcmVersionedAssetId]
      > &
        QueryableStorageEntry<ApiType, [u32, AccountId32, XcmVersionedAssetId]>;
      /**
       * Default version to encode XCM when latest version of destination is unknown. If `None`,
       * then the destinations whose XCM version is unknown are considered unreachable.
       **/
      safeXcmVersion: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Whether or not incoming XCMs (both executed locally and received) should be recorded.
       * Only one XCM program will be recorded at a time.
       * This is meant to be used in runtime APIs, and it's advised it stays false
       * for all other use cases, so as to not degrade regular performance.
       *
       * Only relevant if this pallet is being used as the [`xcm_executor::traits::RecordXcm`]
       * implementation in the XCM executor configuration.
       **/
      shouldRecordXcm: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The Latest versions that we know various locations support.
       **/
      supportedVersion: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: XcmVersionedLocation | { V3: any } | { V4: any } | { V5: any } | string | Uint8Array
        ) => Observable<Option<u32>>,
        [u32, XcmVersionedLocation]
      > &
        QueryableStorageEntry<ApiType, [u32, XcmVersionedLocation]>;
      /**
       * Destinations whose latest XCM version we would like to know. Duplicates not allowed, and
       * the `u32` counter is the number of times that a send to the destination has been attempted,
       * which is used as a prioritization.
       **/
      versionDiscoveryQueue: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ITuple<[XcmVersionedLocation, u32]>>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * All locations that we have requested version notifications from.
       **/
      versionNotifiers: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: XcmVersionedLocation | { V3: any } | { V4: any } | { V5: any } | string | Uint8Array
        ) => Observable<Option<u64>>,
        [u32, XcmVersionedLocation]
      > &
        QueryableStorageEntry<ApiType, [u32, XcmVersionedLocation]>;
      /**
       * The target locations that are subscribed to our version changes, as well as the most recent
       * of our versions we informed them of.
       **/
      versionNotifyTargets: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: XcmVersionedLocation | { V3: any } | { V4: any } | { V5: any } | string | Uint8Array
        ) => Observable<Option<ITuple<[u64, SpWeightsWeightV2Weight, u32]>>>,
        [u32, XcmVersionedLocation]
      > &
        QueryableStorageEntry<ApiType, [u32, XcmVersionedLocation]>;
      /**
       * Global suspension state of the XCM executor.
       **/
      xcmExecutionSuspended: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    proofsDealer: {
      /**
       * A queue of keys that have been challenged manually.
       *
       * The elements in this queue will be challenged in the coming blocks,
       * always ensuring that the maximum number of challenges per block is not exceeded.
       * A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
       * is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
       **/
      challengesQueue: AugmentedQuery<ApiType, () => Observable<Vec<H256>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A counter of blocks in which challenges were distributed.
       *
       * This counter is not necessarily the same as the block number, as challenges are
       * distributed in the `on_poll` hook, which happens at the beginning of every block,
       * so long as the block is not part of a [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
       * During MBMsm, the block number increases, but [`ChallengesTicker`] does not.
       **/
      challengesTicker: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A boolean that represents whether the [`ChallengesTicker`] is paused.
       *
       * By default, this is `false`, meaning that the [`ChallengesTicker`] is incremented every time `on_poll` is called.
       * This can be set to `true` which would pause the [`ChallengesTicker`], preventing `do_new_challenges_round` from
       * being executed. Therefore:
       * - No new random challenges would be emitted and added to [`TickToChallengesSeed`].
       * - No new checkpoint challenges would be emitted and added to [`TickToCheckpointChallenges`].
       * - Deadlines for proof submissions are indefinitely postponed.
       **/
      challengesTickerPaused: AugmentedQuery<ApiType, () => Observable<Option<Null>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The challenge tick of the last checkpoint challenge round.
       *
       * This is used to determine when to include the challenges from the [`ChallengesQueue`] and
       * [`PriorityChallengesQueue`] in the [`TickToCheckpointChallenges`] StorageMap. These checkpoint
       * challenge rounds have to be answered by ALL Providers, and this is enforced by the
       * `submit_proof` extrinsic.
       **/
      lastCheckpointTick: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A value that represents the last tick that was deleted from the [`ValidProofSubmittersLastTicks`] StorageMap.
       *
       * This is used to know which tick to delete from the [`ValidProofSubmittersLastTicks`] StorageMap when the
       * `on_idle` hook is called.
       **/
      lastDeletedTick: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The vector holding whether the last [`Config::BlockFullnessPeriod`] blocks were full or not.
       *
       * Each element in the vector represents a block, and is `true` if the block was full, and `false` otherwise.
       * Note: Ideally we would use a `BitVec` to reduce storage, but since there's no bounded `BitVec` implementation
       * we use a BoundedVec<bool> instead. This uses 7 more bits of storage per element.
       **/
      pastBlocksStatus: AugmentedQuery<ApiType, () => Observable<Vec<bool>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A mapping from block number to the weight used in that block.
       *
       * This is used to check if the network is presumably under a spam attack.
       * It is cleared for blocks older than `current_block` - ([`Config::BlockFullnessPeriod`] + 1).
       **/
      pastBlocksWeight: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<SpWeightsWeightV2Weight>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * A priority queue of keys that have been challenged manually.
       *
       * The difference between this and `ChallengesQueue` is that the challenges
       * in this queue are given priority over the others. So this queue should be
       * emptied before any of the challenges in the `ChallengesQueue` are dispatched.
       * This queue should not be accessible to the public.
       * The elements in this queue will be challenged in the coming blocks,
       * always ensuring that the maximum number of challenges per block is not exceeded.
       * A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
       * is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
       **/
      priorityChallengesQueue: AugmentedQuery<
        ApiType,
        () => Observable<Vec<PalletProofsDealerCustomChallenge>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A mapping from a Provider to its [`ProofSubmissionRecord`], which stores the last tick
       * the Provider submitted a proof for, and the next tick the Provider should submit a proof for.
       *
       * Normally the difference between these two ticks is equal to the Provider's challenge period,
       * but if the Provider's period is changed, this change only affects the next cycle. In other words,
       * for one cycle, `next_tick_to_submit_proof_for - last_tick_proven ≠ provider_challenge_period`.
       *
       * If a Provider submits a proof successfully, both fields are updated.
       *
       * If the Provider fails to submit a proof in time and is slashed, only `next_tick_to_submit_proof_for`
       * is updated.
       **/
      providerToProofSubmissionRecord: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletProofsDealerProofSubmissionRecord>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      slashableProviders: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<u32>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * A mapping from challenges tick to a random seed used for generating the challenges in that tick.
       *
       * This is used to keep track of the challenges' seed in the past.
       * This mapping goes back only [`ChallengeHistoryLengthFor`] blocks. Previous challenges are removed.
       **/
      tickToChallengesSeed: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<H256>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The tick to check and see if Providers failed to submit proofs before their deadline.
       *
       * In a normal situation, this should always be equal to [`ChallengesTicker`].
       * However, in the unlikely scenario where a large number of Providers fail to submit proofs (larger
       * than [`Config::MaxSlashableProvidersPerTick`]), and all of them had the same deadline, not all of
       * them will be marked as slashable. Only the first [`Config::MaxSlashableProvidersPerTick`] will be.
       * In that case, this stored tick will lag behind [`ChallengesTicker`].
       *
       * It is expected that this tick should catch up to [`ChallengesTicker`], as blocks with less
       * slashable Providers follow.
       **/
      tickToCheckForSlashableProviders: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A mapping from challenges tick to a vector of custom challenged keys for that tick.
       *
       * This is used to keep track of the challenges that have been made in the past, specifically
       * in the checkpoint challenge rounds.
       * The vector is bounded by [`MaxCustomChallengesPerBlockFor`].
       * This mapping goes back only [`ChallengeHistoryLengthFor`] ticks. Previous challenges are removed.
       **/
      tickToCheckpointChallenges: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array
        ) => Observable<Option<Vec<PalletProofsDealerCustomChallenge>>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * A mapping from challenge tick to a vector of challenged Providers for that tick.
       *
       * This is used to keep track of the Providers that have been challenged, and should
       * submit a proof by the time of the [`ChallengesTicker`] reaches the number used as
       * key in the mapping. Providers who do submit a proof are removed from their respective
       * entry and pushed forward to the next tick in which they should submit a proof.
       * Those who are still in the entry by the time the tick is reached are considered to
       * have failed to submit a proof and subject to slashing.
       **/
      tickToProvidersDeadlines: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<Null>>,
        [u32, H256]
      > &
        QueryableStorageEntry<ApiType, [u32, H256]>;
      /**
       * A mapping from tick to Providers, which is set if the Provider submitted a valid proof in that tick.
       *
       * This is used to keep track of the Providers that have submitted proofs in the last few
       * ticks, where availability only up to the last [`Config::TargetTicksStorageOfSubmitters`] ticks is guaranteed.
       * This storage is then made available for other pallets to use through the `ProofSubmittersInterface`.
       **/
      validProofSubmittersLastTicks: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<BTreeSet<H256>>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    providers: {
      /**
       * The mapping from an AccountId to a BackupStorageProviderId.
       *
       * This is used to get a Backup Storage Provider's unique identifier needed to access its metadata.
       *
       * This storage is updated in:
       *
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
       * - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which removes the corresponding entry from the map.
       **/
      accountIdToBackupStorageProviderId: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Option<H256>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The mapping from an AccountId to a MainStorageProviderId.
       *
       * This is used to get a Main Storage Provider's unique identifier needed to access its metadata.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Main Storage Provider.
       * - [msp_sign_off](crate::dispatchables::msp_sign_off), which removes the corresponding entry from the map.
       **/
      accountIdToMainStorageProviderId: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Option<H256>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Storage providers currently awaited for to top up their deposit (providers whom have been slashed and as
       * a result have a capacity deficit, i.e. their capacity is below their used capacity).
       *
       * This is primarily used to lookup providers and restrict certain operations while they are in this state.
       *
       * Providers can optionally call the `top_up_deposit` during the grace period to top up their held deposit to cover the capacity deficit.
       * As a result, their provider account would be cleared from this storage.
       *
       * The `on_idle` hook will process every provider in this storage and mark them as insolvent.
       * If a provider is marked as insolvent, the network (e.g users, other providers) can call `issue_storage_request`
       * with a replication target of 1 to fill a slot with another BSP if the provider who was marked as insolvent is in fact a BSP.
       * If it was an MSP, the user can decide to move their buckets to another MSP or delete their buckets (as they normally can).
       **/
      awaitingTopUpFromProviders: AugmentedQuery<
        ApiType,
        (
          arg:
            | PalletStorageProvidersStorageProviderId
            | { BackupStorageProvider: any }
            | { MainStorageProvider: any }
            | string
            | Uint8Array
        ) => Observable<Option<PalletStorageProvidersTopUpMetadata>>,
        [PalletStorageProvidersStorageProviderId]
      > &
        QueryableStorageEntry<ApiType, [PalletStorageProvidersStorageProviderId]>;
      /**
       * The mapping from a BackupStorageProviderId to a BackupStorageProvider.
       *
       * This is used to get a Backup Storage Provider's metadata.
       * It returns `None` if the Backup Storage Provider ID does not correspond to any registered Backup Storage Provider.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
       * - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which removes the corresponding entry from the map.
       * - [change_capacity](crate::dispatchables::change_capacity), which changes the entry's `capacity`.
       **/
      backupStorageProviders: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletStorageProvidersBackupStorageProvider>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The amount of Backup Storage Providers that are currently registered in the runtime.
       *
       * This is used to keep track of the total amount of BSPs in the system.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds one to this storage if the account to confirm is a Backup Storage Provider.
       * - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which subtracts one from this storage.
       **/
      bspCount: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The mapping from a BucketId to that bucket's metadata.
       *
       * This is used to get a bucket's metadata, such as root, user ID, and MSP ID.
       * It returns `None` if the Bucket ID does not correspond to any registered bucket.
       *
       * This storage is updated in:
       * - [add_bucket](shp_traits::MutateProvidersInterface::add_bucket), which adds a new entry to the map.
       * - [change_root_bucket](shp_traits::MutateProvidersInterface::change_root_bucket), which changes the corresponding bucket's root.
       * - [delete_bucket](shp_traits::MutateProvidersInterface::delete_bucket), which removes the entry of the corresponding bucket.
       **/
      buckets: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<PalletStorageProvidersBucket>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The total global reputation weight of all BSPs.
       **/
      globalBspsReputationWeight: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A map of insolvent providers who have failed to top up their deposit before the end of the expiration.
       *
       * Providers are marked insolvent by the `on_idle` hook.
       **/
      insolventProviders: AugmentedQuery<
        ApiType,
        (
          arg:
            | PalletStorageProvidersStorageProviderId
            | { BackupStorageProvider: any }
            | { MainStorageProvider: any }
            | string
            | Uint8Array
        ) => Observable<Option<Null>>,
        [PalletStorageProvidersStorageProviderId]
      > &
        QueryableStorageEntry<ApiType, [PalletStorageProvidersStorageProviderId]>;
      /**
       * The double mapping from a MainStorageProviderId to a BucketIds.
       *
       * This is used to efficiently retrieve the list of buckets that a Main Storage Provider is currently storing.
       *
       * This storage is updated in:
       * - [add_bucket](shp_traits::MutateProvidersInterface::add_bucket)
       * - [delete_bucket](shp_traits::MutateProvidersInterface::delete_bucket)
       **/
      mainStorageProviderIdsToBuckets: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<Null>>,
        [H256, H256]
      > &
        QueryableStorageEntry<ApiType, [H256, H256]>;
      /**
       * Double mapping from a [`MainStorageProviderId`] to [`ValueProposition`]s.
       *
       * These are applied at the bucket level. Propositions are the price per [`Config::StorageDataUnit`] per block and the
       * limit of data that can be stored in the bucket.
       **/
      mainStorageProviderIdsToValuePropositions: AugmentedQuery<
        ApiType,
        (
          arg1: H256 | string | Uint8Array,
          arg2: H256 | string | Uint8Array
        ) => Observable<Option<PalletStorageProvidersValueProposition>>,
        [H256, H256]
      > &
        QueryableStorageEntry<ApiType, [H256, H256]>;
      /**
       * The mapping from a MainStorageProviderId to a MainStorageProvider.
       *
       * This is used to get a Main Storage Provider's metadata.
       * It returns `None` if the Main Storage Provider ID does not correspond to any registered Main Storage Provider.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Main Storage Provider.
       * - [msp_sign_off](crate::dispatchables::msp_sign_off), which removes the corresponding entry from the map.
       * - [change_capacity](crate::dispatchables::change_capacity), which changes the entry's `capacity`.
       **/
      mainStorageProviders: AugmentedQuery<
        ApiType,
        (
          arg: H256 | string | Uint8Array
        ) => Observable<Option<PalletStorageProvidersMainStorageProvider>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The amount of Main Storage Providers that are currently registered in the runtime.
       *
       * This is used to keep track of the total amount of MSPs in the system.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds one to this storage if the account to confirm is a Main Storage Provider.
       * - [msp_sign_off](crate::dispatchables::msp_sign_off), which subtracts one from this storage.
       **/
      mspCount: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A pointer to the earliest available Storage Hub tick to insert a new provider top up expiration item.
       *
       * This should always be greater or equal than `current_sh_tick` + [`Config::ProviderTopUpTtl`].
       **/
      nextAvailableProviderTopUpExpirationShTick: AugmentedQuery<
        ApiType,
        () => Observable<u32>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A pointer to the starting Storage Hub tick number to clean up expired items.
       *
       * If this Storage Hub tick is behind the one, the cleanup algorithm in `on_idle` will
       * attempt to advance this tick pointer as close to or up to the current one. This
       * will execute provided that there is enough remaining weight to do so.
       **/
      nextStartingShTickToCleanUp: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * A map of Storage Hub tick numbers to expired provider top up expired items.
       *
       * Processed in the `on_idle` hook.
       *
       * Provider top up expiration items are ignored and cleared if the provider is not found in the [`AwaitingTopUpFromProviders`] storage.
       * Providers are removed from [`AwaitingTopUpFromProviders`] storage when they have successfully topped up their deposit.
       * If they are still part of the [`AwaitingTopUpFromProviders`] storage after the expiration period, they are marked as insolvent.
       **/
      providerTopUpExpirations: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array
        ) => Observable<Vec<PalletStorageProvidersStorageProviderId>>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The mapping from an AccountId that requested to sign up to a tuple of the metadata with type of the request, and the block
       * number when the request was made.
       *
       * This is used for the two-step process of registering: when a user requests to register as a SP (either MSP or BSP),
       * that request with the metadata and the deposit held is stored here. When the user confirms the sign up, the
       * request is removed from this storage and the user is registered as a SP.
       *
       * This storage is updated in:
       * - [request_msp_sign_up](crate::dispatchables::request_msp_sign_up) and [request_bsp_sign_up](crate::dispatchables::request_bsp_sign_up), which add a new entry to the map.
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up) and [cancel_sign_up](crate::dispatchables::cancel_sign_up), which remove an existing entry from the map.
       **/
      signUpRequests: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array
        ) => Observable<Option<PalletStorageProvidersSignUpRequest>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * The total amount of storage capacity all BSPs have.
       *
       * This is used to keep track of the total amount of storage capacity all BSPs have in the system, which is also the
       * total amount of storage capacity that can be used by users if we factor in the replication factor.
       *
       * This storage is updated in:
       * - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds the capacity of the registered Storage Provider to this storage if the account to confirm is a Backup Storage Provider.
       * - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which subtracts the capacity of the Backup Storage Provider to sign off from this storage.
       **/
      totalBspsCapacity: AugmentedQuery<ApiType, () => Observable<u64>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The total amount of storage capacity of BSPs that is currently in use.
       *
       * This is used to keep track of the total amount of storage capacity that is currently in use by users, which is useful for
       * system metrics and also to calculate the current price of storage.
       **/
      usedBspsCapacity: AugmentedQuery<ApiType, () => Observable<u64>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    randomness: {
      /**
       * Ensures the mandatory inherent was included in the block
       **/
      inherentIncluded: AugmentedQuery<ApiType, () => Observable<Option<Null>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The relay chain block (and anchored parachain block) to use when epoch changes
       **/
      lastRelayBlockAndParaBlockValidForNextEpoch: AugmentedQuery<
        ApiType,
        () => Observable<ITuple<[u32, u32]>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Latest random seed obtained from the one epoch ago randomness from BABE, and the latest block that it can process randomness requests from
       **/
      latestOneEpochAgoRandomness: AugmentedQuery<
        ApiType,
        () => Observable<Option<ITuple<[H256, u32]>>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Latest random seed obtained from the parent block randomness from BABE, and the latest block that it can process randomness requests from
       **/
      latestParentBlockRandomness: AugmentedQuery<
        ApiType,
        () => Observable<Option<ITuple<[H256, u32]>>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Current relay epoch
       **/
      relayEpoch: AugmentedQuery<ApiType, () => Observable<u64>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    session: {
      /**
       * Current index of the session.
       **/
      currentIndex: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Indices of disabled validators.
       *
       * The vec is always kept sorted so that we can find whether a given validator is
       * disabled using binary search. It gets cleared when `on_session_ending` returns
       * a new set of identities.
       **/
      disabledValidators: AugmentedQuery<ApiType, () => Observable<Vec<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The owner of a key. The key is the `KeyTypeId` + the encoded key.
       **/
      keyOwner: AugmentedQuery<
        ApiType,
        (
          arg:
            | ITuple<[SpCoreCryptoKeyTypeId, Bytes]>
            | [SpCoreCryptoKeyTypeId | string | Uint8Array, Bytes | string | Uint8Array]
        ) => Observable<Option<AccountId32>>,
        [ITuple<[SpCoreCryptoKeyTypeId, Bytes]>]
      > &
        QueryableStorageEntry<ApiType, [ITuple<[SpCoreCryptoKeyTypeId, Bytes]>]>;
      /**
       * The next session keys for a validator.
       **/
      nextKeys: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array
        ) => Observable<Option<ShParachainRuntimeSessionKeys>>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * True if the underlying economic identities or weighting behind the validators
       * has changed in the queued validator set.
       **/
      queuedChanged: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The queued keys for the next session. When the next session begins, these keys
       * will be used to determine the validator's session keys.
       **/
      queuedKeys: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ITuple<[AccountId32, ShParachainRuntimeSessionKeys]>>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The current set of validators.
       **/
      validators: AugmentedQuery<ApiType, () => Observable<Vec<AccountId32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    sudo: {
      /**
       * The `AccountId` of the sudo key.
       **/
      key: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    system: {
      /**
       * The full account information for a particular account ID.
       **/
      account: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<FrameSystemAccountInfo>,
        [AccountId32]
      > &
        QueryableStorageEntry<ApiType, [AccountId32]>;
      /**
       * Total length (in bytes) for all extrinsics put together, for the current block.
       **/
      allExtrinsicsLen: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * `Some` if a code upgrade has been authorized.
       **/
      authorizedUpgrade: AugmentedQuery<
        ApiType,
        () => Observable<Option<FrameSystemCodeUpgradeAuthorization>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Map of block numbers to block hashes.
       **/
      blockHash: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<H256>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The current weight for the block.
       **/
      blockWeight: AugmentedQuery<
        ApiType,
        () => Observable<FrameSupportDispatchPerDispatchClassWeight>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Digest of the current block, also part of the block header.
       **/
      digest: AugmentedQuery<ApiType, () => Observable<SpRuntimeDigest>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The number of events in the `Events<T>` list.
       **/
      eventCount: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Events deposited for the current block.
       *
       * NOTE: The item is unbound and should therefore never be read on chain.
       * It could otherwise inflate the PoV size of a block.
       *
       * Events have a large in-memory size. Box the events to not go out-of-memory
       * just in case someone still reads them from within the runtime.
       **/
      events: AugmentedQuery<ApiType, () => Observable<Vec<FrameSystemEventRecord>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Mapping between a topic (represented by T::Hash) and a vector of indexes
       * of events in the `<Events<T>>` list.
       *
       * All topic vectors have deterministic storage locations depending on the topic. This
       * allows light-clients to leverage the changes trie storage tracking mechanism and
       * in case of changes fetch the list of events of interest.
       *
       * The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just
       * the `EventIndex` then in case if the topic has the same contents on the next block
       * no notification will be triggered thus the event might be lost.
       **/
      eventTopics: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Vec<ITuple<[u32, u32]>>>,
        [H256]
      > &
        QueryableStorageEntry<ApiType, [H256]>;
      /**
       * The execution phase of the block.
       **/
      executionPhase: AugmentedQuery<ApiType, () => Observable<Option<FrameSystemPhase>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Total extrinsics count for the current block.
       **/
      extrinsicCount: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Extrinsics data for the current block (maps an extrinsic's index to its data).
       **/
      extrinsicData: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Bytes>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * Whether all inherents have been applied.
       **/
      inherentsApplied: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened.
       **/
      lastRuntimeUpgrade: AugmentedQuery<
        ApiType,
        () => Observable<Option<FrameSystemLastRuntimeUpgradeInfo>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The current block number being processed. Set by `execute_block`.
       **/
      number: AugmentedQuery<ApiType, () => Observable<u32>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Hash of the previous block.
       **/
      parentHash: AugmentedQuery<ApiType, () => Observable<H256>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * True if we have upgraded so that AccountInfo contains three types of `RefCount`. False
       * (default) if not.
       **/
      upgradedToTripleRefCount: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * True if we have upgraded so that `type RefCount` is `u32`. False (default) if not.
       **/
      upgradedToU32RefCount: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    timestamp: {
      /**
       * Whether the timestamp has been updated in this block.
       *
       * This value is updated to `true` upon successful submission of a timestamp by a node.
       * It is then checked at the end of each block execution in the `on_finalize` hook.
       **/
      didUpdate: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The current time for the current block.
       **/
      now: AugmentedQuery<ApiType, () => Observable<u64>, []> & QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    transactionPayment: {
      nextFeeMultiplier: AugmentedQuery<ApiType, () => Observable<u128>, []> &
        QueryableStorageEntry<ApiType, []>;
      storageVersion: AugmentedQuery<
        ApiType,
        () => Observable<PalletTransactionPaymentReleases>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
    xcmpQueue: {
      /**
       * The factor to multiply the base delivery fee by.
       **/
      deliveryFeeFactor: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<u128>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * The suspended inbound XCMP channels. All others are not suspended.
       *
       * This is a `StorageValue` instead of a `StorageMap` since we expect multiple reads per block
       * to different keys with a one byte payload. The access to `BoundedBTreeSet` will be cached
       * within the block and therefore only included once in the proof size.
       *
       * NOTE: The PoV benchmarking cannot know this and will over-estimate, but the actual proof
       * will be smaller.
       **/
      inboundXcmpSuspended: AugmentedQuery<ApiType, () => Observable<BTreeSet<u32>>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The messages outbound in a given XCMP channel.
       **/
      outboundXcmpMessages: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u16 | AnyNumber | Uint8Array
        ) => Observable<Bytes>,
        [u32, u16]
      > &
        QueryableStorageEntry<ApiType, [u32, u16]>;
      /**
       * The non-empty XCMP channels in order of becoming non-empty, and the index of the first
       * and last outbound message. If the two indices are equal, then it indicates an empty
       * queue and there must be a non-`Ok` `OutboundStatus`. We assume queues grow no greater
       * than 65535 items. Queue indices for normal messages begin at one; zero is reserved in
       * case of the need to send a high-priority signal message this block.
       * The bool is true if there is a signal message waiting to be sent.
       **/
      outboundXcmpStatus: AugmentedQuery<
        ApiType,
        () => Observable<Vec<CumulusPalletXcmpQueueOutboundChannelDetails>>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * The configuration which controls the dynamics of the outbound queue.
       **/
      queueConfig: AugmentedQuery<
        ApiType,
        () => Observable<CumulusPalletXcmpQueueQueueConfigData>,
        []
      > &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Whether or not the XCMP queue is suspended from executing incoming XCMs or not.
       **/
      queueSuspended: AugmentedQuery<ApiType, () => Observable<bool>, []> &
        QueryableStorageEntry<ApiType, []>;
      /**
       * Any signal messages waiting to be sent.
       **/
      signalMessages: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Bytes>,
        [u32]
      > &
        QueryableStorageEntry<ApiType, [u32]>;
      /**
       * Generic query
       **/
      [key: string]: QueryableStorageEntry<ApiType>;
    };
  } // AugmentedQueries
} // declare module
