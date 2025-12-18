// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/api-base/types/events";

import type { ApiTypes, AugmentedEvent } from "@polkadot/api-base/types";
import type {
  Bytes,
  Null,
  Option,
  Result,
  U8aFixed,
  Vec,
  bool,
  u128,
  u32,
  u64
} from "@polkadot/types-codec";
import type { ITuple } from "@polkadot/types-codec/types";
import type { AccountId20, H160, H256 } from "@polkadot/types/interfaces/runtime";
import type {
  EthereumLog,
  EvmCoreErrorExitReason,
  FpAccountEthereumSignature,
  FrameSupportTokensMiscBalanceStatus,
  FrameSystemDispatchEventInfo,
  PalletFileSystemFileOperationIntention,
  PalletFileSystemRejectedStorageRequestReason,
  PalletNftsAttributeNamespace,
  PalletNftsPalletAttributes,
  PalletNftsPriceWithDirection,
  PalletProofsDealerCustomChallenge,
  PalletProofsDealerProof,
  PalletStorageProvidersStorageProviderId,
  PalletStorageProvidersTopUpMetadata,
  PalletStorageProvidersValueProposition,
  PalletStorageProvidersValuePropositionWithId,
  ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey,
  ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue,
  ShpFileMetadataFileMetadata,
  ShpTraitsTrieMutation,
  SpConsensusGrandpaAppPublic,
  SpRuntimeDispatchError
} from "@polkadot/types/lookup";

export type __AugmentedEvent<ApiType extends ApiTypes> = AugmentedEvent<ApiType>;

