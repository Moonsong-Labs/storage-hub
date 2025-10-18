// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/api-base/types/calls";

import type { ApiTypes, AugmentedCall, DecoratedCallBase } from "@polkadot/api-base/types";
import type {
  BTreeMap,
  Bytes,
  Null,
  Option,
  Result,
  U256,
  Vec,
  bool,
  u256,
  u32,
  u64
} from "@polkadot/types-codec";
import type { AnyNumber, IMethod, ITuple } from "@polkadot/types-codec/types";
import type {
  BabeEquivocationProof,
  BabeGenesisConfiguration,
  Epoch,
  OpaqueKeyOwnershipProof
} from "@polkadot/types/interfaces/babe";
import type { CheckInherentsResult, InherentData } from "@polkadot/types/interfaces/blockbuilder";
import type { BlockHash } from "@polkadot/types/interfaces/chain";
import type { AuthorityId } from "@polkadot/types/interfaces/consensus";
import type {
  BlockV2,
  EthReceiptV3,
  EthTransactionStatus,
  TransactionV2
} from "@polkadot/types/interfaces/eth";
import type { EvmAccount, EvmCallInfoV2, EvmCreateInfoV2 } from "@polkadot/types/interfaces/evm";
import type { Extrinsic } from "@polkadot/types/interfaces/extrinsics";
import type { GenesisBuildErr } from "@polkadot/types/interfaces/genesisBuilder";
import type {
  AuthorityList,
  GrandpaEquivocationProof,
  SetId
} from "@polkadot/types/interfaces/grandpa";
import type { OpaqueMetadata } from "@polkadot/types/interfaces/metadata";
import type { FeeDetails, RuntimeDispatchInfo } from "@polkadot/types/interfaces/payment";
import type {
  AccountId,
  Balance,
  Block,
  BlockNumber,
  Call,
  ExtrinsicInclusionMode,
  H160,
  H256,
  Header,
  Index,
  KeyTypeId,
  Permill,
  Slot,
  Weight
} from "@polkadot/types/interfaces/runtime";
import type { RuntimeVersion } from "@polkadot/types/interfaces/state";
import type { ApplyExtrinsicResult, DispatchError, Key } from "@polkadot/types/interfaces/system";
import type { TransactionSource, TransactionValidity } from "@polkadot/types/interfaces/txqueue";
import type { IExtrinsic, Observable } from "@polkadot/types/types";
import type {
  BackupStorageProvider,
  BackupStorageProviderId,
  BucketId,
  ChunkId,
  GenericApplyDeltaEventInfoError,
  GetBspInfoError,
  GetChallengePeriodError,
  GetChallengeSeedError,
  GetCheckpointChallengesError,
  GetNextDeadlineTickError,
  GetProofSubmissionRecordError,
  GetStakeError,
  GetUsersWithDebtOverThresholdError,
  IncompleteStorageRequestMetadataResponse,
  IsStorageRequestOpenToVolunteersError,
  MainStorageProviderId,
  Multiaddresses,
  ProviderId,
  QueryAvailableStorageCapacityError,
  QueryBspConfirmChunksToProveForFileError,
  QueryBspsVolunteeredForFileError,
  QueryBucketsForMspError,
  QueryBucketsOfUserStoredByMspError,
  QueryEarliestChangeCapacityBlockError,
  QueryFileEarliestVolunteerBlockError,
  QueryIncompleteStorageRequestMetadataError,
  QueryMspConfirmChunksToProveForFileError,
  QueryMspIdOfBucketIdError,
  QueryProviderMultiaddressesError,
  QueryStorageProviderCapacityError,
  RandomnessOutput,
  StorageDataUnit,
  StorageProviderId,
  StorageRequestMetadata,
  TrieRemoveMutation,
  ValuePropositionWithId
} from "@storagehub/api-augment/solochain-evm/interfaces/storagehubclient";

export type __AugmentedCall<ApiType extends ApiTypes> = AugmentedCall<ApiType>;
export type __DecoratedCallBase<ApiType extends ApiTypes> = DecoratedCallBase<ApiType>;