declare module "@polkadot/api-base/types/events" {
  interface AugmentedEvents<ApiType extends ApiTypes> {
    balances: {
      /**
       * A balance was set by root.
       **/
      BalanceSet: AugmentedEvent<
        ApiType,
        [who: AccountId20, free: u128],
        { who: AccountId20; free: u128 }
      >;
      /**
       * Some amount was burned from an account.
       **/
      Burned: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some amount was deposited (e.g. for transaction fees).
       **/
      Deposit: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * An account was removed whose balance was non-zero but below ExistentialDeposit,
       * resulting in an outright loss.
       **/
      DustLost: AugmentedEvent<
        ApiType,
        [account: AccountId20, amount: u128],
        { account: AccountId20; amount: u128 }
      >;
      /**
       * An account was created with some free balance.
       **/
      Endowed: AugmentedEvent<
        ApiType,
        [account: AccountId20, freeBalance: u128],
        { account: AccountId20; freeBalance: u128 }
      >;
      /**
       * Some balance was frozen.
       **/
      Frozen: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Total issuance was increased by `amount`, creating a credit to be balanced.
       **/
      Issued: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was locked.
       **/
      Locked: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some amount was minted into an account.
       **/
      Minted: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Total issuance was decreased by `amount`, creating a debt to be balanced.
       **/
      Rescinded: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was reserved (moved from free to reserved).
       **/
      Reserved: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some balance was moved from the reserve of the first account to the second account.
       * Final argument indicates the destination balance type.
       **/
      ReserveRepatriated: AugmentedEvent<
        ApiType,
        [
          from: AccountId20,
          to: AccountId20,
          amount: u128,
          destinationStatus: FrameSupportTokensMiscBalanceStatus
        ],
        {
          from: AccountId20;
          to: AccountId20;
          amount: u128;
          destinationStatus: FrameSupportTokensMiscBalanceStatus;
        }
      >;
      /**
       * Some amount was restored into an account.
       **/
      Restored: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some amount was removed from the account (e.g. for misbehavior).
       **/
      Slashed: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some amount was suspended from an account (it can be restored later).
       **/
      Suspended: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some balance was thawed.
       **/
      Thawed: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * The `TotalIssuance` was forcefully changed.
       **/
      TotalIssuanceForced: AugmentedEvent<
        ApiType,
        [old: u128, new_: u128],
        { old: u128; new_: u128 }
      >;
      /**
       * Transfer succeeded.
       **/
      Transfer: AugmentedEvent<
        ApiType,
        [from: AccountId20, to: AccountId20, amount: u128],
        { from: AccountId20; to: AccountId20; amount: u128 }
      >;
      /**
       * Some balance was unlocked.
       **/
      Unlocked: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Some balance was unreserved (moved from reserved to free).
       **/
      Unreserved: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * An account was upgraded.
       **/
      Upgraded: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Some amount was withdrawn from the account (e.g. for transaction fees).
       **/
      Withdraw: AugmentedEvent<
        ApiType,
        [who: AccountId20, amount: u128],
        { who: AccountId20; amount: u128 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    bucketNfts: {
      /**
       * Notifies that access to a bucket has been shared with another account.
       **/
      AccessShared: AugmentedEvent<
        ApiType,
        [issuer: AccountId20, recipient: AccountId20],
        { issuer: AccountId20; recipient: AccountId20 }
      >;
      /**
       * Notifies that an item has been burned.
       **/
      ItemBurned: AugmentedEvent<
        ApiType,
        [account: AccountId20, bucket: H256, itemId: u32],
        { account: AccountId20; bucket: H256; itemId: u32 }
      >;
      /**
       * Notifies that the read access for an item has been updated.
       **/
      ItemReadAccessUpdated: AugmentedEvent<
        ApiType,
        [admin: AccountId20, bucket: H256, itemId: u32],
        { admin: AccountId20; bucket: H256; itemId: u32 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    ethereum: {
      /**
       * An ethereum transaction was successfully executed.
       **/
      Executed: AugmentedEvent<
        ApiType,
        [
          from: H160,
          to: H160,
          transactionHash: H256,
          exitReason: EvmCoreErrorExitReason,
          extraData: Bytes
        ],
        {
          from: H160;
          to: H160;
          transactionHash: H256;
          exitReason: EvmCoreErrorExitReason;
          extraData: Bytes;
        }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    evm: {
      /**
       * A contract has been created at given address.
       **/
      Created: AugmentedEvent<ApiType, [address: H160], { address: H160 }>;
      /**
       * A contract was attempted to be created, but the execution failed.
       **/
      CreatedFailed: AugmentedEvent<ApiType, [address: H160], { address: H160 }>;
      /**
       * A contract has been executed successfully with states applied.
       **/
      Executed: AugmentedEvent<ApiType, [address: H160], { address: H160 }>;
      /**
       * A contract has been executed with errors. States are reverted with only gas fees applied.
       **/
      ExecutedFailed: AugmentedEvent<ApiType, [address: H160], { address: H160 }>;
      /**
       * Ethereum events from contracts.
       **/
      Log: AugmentedEvent<ApiType, [log: EthereumLog], { log: EthereumLog }>;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    fileSystem: {
      /**
       * Notifies that a BSP has been accepted to store a given file.
       **/
      AcceptedBspVolunteer: AugmentedEvent<
        ApiType,
        [
          bspId: H256,
          bucketId: H256,
          location: Bytes,
          fingerprint: H256,
          multiaddresses: Vec<Bytes>,
          owner: AccountId20,
          size_: u64
        ],
        {
          bspId: H256;
          bucketId: H256;
          location: Bytes;
          fingerprint: H256;
          multiaddresses: Vec<Bytes>;
          owner: AccountId20;
          size_: u64;
        }
      >;
      /**
       * Notifies that a BSP's challenge cycle has been initialised, adding the first file
       * key(s) to the BSP's Merkle Patricia Forest.
       **/
      BspChallengeCycleInitialised: AugmentedEvent<
        ApiType,
        [who: AccountId20, bspId: H256],
        { who: AccountId20; bspId: H256 }
      >;
      /**
       * Notifies that a BSP confirmed storing a file(s).
       **/
      BspConfirmedStoring: AugmentedEvent<
        ApiType,
        [
          who: AccountId20,
          bspId: H256,
          confirmedFileKeys: Vec<ITuple<[H256, ShpFileMetadataFileMetadata]>>,
          skippedFileKeys: Vec<H256>,
          newRoot: H256
        ],
        {
          who: AccountId20;
          bspId: H256;
          confirmedFileKeys: Vec<ITuple<[H256, ShpFileMetadataFileMetadata]>>;
          skippedFileKeys: Vec<H256>;
          newRoot: H256;
        }
      >;
      /**
       * Notifies that a BSP has stopped storing a file.
       **/
      BspConfirmStoppedStoring: AugmentedEvent<
        ApiType,
        [bspId: H256, fileKey: H256, newRoot: H256],
        { bspId: H256; fileKey: H256; newRoot: H256 }
      >;
      /**
       * Notifies that file deletions have been completed successfully for a BSP.
       **/
      BspFileDeletionsCompleted: AugmentedEvent<
        ApiType,
        [users: Vec<AccountId20>, fileKeys: Vec<H256>, bspId: H256, oldRoot: H256, newRoot: H256],
        { users: Vec<AccountId20>; fileKeys: Vec<H256>; bspId: H256; oldRoot: H256; newRoot: H256 }
      >;
      BspRequestedToStopStoring: AugmentedEvent<
        ApiType,
        [bspId: H256, fileKey: H256, owner: AccountId20, location: Bytes],
        { bspId: H256; fileKey: H256; owner: AccountId20; location: Bytes }
      >;
      /**
       * Notifies that an empty bucket has been deleted.
       **/
      BucketDeleted: AugmentedEvent<
        ApiType,
        [who: AccountId20, bucketId: H256, maybeCollectionId: Option<u32>],
        { who: AccountId20; bucketId: H256; maybeCollectionId: Option<u32> }
      >;
      /**
       * Notifies that file deletions have been completed successfully for a Bucket.
       **/
      BucketFileDeletionsCompleted: AugmentedEvent<
        ApiType,
        [
          user: AccountId20,
          fileKeys: Vec<H256>,
          bucketId: H256,
          mspId: Option<H256>,
          oldRoot: H256,
          newRoot: H256
        ],
        {
          user: AccountId20;
          fileKeys: Vec<H256>;
          bucketId: H256;
          mspId: Option<H256>;
          oldRoot: H256;
          newRoot: H256;
        }
      >;
      /**
       * Notifies that a bucket's privacy has been updated.
       **/
      BucketPrivacyUpdated: AugmentedEvent<
        ApiType,
        [who: AccountId20, bucketId: H256, collectionId: Option<u32>, private: bool],
        { who: AccountId20; bucketId: H256; collectionId: Option<u32>; private: bool }
      >;
      /**
       * Event to notify if, in the `on_idle` hook when cleaning up an expired storage request,
       * the return of that storage request's deposit to the user failed.
       **/
      FailedToReleaseStorageRequestCreationDeposit: AugmentedEvent<
        ApiType,
        [fileKey: H256, owner: AccountId20, amountToReturn: u128, error: SpRuntimeDispatchError],
        { fileKey: H256; owner: AccountId20; amountToReturn: u128; error: SpRuntimeDispatchError }
      >;
      /**
       * Notifies that a file deletion has been requested.
       * Contains a signed intention that allows any actor to execute the actual deletion.
       **/
      FileDeletionRequested: AugmentedEvent<
        ApiType,
        [
          signedDeleteIntention: PalletFileSystemFileOperationIntention,
          signature: FpAccountEthereumSignature
        ],
        {
          signedDeleteIntention: PalletFileSystemFileOperationIntention;
          signature: FpAccountEthereumSignature;
        }
      >;
      /**
       * Notifies that a storage request was marked as incomplete.
       *
       * This is important for fisherman nodes to listen and react to, to delete
       * the file key from the BSPs and/or Bucket storing that file from their forest.
       **/
      IncompleteStorageRequest: AugmentedEvent<ApiType, [fileKey: H256], { fileKey: H256 }>;
      /**
       * Notifies that an incomplete storage request has been fully cleaned up.
       *
       * This event is emitted in two scenarios:
       * 1. When an incomplete storage request is created but there are no providers to clean
       * (e.g., MSP confirmed with inclusion proof and no BSPs confirmed).
       * 2. When the file has been removed from all providers and the incomplete storage
       * request entry is removed from storage.
       **/
      IncompleteStorageRequestCleanedUp: AugmentedEvent<
        ApiType,
        [fileKey: H256],
        { fileKey: H256 }
      >;
      /**
       * Notifies that a bucket has been moved to a new MSP under a new value proposition.
       **/
      MoveBucketAccepted: AugmentedEvent<
        ApiType,
        [bucketId: H256, oldMspId: Option<H256>, newMspId: H256, valuePropId: H256],
        { bucketId: H256; oldMspId: Option<H256>; newMspId: H256; valuePropId: H256 }
      >;
      /**
       * Notifies that a bucket move request has been rejected by the MSP.
       **/
      MoveBucketRejected: AugmentedEvent<
        ApiType,
        [bucketId: H256, oldMspId: Option<H256>, newMspId: H256],
        { bucketId: H256; oldMspId: Option<H256>; newMspId: H256 }
      >;
      /**
       * Notifies that a bucket is being moved to a new MSP.
       **/
      MoveBucketRequested: AugmentedEvent<
        ApiType,
        [who: AccountId20, bucketId: H256, newMspId: H256, newValuePropId: H256],
        { who: AccountId20; bucketId: H256; newMspId: H256; newValuePropId: H256 }
      >;
      /**
       * Notifies that a move bucket request has expired.
       **/
      MoveBucketRequestExpired: AugmentedEvent<ApiType, [bucketId: H256], { bucketId: H256 }>;
      /**
       * Notifies that a Main Storage Provider (MSP) has accepted a storage request for a specific file key.
       *
       * This event is emitted when an MSP agrees to store a file, but the storage request
       * is not yet fully fulfilled (i.e., the required number of Backup Storage Providers
       * have not yet confirmed storage).
       *
       * # Note
       * This event is not emitted when the storage request is immediately fulfilled upon
       * MSP acceptance. In such cases, a [`StorageRequestFulfilled`] event is emitted instead.
       **/
      MspAcceptedStorageRequest: AugmentedEvent<
        ApiType,
        [fileKey: H256, fileMetadata: ShpFileMetadataFileMetadata],
        { fileKey: H256; fileMetadata: ShpFileMetadataFileMetadata }
      >;
      /**
       * Notifies that a MSP has stopped storing a bucket.
       **/
      MspStoppedStoringBucket: AugmentedEvent<
        ApiType,
        [mspId: H256, owner: AccountId20, bucketId: H256],
        { mspId: H256; owner: AccountId20; bucketId: H256 }
      >;
      /**
       * Notifies that a MSP has stopped storing a bucket because its owner has become insolvent.
       **/
      MspStopStoringBucketInsolventUser: AugmentedEvent<
        ApiType,
        [mspId: H256, owner: AccountId20, bucketId: H256],
        { mspId: H256; owner: AccountId20; bucketId: H256 }
      >;
      /**
       * Notifies that a new bucket has been created.
       **/
      NewBucket: AugmentedEvent<
        ApiType,
        [
          who: AccountId20,
          mspId: H256,
          bucketId: H256,
          name: Bytes,
          root: H256,
          collectionId: Option<u32>,
          private: bool,
          valuePropId: H256
        ],
        {
          who: AccountId20;
          mspId: H256;
          bucketId: H256;
          name: Bytes;
          root: H256;
          collectionId: Option<u32>;
          private: bool;
          valuePropId: H256;
        }
      >;
      /**
       * Notifies that a new collection has been created and associated with a bucket.
       **/
      NewCollectionAndAssociation: AugmentedEvent<
        ApiType,
        [who: AccountId20, bucketId: H256, collectionId: u32],
        { who: AccountId20; bucketId: H256; collectionId: u32 }
      >;
      /**
       * Notifies that a new file has been requested to be stored.
       **/
      NewStorageRequest: AugmentedEvent<
        ApiType,
        [
          who: AccountId20,
          fileKey: H256,
          bucketId: H256,
          location: Bytes,
          fingerprint: H256,
          size_: u64,
          peerIds: Vec<Bytes>,
          expiresAt: u32
        ],
        {
          who: AccountId20;
          fileKey: H256;
          bucketId: H256;
          location: Bytes;
          fingerprint: H256;
          size_: u64;
          peerIds: Vec<Bytes>;
          expiresAt: u32;
        }
      >;
      /**
       * Notifies that a SP has stopped storing a file because its owner has become insolvent.
       **/
      SpStopStoringInsolventUser: AugmentedEvent<
        ApiType,
        [spId: H256, fileKey: H256, owner: AccountId20, location: Bytes, newRoot: H256],
        { spId: H256; fileKey: H256; owner: AccountId20; location: Bytes; newRoot: H256 }
      >;
      /**
       * Notifies the expiration of a storage request. This means that the storage request has
       * been accepted by the MSP but the BSP target has not been reached (possibly 0 BSPs).
       * Note: This is a valid storage outcome, the user being responsible to track the number
       * of BSPs and choose to either delete the file and re-issue a storage request or continue.
       **/
      StorageRequestExpired: AugmentedEvent<ApiType, [fileKey: H256], { fileKey: H256 }>;
      /**
       * Notifies that a storage request for a file key has been fulfilled.
       * This means that the storage request has been accepted by the MSP and the BSP target
       * has been reached.
       **/
      StorageRequestFulfilled: AugmentedEvent<ApiType, [fileKey: H256], { fileKey: H256 }>;
      /**
       * Notifies that a storage request has either been directly rejected by the MSP or
       * the MSP did not respond to the storage request in time.
       * Note: the storage request will be marked as "incomplete", and it is expected that fisherman
       * nodes will pick it up and delete the file from the confirmed BSPs as well as the Bucket.
       **/
      StorageRequestRejected: AugmentedEvent<
        ApiType,
        [
          fileKey: H256,
          mspId: H256,
          bucketId: H256,
          reason: PalletFileSystemRejectedStorageRequestReason
        ],
        {
          fileKey: H256;
          mspId: H256;
          bucketId: H256;
          reason: PalletFileSystemRejectedStorageRequestReason;
        }
      >;
      /**
       * Notifies that a storage request has been revoked by the user who initiated it.
       * Note: the storage request will be marked as "incomplete", and it is expected that fisherman
       * nodes will pick it up and delete the file from the confirmed BSPs as well as the Bucket.
       **/
      StorageRequestRevoked: AugmentedEvent<ApiType, [fileKey: H256], { fileKey: H256 }>;
      /**
       * Event to notify of incoherencies in used capacity.
       **/
      UsedCapacityShouldBeZero: AugmentedEvent<
        ApiType,
        [actualUsedCapacity: u64],
        { actualUsedCapacity: u64 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    grandpa: {
      /**
       * New authority set has been applied.
       **/
      NewAuthorities: AugmentedEvent<
        ApiType,
        [authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>],
        { authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>> }
      >;
      /**
       * Current authority set has been paused.
       **/
      Paused: AugmentedEvent<ApiType, []>;
      /**
       * Current authority set has been resumed.
       **/
      Resumed: AugmentedEvent<ApiType, []>;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    nfts: {
      /**
       * All approvals of an item got cancelled.
       **/
      AllApprovalsCancelled: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, owner: AccountId20],
        { collection: u32; item: u32; owner: AccountId20 }
      >;
      /**
       * An approval for a `delegate` account to transfer the `item` of an item
       * `collection` was cancelled by its `owner`.
       **/
      ApprovalCancelled: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, owner: AccountId20, delegate: AccountId20],
        { collection: u32; item: u32; owner: AccountId20; delegate: AccountId20 }
      >;
      /**
       * Attribute metadata has been cleared for a `collection` or `item`.
       **/
      AttributeCleared: AugmentedEvent<
        ApiType,
        [
          collection: u32,
          maybeItem: Option<u32>,
          key: Bytes,
          namespace: PalletNftsAttributeNamespace
        ],
        {
          collection: u32;
          maybeItem: Option<u32>;
          key: Bytes;
          namespace: PalletNftsAttributeNamespace;
        }
      >;
      /**
       * New attribute metadata has been set for a `collection` or `item`.
       **/
      AttributeSet: AugmentedEvent<
        ApiType,
        [
          collection: u32,
          maybeItem: Option<u32>,
          key: Bytes,
          value: Bytes,
          namespace: PalletNftsAttributeNamespace
        ],
        {
          collection: u32;
          maybeItem: Option<u32>;
          key: Bytes;
          value: Bytes;
          namespace: PalletNftsAttributeNamespace;
        }
      >;
      /**
       * An `item` was destroyed.
       **/
      Burned: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, owner: AccountId20],
        { collection: u32; item: u32; owner: AccountId20 }
      >;
      /**
       * A `collection` has had its config changed by the `Force` origin.
       **/
      CollectionConfigChanged: AugmentedEvent<ApiType, [collection: u32], { collection: u32 }>;
      /**
       * Some `collection` was locked.
       **/
      CollectionLocked: AugmentedEvent<ApiType, [collection: u32], { collection: u32 }>;
      /**
       * Max supply has been set for a collection.
       **/
      CollectionMaxSupplySet: AugmentedEvent<
        ApiType,
        [collection: u32, maxSupply: u32],
        { collection: u32; maxSupply: u32 }
      >;
      /**
       * Metadata has been cleared for a `collection`.
       **/
      CollectionMetadataCleared: AugmentedEvent<ApiType, [collection: u32], { collection: u32 }>;
      /**
       * New metadata has been set for a `collection`.
       **/
      CollectionMetadataSet: AugmentedEvent<
        ApiType,
        [collection: u32, data: Bytes],
        { collection: u32; data: Bytes }
      >;
      /**
       * Mint settings for a collection had changed.
       **/
      CollectionMintSettingsUpdated: AugmentedEvent<
        ApiType,
        [collection: u32],
        { collection: u32 }
      >;
      /**
       * A `collection` was created.
       **/
      Created: AugmentedEvent<
        ApiType,
        [collection: u32, creator: AccountId20, owner: AccountId20],
        { collection: u32; creator: AccountId20; owner: AccountId20 }
      >;
      /**
       * A `collection` was destroyed.
       **/
      Destroyed: AugmentedEvent<ApiType, [collection: u32], { collection: u32 }>;
      /**
       * A `collection` was force-created.
       **/
      ForceCreated: AugmentedEvent<
        ApiType,
        [collection: u32, owner: AccountId20],
        { collection: u32; owner: AccountId20 }
      >;
      /**
       * An `item` was issued.
       **/
      Issued: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, owner: AccountId20],
        { collection: u32; item: u32; owner: AccountId20 }
      >;
      /**
       * A new approval to modify item attributes was added.
       **/
      ItemAttributesApprovalAdded: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, delegate: AccountId20],
        { collection: u32; item: u32; delegate: AccountId20 }
      >;
      /**
       * A new approval to modify item attributes was removed.
       **/
      ItemAttributesApprovalRemoved: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, delegate: AccountId20],
        { collection: u32; item: u32; delegate: AccountId20 }
      >;
      /**
       * An item was bought.
       **/
      ItemBought: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, price: u128, seller: AccountId20, buyer: AccountId20],
        { collection: u32; item: u32; price: u128; seller: AccountId20; buyer: AccountId20 }
      >;
      /**
       * Metadata has been cleared for an item.
       **/
      ItemMetadataCleared: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32],
        { collection: u32; item: u32 }
      >;
      /**
       * New metadata has been set for an item.
       **/
      ItemMetadataSet: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, data: Bytes],
        { collection: u32; item: u32; data: Bytes }
      >;
      /**
       * The price for the item was removed.
       **/
      ItemPriceRemoved: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32],
        { collection: u32; item: u32 }
      >;
      /**
       * The price was set for the item.
       **/
      ItemPriceSet: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, price: u128, whitelistedBuyer: Option<AccountId20>],
        { collection: u32; item: u32; price: u128; whitelistedBuyer: Option<AccountId20> }
      >;
      /**
       * `item` metadata or attributes were locked.
       **/
      ItemPropertiesLocked: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, lockMetadata: bool, lockAttributes: bool],
        { collection: u32; item: u32; lockMetadata: bool; lockAttributes: bool }
      >;
      /**
       * An `item` became non-transferable.
       **/
      ItemTransferLocked: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32],
        { collection: u32; item: u32 }
      >;
      /**
       * An `item` became transferable.
       **/
      ItemTransferUnlocked: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32],
        { collection: u32; item: u32 }
      >;
      /**
       * Event gets emitted when the `NextCollectionId` gets incremented.
       **/
      NextCollectionIdIncremented: AugmentedEvent<
        ApiType,
        [nextId: Option<u32>],
        { nextId: Option<u32> }
      >;
      /**
       * The owner changed.
       **/
      OwnerChanged: AugmentedEvent<
        ApiType,
        [collection: u32, newOwner: AccountId20],
        { collection: u32; newOwner: AccountId20 }
      >;
      /**
       * Ownership acceptance has changed for an account.
       **/
      OwnershipAcceptanceChanged: AugmentedEvent<
        ApiType,
        [who: AccountId20, maybeCollection: Option<u32>],
        { who: AccountId20; maybeCollection: Option<u32> }
      >;
      /**
       * A new attribute in the `Pallet` namespace was set for the `collection` or an `item`
       * within that `collection`.
       **/
      PalletAttributeSet: AugmentedEvent<
        ApiType,
        [collection: u32, item: Option<u32>, attribute: PalletNftsPalletAttributes, value: Bytes],
        { collection: u32; item: Option<u32>; attribute: PalletNftsPalletAttributes; value: Bytes }
      >;
      /**
       * New attributes have been set for an `item` of the `collection`.
       **/
      PreSignedAttributesSet: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, namespace: PalletNftsAttributeNamespace],
        { collection: u32; item: u32; namespace: PalletNftsAttributeNamespace }
      >;
      /**
       * The deposit for a set of `item`s within a `collection` has been updated.
       **/
      Redeposited: AugmentedEvent<
        ApiType,
        [collection: u32, successfulItems: Vec<u32>],
        { collection: u32; successfulItems: Vec<u32> }
      >;
      /**
       * The swap was cancelled.
       **/
      SwapCancelled: AugmentedEvent<
        ApiType,
        [
          offeredCollection: u32,
          offeredItem: u32,
          desiredCollection: u32,
          desiredItem: Option<u32>,
          price: Option<PalletNftsPriceWithDirection>,
          deadline: u32
        ],
        {
          offeredCollection: u32;
          offeredItem: u32;
          desiredCollection: u32;
          desiredItem: Option<u32>;
          price: Option<PalletNftsPriceWithDirection>;
          deadline: u32;
        }
      >;
      /**
       * The swap has been claimed.
       **/
      SwapClaimed: AugmentedEvent<
        ApiType,
        [
          sentCollection: u32,
          sentItem: u32,
          sentItemOwner: AccountId20,
          receivedCollection: u32,
          receivedItem: u32,
          receivedItemOwner: AccountId20,
          price: Option<PalletNftsPriceWithDirection>,
          deadline: u32
        ],
        {
          sentCollection: u32;
          sentItem: u32;
          sentItemOwner: AccountId20;
          receivedCollection: u32;
          receivedItem: u32;
          receivedItemOwner: AccountId20;
          price: Option<PalletNftsPriceWithDirection>;
          deadline: u32;
        }
      >;
      /**
       * An `item` swap intent was created.
       **/
      SwapCreated: AugmentedEvent<
        ApiType,
        [
          offeredCollection: u32,
          offeredItem: u32,
          desiredCollection: u32,
          desiredItem: Option<u32>,
          price: Option<PalletNftsPriceWithDirection>,
          deadline: u32
        ],
        {
          offeredCollection: u32;
          offeredItem: u32;
          desiredCollection: u32;
          desiredItem: Option<u32>;
          price: Option<PalletNftsPriceWithDirection>;
          deadline: u32;
        }
      >;
      /**
       * The management team changed.
       **/
      TeamChanged: AugmentedEvent<
        ApiType,
        [
          collection: u32,
          issuer: Option<AccountId20>,
          admin: Option<AccountId20>,
          freezer: Option<AccountId20>
        ],
        {
          collection: u32;
          issuer: Option<AccountId20>;
          admin: Option<AccountId20>;
          freezer: Option<AccountId20>;
        }
      >;
      /**
       * A tip was sent.
       **/
      TipSent: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, sender: AccountId20, receiver: AccountId20, amount: u128],
        { collection: u32; item: u32; sender: AccountId20; receiver: AccountId20; amount: u128 }
      >;
      /**
       * An `item` of a `collection` has been approved by the `owner` for transfer by
       * a `delegate`.
       **/
      TransferApproved: AugmentedEvent<
        ApiType,
        [
          collection: u32,
          item: u32,
          owner: AccountId20,
          delegate: AccountId20,
          deadline: Option<u32>
        ],
        {
          collection: u32;
          item: u32;
          owner: AccountId20;
          delegate: AccountId20;
          deadline: Option<u32>;
        }
      >;
      /**
       * An `item` was transferred.
       **/
      Transferred: AugmentedEvent<
        ApiType,
        [collection: u32, item: u32, from: AccountId20, to: AccountId20],
        { collection: u32; item: u32; from: AccountId20; to: AccountId20 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    offences: {
      /**
       * There is an offence reported of the given `kind` happened at the `session_index` and
       * (kind-specific) time slot. This event is not deposited for duplicate slashes.
       * \[kind, timeslot\].
       **/
      Offence: AugmentedEvent<
        ApiType,
        [kind: U8aFixed, timeslot: Bytes],
        { kind: U8aFixed; timeslot: Bytes }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    parameters: {
      /**
       * A Parameter was set.
       *
       * Is also emitted when the value was not changed.
       **/
      Updated: AugmentedEvent<
        ApiType,
        [
          key: ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey,
          oldValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>,
          newValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>
        ],
        {
          key: ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey;
          oldValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
          newValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
        }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    paymentStreams: {
      /**
       * Event emitted when a dynamic-rate payment stream is created. Provides information about the User and Provider of the stream
       * and the initial amount provided.
       **/
      DynamicRatePaymentStreamCreated: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256, amountProvided: u64],
        { userAccount: AccountId20; providerId: H256; amountProvided: u64 }
      >;
      /**
       * Event emitted when a dynamic-rate payment stream is removed. Provides information about the User and Provider of the stream.
       **/
      DynamicRatePaymentStreamDeleted: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256],
        { userAccount: AccountId20; providerId: H256 }
      >;
      /**
       * Event emitted when a dynamic-rate payment stream is updated. Provides information about the User and Provider of the stream
       * and the new amount provided.
       **/
      DynamicRatePaymentStreamUpdated: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256, newAmountProvided: u64],
        { userAccount: AccountId20; providerId: H256; newAmountProvided: u64 }
      >;
      /**
       * Event emitted when a fixed-rate payment stream is created. Provides information about the Provider and User of the stream
       * and its initial rate.
       **/
      FixedRatePaymentStreamCreated: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256, rate: u128],
        { userAccount: AccountId20; providerId: H256; rate: u128 }
      >;
      /**
       * Event emitted when a fixed-rate payment stream is removed. Provides information about the User and Provider of the stream.
       **/
      FixedRatePaymentStreamDeleted: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256],
        { userAccount: AccountId20; providerId: H256 }
      >;
      /**
       * Event emitted when a fixed-rate payment stream is updated. Provides information about the User and Provider of the stream
       * and the new rate of the stream.
       **/
      FixedRatePaymentStreamUpdated: AugmentedEvent<
        ApiType,
        [userAccount: AccountId20, providerId: H256, newRate: u128],
        { userAccount: AccountId20; providerId: H256; newRate: u128 }
      >;
      /**
       * Event emitted when the `on_poll` hook detects that the tick of the proof submitters that needs to process is not the one immediately after the last processed tick.
       **/
      InconsistentTickProcessing: AugmentedEvent<
        ApiType,
        [lastProcessedTick: u32, tickToProcess: u32],
        { lastProcessedTick: u32; tickToProcess: u32 }
      >;
      /**
       * Event emitted when a Provider's last chargeable tick and price index are updated. Provides information about the Provider of the stream,
       * the tick number of the last chargeable tick and the price index at that tick.
       **/
      LastChargeableInfoUpdated: AugmentedEvent<
        ApiType,
        [providerId: H256, lastChargeableTick: u32, lastChargeablePriceIndex: u128],
        { providerId: H256; lastChargeableTick: u32; lastChargeablePriceIndex: u128 }
      >;
      /**
       * Event emitted when a payment is charged. Provides information about the user that was charged,
       * the Provider that received the funds, the tick up to which it was charged and the amount that was charged.
       **/
      PaymentStreamCharged: AugmentedEvent<
        ApiType,
        [
          userAccount: AccountId20,
          providerId: H256,
          amount: u128,
          lastTickCharged: u32,
          chargedAtTick: u32
        ],
        {
          userAccount: AccountId20;
          providerId: H256;
          amount: u128;
          lastTickCharged: u32;
          chargedAtTick: u32;
        }
      >;
      /**
       * Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has paid all its outstanding debt.
       **/
      UserPaidAllDebts: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has paid some (but not all) of its outstanding debt.
       **/
      UserPaidSomeDebts: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Event emitted when multiple payment streams have been charged from a Provider. Provides information about
       * the charged users, the Provider that received the funds and the tick when the charge happened.
       **/
      UsersCharged: AugmentedEvent<
        ApiType,
        [userAccounts: Vec<AccountId20>, providerId: H256, chargedAtTick: u32],
        { userAccounts: Vec<AccountId20>; providerId: H256; chargedAtTick: u32 }
      >;
      /**
       * Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has waited the cooldown period,
       * correctly paid all their outstanding debt and can now contract new services again.
       **/
      UserSolvent: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Event emitted when a Provider is correctly trying to charge a User and that User does not have enough funds to pay for their services.
       * This event is emitted to flag the user and let the network know that the user is not paying for the requested services, so other Providers can
       * stop providing services to that user.
       **/
      UserWithoutFunds: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    proofsDealer: {
      /**
       * The [`ChallengesTicker`] has been paused or unpaused.
       **/
      ChallengesTickerSet: AugmentedEvent<ApiType, [paused: bool], { paused: bool }>;
      /**
       * A set of mutations has been applied to a given Forest.
       * This is the generic version of [`MutationsAppliedForProvider`](Event::MutationsAppliedForProvider)
       * when [`generic_apply_delta`](ProofsDealerInterface::generic_apply_delta) is used
       * and the root is not necessarily linked to a specific Provider.
       *
       * Additional information for context on where the mutations were applied can be provided
       * by using the `event_info` field.
       **/
      MutationsApplied: AugmentedEvent<
        ApiType,
        [
          mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>,
          oldRoot: H256,
          newRoot: H256,
          eventInfo: Option<Bytes>
        ],
        {
          mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>;
          oldRoot: H256;
          newRoot: H256;
          eventInfo: Option<Bytes>;
        }
      >;
      /**
       * A set of mutations has been applied to the Forest of a given Provider.
       **/
      MutationsAppliedForProvider: AugmentedEvent<
        ApiType,
        [
          providerId: H256,
          mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>,
          oldRoot: H256,
          newRoot: H256
        ],
        {
          providerId: H256;
          mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>;
          oldRoot: H256;
          newRoot: H256;
        }
      >;
      /**
       * A manual challenge was submitted.
       **/
      NewChallenge: AugmentedEvent<
        ApiType,
        [who: Option<AccountId20>, keyChallenged: H256],
        { who: Option<AccountId20>; keyChallenged: H256 }
      >;
      /**
       * A provider's challenge cycle was initialised.
       **/
      NewChallengeCycleInitialised: AugmentedEvent<
        ApiType,
        [
          currentTick: u32,
          nextChallengeDeadline: u32,
          provider: H256,
          maybeProviderAccount: Option<AccountId20>
        ],
        {
          currentTick: u32;
          nextChallengeDeadline: u32;
          provider: H256;
          maybeProviderAccount: Option<AccountId20>;
        }
      >;
      /**
       * A new challenge seed was generated.
       **/
      NewChallengeSeed: AugmentedEvent<
        ApiType,
        [challengesTicker: u32, seed: H256],
        { challengesTicker: u32; seed: H256 }
      >;
      /**
       * A new checkpoint challenge was generated.
       **/
      NewCheckpointChallenge: AugmentedEvent<
        ApiType,
        [challengesTicker: u32, challenges: Vec<PalletProofsDealerCustomChallenge>],
        { challengesTicker: u32; challenges: Vec<PalletProofsDealerCustomChallenge> }
      >;
      /**
       * A priority challenge was submitted.
       **/
      NewPriorityChallenge: AugmentedEvent<
        ApiType,
        [who: Option<AccountId20>, keyChallenged: H256, shouldRemoveKey: bool],
        { who: Option<AccountId20>; keyChallenged: H256; shouldRemoveKey: bool }
      >;
      /**
       * No record of the last tick the Provider submitted a proof for.
       **/
      NoRecordOfLastSubmittedProof: AugmentedEvent<ApiType, [provider: H256], { provider: H256 }>;
      /**
       * A proof was accepted.
       **/
      ProofAccepted: AugmentedEvent<
        ApiType,
        [providerId: H256, proof: PalletProofsDealerProof, lastTickProven: u32],
        { providerId: H256; proof: PalletProofsDealerProof; lastTickProven: u32 }
      >;
      /**
       * A provider was marked as slashable and their challenge deadline was forcefully pushed.
       **/
      SlashableProvider: AugmentedEvent<
        ApiType,
        [provider: H256, nextChallengeDeadline: u32],
        { provider: H256; nextChallengeDeadline: u32 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    providers: {
      /**
       * Event emitted when a provider has been slashed and they have reached a capacity deficit (i.e. the provider's capacity fell below their used capacity)
       * signalling the end of the grace period since an automatic top up could not be performed due to insufficient free balance.
       **/
      AwaitingTopUp: AugmentedEvent<
        ApiType,
        [providerId: H256, topUpMetadata: PalletStorageProvidersTopUpMetadata],
        { providerId: H256; topUpMetadata: PalletStorageProvidersTopUpMetadata }
      >;
      /**
       * Event emitted when a BSP has been deleted.
       **/
      BspDeleted: AugmentedEvent<ApiType, [providerId: H256], { providerId: H256 }>;
      /**
       * Event emitted when a Backup Storage Provider has requested to sign up successfully. Provides information about
       * that BSP's account id, its multiaddresses, and the total data it can store according to its stake.
       **/
      BspRequestSignUpSuccess: AugmentedEvent<
        ApiType,
        [who: AccountId20, multiaddresses: Vec<Bytes>, capacity: u64],
        { who: AccountId20; multiaddresses: Vec<Bytes>; capacity: u64 }
      >;
      /**
       * Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
       * that BSP's account id.
       **/
      BspSignOffSuccess: AugmentedEvent<
        ApiType,
        [who: AccountId20, bspId: H256],
        { who: AccountId20; bspId: H256 }
      >;
      /**
       * Event emitted when a Backup Storage Provider has confirmed its sign up successfully. Provides information about
       * that BSP's account id, the initial root of the Merkle Patricia Trie that it stores, the total data it can store
       * according to its stake, and its multiaddress.
       **/
      BspSignUpSuccess: AugmentedEvent<
        ApiType,
        [who: AccountId20, bspId: H256, root: H256, multiaddresses: Vec<Bytes>, capacity: u64],
        { who: AccountId20; bspId: H256; root: H256; multiaddresses: Vec<Bytes>; capacity: u64 }
      >;
      /**
       * Event emitted when a bucket's root has been changed.
       **/
      BucketRootChanged: AugmentedEvent<
        ApiType,
        [bucketId: H256, oldRoot: H256, newRoot: H256],
        { bucketId: H256; oldRoot: H256; newRoot: H256 }
      >;
      /**
       * Event emitted when the provider that has been marked as insolvent was a MSP. It notifies the users of that MSP
       * the buckets that it was holding, so they can take appropriate measures.
       **/
      BucketsOfInsolventMsp: AugmentedEvent<
        ApiType,
        [mspId: H256, buckets: Vec<H256>],
        { mspId: H256; buckets: Vec<H256> }
      >;
      /**
       * Event emitted when a SP has changed its capacity successfully. Provides information about
       * that SP's account id, its old total data that could store, and the new total data.
       **/
      CapacityChanged: AugmentedEvent<
        ApiType,
        [
          who: AccountId20,
          providerId: PalletStorageProvidersStorageProviderId,
          oldCapacity: u64,
          newCapacity: u64,
          nextBlockWhenChangeAllowed: u32
        ],
        {
          who: AccountId20;
          providerId: PalletStorageProvidersStorageProviderId;
          oldCapacity: u64;
          newCapacity: u64;
          nextBlockWhenChangeAllowed: u32;
        }
      >;
      /**
       * Event emitted when the account ID of a provider that has just been marked as insolvent can't be found in storage.
       **/
      FailedToGetOwnerAccountOfInsolventProvider: AugmentedEvent<
        ApiType,
        [providerId: H256],
        { providerId: H256 }
      >;
      /**
       * Event emitted when there was an inconsistency error and the provider was found in `ProviderTopUpExpirations`
       * for a tick that wasn't actually when its top up expired, and when trying to insert it with the actual
       * expiration tick in `ProviderTopUpExpirations` the append failed.
       *
       * The result of this is that the provider's top up expiration will be reinserted at the correct expiration tick based on the
       * `TopUpMetadata` found in `AwaitingTopUpFromProviders` storage.
       **/
      FailedToInsertProviderTopUpExpiration: AugmentedEvent<
        ApiType,
        [providerId: H256, expirationTick: u32],
        { providerId: H256; expirationTick: u32 }
      >;
      /**
       * Event emitted when there's an error slashing the now insolvent provider.
       **/
      FailedToSlashInsolventProvider: AugmentedEvent<
        ApiType,
        [providerId: H256, amountToSlash: u128, error: SpRuntimeDispatchError],
        { providerId: H256; amountToSlash: u128; error: SpRuntimeDispatchError }
      >;
      /**
       * Event emitted when there's an error stopping all cycles for an insolvent Backup Storage Provider.
       **/
      FailedToStopAllCyclesForInsolventBsp: AugmentedEvent<
        ApiType,
        [providerId: H256, error: SpRuntimeDispatchError],
        { providerId: H256; error: SpRuntimeDispatchError }
      >;
      /**
       * Event emitted when an MSP has been deleted.
       **/
      MspDeleted: AugmentedEvent<ApiType, [providerId: H256], { providerId: H256 }>;
      /**
       * Event emitted when a Main Storage Provider has requested to sign up successfully. Provides information about
       * that MSP's account id, its multiaddresses, the total data it can store according to its stake, and its value proposition.
       **/
      MspRequestSignUpSuccess: AugmentedEvent<
        ApiType,
        [who: AccountId20, multiaddresses: Vec<Bytes>, capacity: u64],
        { who: AccountId20; multiaddresses: Vec<Bytes>; capacity: u64 }
      >;
      /**
       * Event emitted when a Main Storage Provider has signed off successfully. Provides information about
       * that MSP's account id.
       **/
      MspSignOffSuccess: AugmentedEvent<
        ApiType,
        [who: AccountId20, mspId: H256],
        { who: AccountId20; mspId: H256 }
      >;
      /**
       * Event emitted when a Main Storage Provider has confirmed its sign up successfully. Provides information about
       * that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
       **/
      MspSignUpSuccess: AugmentedEvent<
        ApiType,
        [
          who: AccountId20,
          mspId: H256,
          multiaddresses: Vec<Bytes>,
          capacity: u64,
          valueProp: PalletStorageProvidersValuePropositionWithId
        ],
        {
          who: AccountId20;
          mspId: H256;
          multiaddresses: Vec<Bytes>;
          capacity: u64;
          valueProp: PalletStorageProvidersValuePropositionWithId;
        }
      >;
      /**
       * Event emitted when a Provider has added a new MultiAddress to its account.
       **/
      MultiAddressAdded: AugmentedEvent<
        ApiType,
        [providerId: H256, newMultiaddress: Bytes],
        { providerId: H256; newMultiaddress: Bytes }
      >;
      /**
       * Event emitted when a Provider has removed a MultiAddress from its account.
       **/
      MultiAddressRemoved: AugmentedEvent<
        ApiType,
        [providerId: H256, removedMultiaddress: Bytes],
        { providerId: H256; removedMultiaddress: Bytes }
      >;
      /**
       * Event emitted when a provider has been marked as insolvent.
       *
       * This happens when the provider hasn't topped up their deposit within the grace period after being slashed
       * and they have a capacity deficit (i.e. their capacity based on their stake is below their used capacity by the files it stores).
       **/
      ProviderInsolvent: AugmentedEvent<ApiType, [providerId: H256], { providerId: H256 }>;
      /**
       * Event emitted when a sign up request has been canceled successfully. Provides information about
       * the account id of the user that canceled the request.
       **/
      SignUpRequestCanceled: AugmentedEvent<ApiType, [who: AccountId20], { who: AccountId20 }>;
      /**
       * Event emitted when a SP has been slashed.
       **/
      Slashed: AugmentedEvent<
        ApiType,
        [providerId: H256, amount: u128],
        { providerId: H256; amount: u128 }
      >;
      /**
       * Event emitted when an SP has topped up its deposit based on slash amount.
       **/
      TopUpFulfilled: AugmentedEvent<
        ApiType,
        [providerId: H256, amount: u128],
        { providerId: H256; amount: u128 }
      >;
      /**
       * Event emitted when an MSP adds a new value proposition.
       **/
      ValuePropAdded: AugmentedEvent<
        ApiType,
        [mspId: H256, valuePropId: H256, valueProp: PalletStorageProvidersValueProposition],
        { mspId: H256; valuePropId: H256; valueProp: PalletStorageProvidersValueProposition }
      >;
      /**
       * Event emitted when an MSP's value proposition is made unavailable.
       **/
      ValuePropUnavailable: AugmentedEvent<
        ApiType,
        [mspId: H256, valuePropId: H256],
        { mspId: H256; valuePropId: H256 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    randomness: {
      /**
       * Event emitted when a new random seed is available from the relay chain
       **/
      NewOneEpochAgoRandomnessAvailable: AugmentedEvent<
        ApiType,
        [randomnessSeed: H256, fromEpoch: u64, validUntilBlock: u32],
        { randomnessSeed: H256; fromEpoch: u64; validUntilBlock: u32 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    session: {
      /**
       * New session has happened. Note that the argument is the session index, not the
       * block number as the type might suggest.
       **/
      NewSession: AugmentedEvent<ApiType, [sessionIndex: u32], { sessionIndex: u32 }>;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    sudo: {
      /**
       * The sudo key has been updated.
       **/
      KeyChanged: AugmentedEvent<
        ApiType,
        [old: Option<AccountId20>, new_: AccountId20],
        { old: Option<AccountId20>; new_: AccountId20 }
      >;
      /**
       * The key was permanently removed.
       **/
      KeyRemoved: AugmentedEvent<ApiType, []>;
      /**
       * A sudo call just took place.
       **/
      Sudid: AugmentedEvent<
        ApiType,
        [sudoResult: Result<Null, SpRuntimeDispatchError>],
        { sudoResult: Result<Null, SpRuntimeDispatchError> }
      >;
      /**
       * A [sudo_as](Pallet::sudo_as) call just took place.
       **/
      SudoAsDone: AugmentedEvent<
        ApiType,
        [sudoResult: Result<Null, SpRuntimeDispatchError>],
        { sudoResult: Result<Null, SpRuntimeDispatchError> }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    system: {
      /**
       * `:code` was updated.
       **/
      CodeUpdated: AugmentedEvent<ApiType, []>;
      /**
       * An extrinsic failed.
       **/
      ExtrinsicFailed: AugmentedEvent<
        ApiType,
        [dispatchError: SpRuntimeDispatchError, dispatchInfo: FrameSystemDispatchEventInfo],
        { dispatchError: SpRuntimeDispatchError; dispatchInfo: FrameSystemDispatchEventInfo }
      >;
      /**
       * An extrinsic completed successfully.
       **/
      ExtrinsicSuccess: AugmentedEvent<
        ApiType,
        [dispatchInfo: FrameSystemDispatchEventInfo],
        { dispatchInfo: FrameSystemDispatchEventInfo }
      >;
      /**
       * An account was reaped.
       **/
      KilledAccount: AugmentedEvent<ApiType, [account: AccountId20], { account: AccountId20 }>;
      /**
       * A new account was created.
       **/
      NewAccount: AugmentedEvent<ApiType, [account: AccountId20], { account: AccountId20 }>;
      /**
       * On on-chain remark happened.
       **/
      Remarked: AugmentedEvent<
        ApiType,
        [sender: AccountId20, hash_: H256],
        { sender: AccountId20; hash_: H256 }
      >;
      /**
       * An upgrade was authorized.
       **/
      UpgradeAuthorized: AugmentedEvent<
        ApiType,
        [codeHash: H256, checkVersion: bool],
        { codeHash: H256; checkVersion: bool }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
    transactionPayment: {
      /**
       * A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,
       * has been paid by `who`.
       **/
      TransactionFeePaid: AugmentedEvent<
        ApiType,
        [who: AccountId20, actualFee: u128, tip: u128],
        { who: AccountId20; actualFee: u128; tip: u128 }
      >;
      /**
       * Generic event
       **/
      [key: string]: AugmentedEvent<ApiType>;
    };
  } // AugmentedEvents
} // declare module