declare module "@polkadot/api-base/types/calls" {
  interface AugmentedCalls<ApiType extends ApiTypes> {
    /** 0xbc9d89904f5b923f/1 */
    accountNonceApi: {
      /**
       * The API to query account nonce (aka transaction index)
       **/
      accountNonce: AugmentedCall<
        ApiType,
        (accountId: AccountId | string | Uint8Array) => Observable<Index>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xcbca25e39f142387/2 */
    babeApi: {
      /**
       * Return the genesis configuration for BABE. The configuration is only read on genesis.
       **/
      configuration: AugmentedCall<ApiType, () => Observable<BabeGenesisConfiguration>>;
      /**
       * Returns information regarding the current epoch.
       **/
      currentEpoch: AugmentedCall<ApiType, () => Observable<Epoch>>;
      /**
       * Returns the slot that started the current epoch.
       **/
      currentEpochStart: AugmentedCall<ApiType, () => Observable<Slot>>;
      /**
       * Generates a proof of key ownership for the given authority in the current epoch.
       **/
      generateKeyOwnershipProof: AugmentedCall<
        ApiType,
        (
          slot: Slot | AnyNumber | Uint8Array,
          authorityId: AuthorityId | string | Uint8Array
        ) => Observable<Option<OpaqueKeyOwnershipProof>>
      >;
      /**
       * Returns information regarding the next epoch (which was already previously announced).
       **/
      nextEpoch: AugmentedCall<ApiType, () => Observable<Epoch>>;
      /**
       * Submits an unsigned extrinsic to report an equivocation.
       **/
      submitReportEquivocationUnsignedExtrinsic: AugmentedCall<
        ApiType,
        (
          equivocationProof:
            | BabeEquivocationProof
            | { offender?: any; slotNumber?: any; firstHeader?: any; secondHeader?: any }
            | string
            | Uint8Array,
          keyOwnerProof: OpaqueKeyOwnershipProof | string | Uint8Array
        ) => Observable<Option<Null>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x40fe3ad401f8959a/6 */
    blockBuilder: {
      /**
       * Apply the given extrinsic.
       **/
      applyExtrinsic: AugmentedCall<
        ApiType,
        (
          extrinsic: Extrinsic | IExtrinsic | string | Uint8Array
        ) => Observable<ApplyExtrinsicResult>
      >;
      /**
       * Check that the inherents are valid.
       **/
      checkInherents: AugmentedCall<
        ApiType,
        (
          block: Block | { header?: any; extrinsics?: any } | string | Uint8Array,
          data: InherentData | { data?: any } | string | Uint8Array
        ) => Observable<CheckInherentsResult>
      >;
      /**
       * Finish the current block.
       **/
      finalizeBlock: AugmentedCall<ApiType, () => Observable<Header>>;
      /**
       * Generate inherent extrinsics.
       **/
      inherentExtrinsics: AugmentedCall<
        ApiType,
        (
          inherent: InherentData | { data?: any } | string | Uint8Array
        ) => Observable<Vec<Extrinsic>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xe65b00e46cedd0aa/2 */
    convertTransactionRuntimeApi: {
      /**
       * Converts an Ethereum-style transaction to Extrinsic
       **/
      convertTransaction: AugmentedCall<
        ApiType,
        (
          transaction:
            | TransactionV2
            | { Legacy: any }
            | { EIP2930: any }
            | { EIP1559: any }
            | string
            | Uint8Array
        ) => Observable<Extrinsic>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xdf6acb689907609b/5 */
    core: {
      /**
       * Execute the given block.
       **/
      executeBlock: AugmentedCall<
        ApiType,
        (
          block: Block | { header?: any; extrinsics?: any } | string | Uint8Array
        ) => Observable<Null>
      >;
      /**
       * Initialize a block with the given header.
       **/
      initializeBlock: AugmentedCall<
        ApiType,
        (
          header:
            | Header
            | {
                parentHash?: any;
                number?: any;
                stateRoot?: any;
                extrinsicsRoot?: any;
                digest?: any;
              }
            | string
            | Uint8Array
        ) => Observable<ExtrinsicInclusionMode>
      >;
      /**
       * Returns the version of the runtime.
       **/
      version: AugmentedCall<ApiType, () => Observable<RuntimeVersion>>;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x582211f65bb14b89/5 */
    ethereumRuntimeRPCApi: {
      /**
       * Returns pallet_evm::Accounts by address.
       **/
      accountBasic: AugmentedCall<
        ApiType,
        (address: H160 | string | Uint8Array) => Observable<EvmAccount>
      >;
      /**
       * For a given account address, returns pallet_evm::AccountCodes.
       **/
      accountCodeAt: AugmentedCall<
        ApiType,
        (address: H160 | string | Uint8Array) => Observable<Bytes>
      >;
      /**
       * Returns the converted FindAuthor::find_author authority id.
       **/
      author: AugmentedCall<ApiType, () => Observable<H160>>;
      /**
       * Returns a frame_ethereum::call response. If `estimate` is true,
       **/
      call: AugmentedCall<
        ApiType,
        (
          from: H160 | string | Uint8Array,
          to: H160 | string | Uint8Array,
          data: Bytes | string | Uint8Array,
          value: U256 | AnyNumber | Uint8Array,
          gasLimit: U256 | AnyNumber | Uint8Array,
          maxFeePerGas: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          maxPriorityFeePerGas: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          nonce: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          estimate: bool | boolean | Uint8Array,
          accessList:
            | Option<Vec<ITuple<[H160, Vec<H256>]>>>
            | null
            | Uint8Array
            | Vec<ITuple<[H160, Vec<H256>]>>
            | [H160 | string | Uint8Array, Vec<H256> | (H256 | string | Uint8Array)[]][]
        ) => Observable<Result<EvmCallInfoV2, DispatchError>>
      >;
      /**
       * Returns runtime defined pallet_evm::ChainId.
       **/
      chainId: AugmentedCall<ApiType, () => Observable<u64>>;
      /**
       * Returns a frame_ethereum::call response. If `estimate` is true,
       **/
      create: AugmentedCall<
        ApiType,
        (
          from: H160 | string | Uint8Array,
          data: Bytes | string | Uint8Array,
          value: U256 | AnyNumber | Uint8Array,
          gasLimit: U256 | AnyNumber | Uint8Array,
          maxFeePerGas: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          maxPriorityFeePerGas: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          nonce: Option<U256> | null | Uint8Array | U256 | AnyNumber,
          estimate: bool | boolean | Uint8Array,
          accessList:
            | Option<Vec<ITuple<[H160, Vec<H256>]>>>
            | null
            | Uint8Array
            | Vec<ITuple<[H160, Vec<H256>]>>
            | [H160 | string | Uint8Array, Vec<H256> | (H256 | string | Uint8Array)[]][]
        ) => Observable<Result<EvmCreateInfoV2, DispatchError>>
      >;
      /**
       * Return all the current data for a block in a single runtime call.
       **/
      currentAll: AugmentedCall<
        ApiType,
        () => Observable<
          ITuple<[Option<BlockV2>, Option<Vec<EthReceiptV3>>, Option<Vec<EthTransactionStatus>>]>
        >
      >;
      /**
       * Return the current block.
       **/
      currentBlock: AugmentedCall<ApiType, () => Observable<BlockV2>>;
      /**
       * Return the current receipt.
       **/
      currentReceipts: AugmentedCall<ApiType, () => Observable<Option<Vec<EthReceiptV3>>>>;
      /**
       * Return the current transaction status.
       **/
      currentTransactionStatuses: AugmentedCall<
        ApiType,
        () => Observable<Option<Vec<EthTransactionStatus>>>
      >;
      /**
       * Return the elasticity multiplier.
       **/
      elasticity: AugmentedCall<ApiType, () => Observable<Option<Permill>>>;
      /**
       * Receives a `Vec<OpaqueExtrinsic>` and filters all the ethereum transactions.
       **/
      extrinsicFilter: AugmentedCall<
        ApiType,
        (
          xts: Vec<Extrinsic> | (Extrinsic | IExtrinsic | string | Uint8Array)[]
        ) => Observable<Vec<TransactionV2>>
      >;
      /**
       * Returns FixedGasPrice::min_gas_price
       **/
      gasPrice: AugmentedCall<ApiType, () => Observable<u256>>;
      /**
       * For a given account address and index, returns pallet_evm::AccountStorages.
       **/
      storageAt: AugmentedCall<
        ApiType,
        (
          address: H160 | string | Uint8Array,
          index: u256 | AnyNumber | Uint8Array
        ) => Observable<H256>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xb9e7717ace5b45cd/1 */
    fileSystemApi: {
      /**
       * Decodes the BucketId expected to be found in the event info of a generic apply delta.
       **/
      decodeGenericApplyDeltaEventInfo: AugmentedCall<
        ApiType,
        (
          encodedEventInfo: Bytes | string | Uint8Array
        ) => Observable<Result<BucketId, GenericApplyDeltaEventInfoError>>
      >;
      /**
       * Check if a storage request is open to volunteers.
       **/
      isStorageRequestOpenToVolunteers: AugmentedCall<
        ApiType,
        (
          fileKey: H256 | string | Uint8Array
        ) => Observable<Result<bool, IsStorageRequestOpenToVolunteersError>>
      >;
      /**
       * List incomplete storage request keys with pagination.
       **/
      listIncompleteStorageRequestKeys: AugmentedCall<
        ApiType,
        (
          startAfter: Option<H256> | null | Uint8Array | H256 | string,
          limit: u32 | AnyNumber | Uint8Array
        ) => Observable<Vec<H256>>
      >;
      /**
       * Get pending storage requests for a Main Storage Provider.
       **/
      pendingStorageRequestsByMsp: AugmentedCall<
        ApiType,
        (
          mspId: MainStorageProviderId | string | Uint8Array
        ) => Observable<BTreeMap<H256, StorageRequestMetadata>>
      >;
      /**
       * Query the chunks that a BSP needs to prove to confirm that it is storing a file.
       **/
      queryBspConfirmChunksToProveForFile: AugmentedCall<
        ApiType,
        (
          bspId: BackupStorageProviderId | string | Uint8Array,
          fileKey: H256 | string | Uint8Array
        ) => Observable<Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>>
      >;
      /**
       * Query the BSPs that volunteered for a file.
       **/
      queryBspsVolunteeredForFile: AugmentedCall<
        ApiType,
        (
          fileKey: H256 | string | Uint8Array
        ) => Observable<Result<Vec<BackupStorageProviderId>, QueryBspsVolunteeredForFileError>>
      >;
      /**
       * Query the earliest tick number that a BSP can volunteer for a file.
       **/
      queryEarliestFileVolunteerTick: AugmentedCall<
        ApiType,
        (
          bspId: BackupStorageProviderId | string | Uint8Array,
          fileKey: H256 | string | Uint8Array
        ) => Observable<Result<BlockNumber, QueryFileEarliestVolunteerBlockError>>
      >;
      /**
       * Query incomplete storage request metadata for a file key.
       **/
      queryIncompleteStorageRequestMetadata: AugmentedCall<
        ApiType,
        (
          fileKey: H256 | string | Uint8Array
        ) => Observable<
          Result<
            IncompleteStorageRequestMetadataResponse,
            QueryIncompleteStorageRequestMetadataError
          >
        >
      >;
      /**
       * Query the chunks that a MSP needs to prove to confirm that it is storing a file.
       **/
      queryMspConfirmChunksToProveForFile: AugmentedCall<
        ApiType,
        (
          mspId: MainStorageProviderId | string | Uint8Array,
          fileKey: H256 | string | Uint8Array
        ) => Observable<Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>>
      >;
      /**
       * Get the storage requests for a given MSP.
       **/
      storageRequestsByMsp: AugmentedCall<
        ApiType,
        (
          mspId: MainStorageProviderId | string | Uint8Array
        ) => Observable<BTreeMap<H256, StorageRequestMetadata>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xfbc577b9d747efd6/1 */
    genesisBuilder: {
      /**
       * Build `RuntimeGenesisConfig` from a JSON blob not using any defaults and store it in the storage.
       **/
      buildConfig: AugmentedCall<
        ApiType,
        (json: Bytes | string | Uint8Array) => Observable<Result<ITuple<[]>, GenesisBuildErr>>
      >;
      /**
       * Creates the default `RuntimeGenesisConfig` and returns it as a JSON blob.
       **/
      createDefaultConfig: AugmentedCall<ApiType, () => Observable<Bytes>>;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xed99c5acb25eedf5/3 */
    grandpaApi: {
      /**
       * Get current GRANDPA authority set id.
       **/
      currentSetId: AugmentedCall<ApiType, () => Observable<SetId>>;
      /**
       * Generates a proof of key ownership for the given authority in the given set.
       **/
      generateKeyOwnershipProof: AugmentedCall<
        ApiType,
        (
          setId: SetId | AnyNumber | Uint8Array,
          authorityId: AuthorityId | string | Uint8Array
        ) => Observable<Option<OpaqueKeyOwnershipProof>>
      >;
      /**
       * Get the current GRANDPA authorities and weights. This should not change except for when changes are scheduled and the corresponding delay has passed.
       **/
      grandpaAuthorities: AugmentedCall<ApiType, () => Observable<AuthorityList>>;
      /**
       * Submits an unsigned extrinsic to report an equivocation.
       **/
      submitReportEquivocationUnsignedExtrinsic: AugmentedCall<
        ApiType,
        (
          equivocationProof:
            | GrandpaEquivocationProof
            | { setId?: any; equivocation?: any }
            | string
            | Uint8Array,
          keyOwnerProof: OpaqueKeyOwnershipProof | string | Uint8Array
        ) => Observable<Option<Null>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x37e397fc7c91f5e4/2 */
    metadata: {
      /**
       * Returns the metadata of a runtime
       **/
      metadata: AugmentedCall<ApiType, () => Observable<OpaqueMetadata>>;
      /**
       * Returns the metadata at a given version.
       **/
      metadataAtVersion: AugmentedCall<
        ApiType,
        (version: u32 | AnyNumber | Uint8Array) => Observable<Option<OpaqueMetadata>>
      >;
      /**
       * Returns the supported metadata versions.
       **/
      metadataVersions: AugmentedCall<ApiType, () => Observable<Vec<u32>>>;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xf78b278be53f454c/2 */
    offchainWorkerApi: {
      /**
       * Starts the off-chain task for given block header.
       **/
      offchainWorker: AugmentedCall<
        ApiType,
        (
          header:
            | Header
            | {
                parentHash?: any;
                number?: any;
                stateRoot?: any;
                extrinsicsRoot?: any;
                digest?: any;
              }
            | string
            | Uint8Array
        ) => Observable<Null>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x1078d7ac24a07b0e/1 */
    paymentStreamsApi: {
      /**
       * Get the current price per giga unit per tick
       **/
      getCurrentPricePerGigaUnitPerTick: AugmentedCall<ApiType, () => Observable<Balance>>;
      /**
       * Get the Providers that have at least one payment stream with a specific user.
       **/
      getProvidersWithPaymentStreamsWithUser: AugmentedCall<
        ApiType,
        (userAccount: AccountId | string | Uint8Array) => Observable<Vec<ProviderId>>
      >;
      /**
       * Get the payment streams of a provider.
       **/
      getUsersOfPaymentStreamsOfProvider: AugmentedCall<
        ApiType,
        (providerId: ProviderId | string | Uint8Array) => Observable<Vec<AccountId>>
      >;
      /**
       * Get the users that have a debt to the provider greater than the threshold.
       **/
      getUsersWithDebtOverThreshold: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array,
          threshold: Balance | AnyNumber | Uint8Array
        ) => Observable<Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x0be7208954c7c6c9/1 */
    proofsDealerApi: {
      /**
       * Get the challenge period for a given Provider.
       **/
      getChallengePeriod: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<BlockNumber, GetChallengePeriodError>>
      >;
      /**
       * Get the seed for a given challenge tick.
       **/
      getChallengeSeed: AugmentedCall<
        ApiType,
        (
          tick: BlockNumber | AnyNumber | Uint8Array
        ) => Observable<Result<RandomnessOutput, GetChallengeSeedError>>
      >;
      /**
       * Get challenges from a seed.
       **/
      getChallengesFromSeed: AugmentedCall<
        ApiType,
        (
          seed: RandomnessOutput | string | Uint8Array,
          providerId: ProviderId | string | Uint8Array,
          count: u32 | AnyNumber | Uint8Array
        ) => Observable<Vec<Key>>
      >;
      /**
       * Get the checkpoint challenge period.
       **/
      getCheckpointChallengePeriod: AugmentedCall<ApiType, () => Observable<BlockNumber>>;
      /**
       * Get checkpoint challenges for a given block.
       **/
      getCheckpointChallenges: AugmentedCall<
        ApiType,
        (
          tick: BlockNumber | AnyNumber | Uint8Array
        ) => Observable<
          Result<Vec<ITuple<[Key, Option<TrieRemoveMutation>]>>, GetCheckpointChallengesError>
        >
      >;
      /**
       * Get the current tick.
       **/
      getCurrentTick: AugmentedCall<ApiType, () => Observable<BlockNumber>>;
      /**
       * Get forest challenges from a seed.
       **/
      getForestChallengesFromSeed: AugmentedCall<
        ApiType,
        (
          seed: RandomnessOutput | string | Uint8Array,
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Vec<Key>>
      >;
      /**
       * Get the last checkpoint challenge tick.
       **/
      getLastCheckpointChallengeTick: AugmentedCall<ApiType, () => Observable<BlockNumber>>;
      /**
       * Get the last tick for which the submitter submitted a proof.
       **/
      getLastTickProviderSubmittedProof: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<BlockNumber, GetProofSubmissionRecordError>>
      >;
      /**
       * Get the next deadline tick.
       **/
      getNextDeadlineTick: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<BlockNumber, GetNextDeadlineTickError>>
      >;
      /**
       * Get the next tick for which the submitter should submit a proof.
       **/
      getNextTickToSubmitProofFor: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<BlockNumber, GetProofSubmissionRecordError>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xab3c0572291feb8b/1 */
    sessionKeys: {
      /**
       * Decode the given public session keys.
       **/
      decodeSessionKeys: AugmentedCall<
        ApiType,
        (
          encoded: Bytes | string | Uint8Array
        ) => Observable<Option<Vec<ITuple<[Bytes, KeyTypeId]>>>>
      >;
      /**
       * Generate a set of session keys with optionally using the given seed.
       **/
      generateSessionKeys: AugmentedCall<
        ApiType,
        (seed: Option<Bytes> | null | Uint8Array | Bytes | string) => Observable<Bytes>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x966604ffe78eb092/1 */
    storageProvidersApi: {
      /**
       * Check if a provider can be deleted.
       **/
      canDeleteProvider: AugmentedCall<
        ApiType,
        (providerId: ProviderId | string | Uint8Array) => Observable<bool>
      >;
      /**
       * Get the BSP info for a given BSP ID.
       **/
      getBspInfo: AugmentedCall<
        ApiType,
        (
          bspId: BackupStorageProviderId | string | Uint8Array
        ) => Observable<Result<BackupStorageProvider, GetBspInfoError>>
      >;
      /**
       * Get the stake of a BSP.
       **/
      getBspStake: AugmentedCall<
        ApiType,
        (
          bspId: BackupStorageProviderId | string | Uint8Array
        ) => Observable<Result<Balance, GetStakeError>>
      >;
      /**
       * Get the slashable amount corresponding to the configured max file size.
       **/
      getSlashAmountPerMaxFileSize: AugmentedCall<ApiType, () => Observable<Balance>>;
      /**
       * Get the Storage Provider ID for a given Account ID.
       **/
      getStorageProviderId: AugmentedCall<
        ApiType,
        (who: AccountId | string | Uint8Array) => Observable<Option<StorageProviderId>>
      >;
      /**
       * Get the worst case scenario slashable amount for a provider.
       **/
      getWorstCaseScenarioSlashableAmount: AugmentedCall<
        ApiType,
        (providerId: ProviderId | string | Uint8Array) => Observable<Option<Balance>>
      >;
      /**
       * Query the available storage capacity.
       **/
      queryAvailableStorageCapacity: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<StorageDataUnit, QueryAvailableStorageCapacityError>>
      >;
      /**
       * Get the Buckets that an MSP is storing.
       **/
      queryBucketsForMsp: AugmentedCall<
        ApiType,
        (
          mspId: MainStorageProviderId | string | Uint8Array
        ) => Observable<Result<Vec<BucketId>, QueryBucketsForMspError>>
      >;
      /**
       * Query the buckets stored by an MSP that belong to a specific user.
       **/
      queryBucketsOfUserStoredByMsp: AugmentedCall<
        ApiType,
        (
          mspId: ProviderId | string | Uint8Array,
          user: AccountId | string | Uint8Array
        ) => Observable<Result<Vec<H256>, QueryBucketsOfUserStoredByMspError>>
      >;
      /**
       * Query the earliest block number that a BSP can change its capacity.
       **/
      queryEarliestChangeCapacityBlock: AugmentedCall<
        ApiType,
        (
          providerId: BackupStorageProviderId | string | Uint8Array
        ) => Observable<Result<BlockNumber, QueryEarliestChangeCapacityBlockError>>
      >;
      /**
       * Query the MSP ID of a bucket ID.
       **/
      queryMspIdOfBucketId: AugmentedCall<
        ApiType,
        (
          bucketId: H256 | string | Uint8Array
        ) => Observable<Result<ProviderId, QueryMspIdOfBucketIdError>>
      >;
      /**
       * Query the provider's multiaddresses.
       **/
      queryProviderMultiaddresses: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<Multiaddresses, QueryProviderMultiaddressesError>>
      >;
      /**
       * Query the storage provider capacity.
       **/
      queryStorageProviderCapacity: AugmentedCall<
        ApiType,
        (
          providerId: ProviderId | string | Uint8Array
        ) => Observable<Result<StorageDataUnit, QueryStorageProviderCapacityError>>
      >;
      /**
       * Query the value propositions for a MSP.
       **/
      queryValuePropositionsForMsp: AugmentedCall<
        ApiType,
        (
          mspId: MainStorageProviderId | string | Uint8Array
        ) => Observable<Vec<ValuePropositionWithId>>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xd2bc9897eed08f15/3 */
    taggedTransactionQueue: {
      /**
       * Validate the transaction.
       **/
      validateTransaction: AugmentedCall<
        ApiType,
        (
          source: TransactionSource | "InBlock" | "Local" | "External" | number | Uint8Array,
          tx: Extrinsic | IExtrinsic | string | Uint8Array,
          blockHash: BlockHash | string | Uint8Array
        ) => Observable<TransactionValidity>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0x37c8bb1350a9a2a8/4 */
    transactionPaymentApi: {
      /**
       * The transaction fee details
       **/
      queryFeeDetails: AugmentedCall<
        ApiType,
        (
          uxt: Extrinsic | IExtrinsic | string | Uint8Array,
          len: u32 | AnyNumber | Uint8Array
        ) => Observable<FeeDetails>
      >;
      /**
       * The transaction info
       **/
      queryInfo: AugmentedCall<
        ApiType,
        (
          uxt: Extrinsic | IExtrinsic | string | Uint8Array,
          len: u32 | AnyNumber | Uint8Array
        ) => Observable<RuntimeDispatchInfo>
      >;
      /**
       * Query the output of the current LengthToFee given some input
       **/
      queryLengthToFee: AugmentedCall<
        ApiType,
        (length: u32 | AnyNumber | Uint8Array) => Observable<Balance>
      >;
      /**
       * Query the output of the current WeightToFee given some input
       **/
      queryWeightToFee: AugmentedCall<
        ApiType,
        (
          weight: Weight | { refTime?: any; proofSize?: any } | string | Uint8Array
        ) => Observable<Balance>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
    /** 0xf3ff14d5ab527059/3 */
    transactionPaymentCallApi: {
      /**
       * The call fee details
       **/
      queryCallFeeDetails: AugmentedCall<
        ApiType,
        (
          call: Call | IMethod | string | Uint8Array,
          len: u32 | AnyNumber | Uint8Array
        ) => Observable<FeeDetails>
      >;
      /**
       * The call info
       **/
      queryCallInfo: AugmentedCall<
        ApiType,
        (
          call: Call | IMethod | string | Uint8Array,
          len: u32 | AnyNumber | Uint8Array
        ) => Observable<RuntimeDispatchInfo>
      >;
      /**
       * Query the output of the current LengthToFee given some input
       **/
      queryLengthToFee: AugmentedCall<
        ApiType,
        (length: u32 | AnyNumber | Uint8Array) => Observable<Balance>
      >;
      /**
       * Query the output of the current WeightToFee given some input
       **/
      queryWeightToFee: AugmentedCall<
        ApiType,
        (
          weight: Weight | { refTime?: any; proofSize?: any } | string | Uint8Array
        ) => Observable<Balance>
      >;
      /**
       * Generic call
       **/
      [key: string]: DecoratedCallBase<ApiType>;
    };
  } // AugmentedCalls
} // declare module
