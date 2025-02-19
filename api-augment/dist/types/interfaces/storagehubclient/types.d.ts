import type {
  Bytes,
  Enum,
  Null,
  Struct,
  U8aFixed,
  Vec,
  bool,
  u32,
  u64
} from "@polkadot/types-codec";
import type { AccountId, BlockNumber, H256 } from "@polkadot/types/interfaces/runtime";
/** @name BackupStorageProvider */
export interface BackupStorageProvider extends Struct {
  readonly capacity: StorageDataUnit;
  readonly capacity_used: StorageDataUnit;
  readonly multiaddresses: Multiaddresses;
  readonly root: MerklePatriciaRoot;
  readonly last_capacity_change: BlockNumber;
  readonly owner_account: AccountId;
  readonly payment_account: AccountId;
  readonly reputation_weight: ReputationWeightType;
  readonly sign_up_block: BlockNumber;
}
/** @name BackupStorageProviderId */
export interface BackupStorageProviderId extends H256 {}
/** @name BucketId */
export interface BucketId extends H256 {}
/** @name CheckpointChallenge */
export interface CheckpointChallenge extends Struct {
  readonly file_key: H256;
  readonly should_remove_file: bool;
}
/** @name ChunkId */
export interface ChunkId extends u64 {}
/** @name FileMetadata */
export interface FileMetadata extends Struct {
  readonly owner: Bytes;
  readonly bucket_id: Bytes;
  readonly location: Bytes;
  readonly file_size: u64;
  readonly fingerprint: U8aFixed;
}
/** @name GenericApplyDeltaEventInfoError */
export interface GenericApplyDeltaEventInfoError extends Enum {
  readonly isDecodeError: boolean;
  readonly type: "DecodeError";
}
/** @name GetBspInfoError */
export interface GetBspInfoError extends Enum {
  readonly isBspNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "BspNotRegistered" | "InternalApiError";
}
/** @name GetChallengePeriodError */
export interface GetChallengePeriodError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalApiError";
}
/** @name GetChallengeSeedError */
export interface GetChallengeSeedError extends Enum {
  readonly isTickBeyondLastSeedStored: boolean;
  readonly isTickIsInTheFuture: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "TickBeyondLastSeedStored" | "TickIsInTheFuture" | "InternalApiError";
}
/** @name GetCheckpointChallengesError */
export interface GetCheckpointChallengesError extends Enum {
  readonly isTickGreaterThanLastCheckpointTick: boolean;
  readonly isNoCheckpointChallengesInTick: boolean;
  readonly isInternalApiError: boolean;
  readonly type:
    | "TickGreaterThanLastCheckpointTick"
    | "NoCheckpointChallengesInTick"
    | "InternalApiError";
}
/** @name GetFileFromFileStorageResult */
export interface GetFileFromFileStorageResult extends Enum {
  readonly isFileNotFound: boolean;
  readonly isFileFound: boolean;
  readonly asFileFound: FileMetadata;
  readonly isIncompleteFile: boolean;
  readonly asIncompleteFile: IncompleteFileStatus;
  readonly isFileFoundWithInconsistency: boolean;
  readonly asFileFoundWithInconsistency: FileMetadata;
  readonly type: "FileNotFound" | "FileFound" | "IncompleteFile" | "FileFoundWithInconsistency";
}
/** @name GetNextDeadlineTickError */
export interface GetNextDeadlineTickError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isProviderNotInitialised: boolean;
  readonly isArithmeticOverflow: boolean;
  readonly isInternalApiError: boolean;
  readonly type:
    | "ProviderNotRegistered"
    | "ProviderNotInitialised"
    | "ArithmeticOverflow"
    | "InternalApiError";
}
/** @name GetProofSubmissionRecordError */
export interface GetProofSubmissionRecordError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isProviderNeverSubmittedProof: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "ProviderNeverSubmittedProof" | "InternalApiError";
}
/** @name GetStakeError */
export interface GetStakeError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalError";
}
/** @name GetUsersWithDebtOverThresholdError */
export interface GetUsersWithDebtOverThresholdError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isProviderWithoutPaymentStreams: boolean;
  readonly isAmountToChargeOverflow: boolean;
  readonly isAmountToChargeUnderflow: boolean;
  readonly isDebtOverflow: boolean;
  readonly isInternalApiError: boolean;
  readonly type:
    | "ProviderNotRegistered"
    | "ProviderWithoutPaymentStreams"
    | "AmountToChargeOverflow"
    | "AmountToChargeUnderflow"
    | "DebtOverflow"
    | "InternalApiError";
}
/** @name IncompleteFileStatus */
export interface IncompleteFileStatus extends Struct {
  readonly file_metadata: FileMetadata;
  readonly stored_chunks: u64;
  readonly total_chunks: u64;
}
/** @name IsStorageRequestOpenToVolunteersError */
export interface IsStorageRequestOpenToVolunteersError extends Enum {
  readonly isStorageRequestNotFound: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "StorageRequestNotFound" | "InternalApiError";
}
/** @name Key */
export interface Key extends H256 {}
/** @name LoadFileInStorageResult */
export interface LoadFileInStorageResult extends Struct {
  readonly file_key: H256;
  readonly file_metadata: FileMetadata;
}
/** @name MainStorageProviderId */
export interface MainStorageProviderId extends H256 {}
/** @name MerklePatriciaRoot */
export interface MerklePatriciaRoot extends H256 {}
/** @name Multiaddresses */
export interface Multiaddresses extends Vec<Bytes> {}
/** @name ProviderId */
export interface ProviderId extends H256 {}
/** @name QueryAvailableStorageCapacityError */
export interface QueryAvailableStorageCapacityError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalApiError";
}
/** @name QueryBspConfirmChunksToProveForFileError */
export interface QueryBspConfirmChunksToProveForFileError extends Enum {
  readonly isStorageRequestNotFound: boolean;
  readonly isConfirmChunks: boolean;
  readonly asConfirmChunks: QueryConfirmChunksToProveForFileError;
  readonly isInternalError: boolean;
  readonly type: "StorageRequestNotFound" | "ConfirmChunks" | "InternalError";
}
/** @name QueryBucketsForMspError */
export interface QueryBucketsForMspError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalError";
}
/** @name QueryConfirmChunksToProveForFileError */
export interface QueryConfirmChunksToProveForFileError extends Enum {
  readonly isChallengedChunkToChunkIdError: boolean;
  readonly type: "ChallengedChunkToChunkIdError";
}
/** @name QueryEarliestChangeCapacityBlockError */
export interface QueryEarliestChangeCapacityBlockError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalApiError";
}
/** @name QueryFileEarliestVolunteerBlockError */
export interface QueryFileEarliestVolunteerBlockError extends Enum {
  readonly isFailedToEncodeFingerprint: boolean;
  readonly isFailedToEncodeBsp: boolean;
  readonly isThresholdArithmeticError: boolean;
  readonly isStorageRequestNotFound: boolean;
  readonly isInternalError: boolean;
  readonly type:
    | "FailedToEncodeFingerprint"
    | "FailedToEncodeBsp"
    | "ThresholdArithmeticError"
    | "StorageRequestNotFound"
    | "InternalError";
}
/** @name QueryMspConfirmChunksToProveForFileError */
export interface QueryMspConfirmChunksToProveForFileError extends Enum {
  readonly isStorageRequestNotFound: boolean;
  readonly isConfirmChunks: boolean;
  readonly asConfirmChunks: QueryConfirmChunksToProveForFileError;
  readonly isInternalError: boolean;
  readonly type: "StorageRequestNotFound" | "ConfirmChunks" | "InternalError";
}
/** @name QueryMspIdOfBucketIdError */
export interface QueryMspIdOfBucketIdError extends Enum {
  readonly isBucketNotFound: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "BucketNotFound" | "InternalApiError";
}
/** @name QueryProviderMultiaddressesError */
export interface QueryProviderMultiaddressesError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalApiError";
}
/** @name QueryStorageProviderCapacityError */
export interface QueryStorageProviderCapacityError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "InternalApiError";
}
/** @name RandomnessOutput */
export interface RandomnessOutput extends H256 {}
/** @name ReputationWeightType */
export interface ReputationWeightType extends u32 {}
/** @name SaveFileToDisk */
export interface SaveFileToDisk extends Enum {
  readonly isFileNotFound: boolean;
  readonly isSuccess: boolean;
  readonly asSuccess: FileMetadata;
  readonly isIncompleteFile: boolean;
  readonly asIncompleteFile: IncompleteFileStatus;
  readonly type: "FileNotFound" | "Success" | "IncompleteFile";
}
/** @name ShouldRemoveFile */
export interface ShouldRemoveFile extends bool {}
/** @name StorageData */
export interface StorageData extends u64 {}
/** @name StorageDataUnit */
export interface StorageDataUnit extends u64 {}
/** @name StorageProviderId */
export interface StorageProviderId extends Enum {
  readonly isBackupStorageProvider: boolean;
  readonly asBackupStorageProvider: BackupStorageProviderId;
  readonly isMainStorageProvider: boolean;
  readonly asMainStorageProvider: MainStorageProviderId;
  readonly type: "BackupStorageProvider" | "MainStorageProvider";
}
/** @name TrieRemoveMutation */
export interface TrieRemoveMutation extends Null {}
/** @name ValuePropId */
export interface ValuePropId extends H256 {}
/** @name ValueProposition */
export interface ValueProposition extends Struct {
  readonly price_per_giga_unit_of_data_per_block: u64;
  readonly bucket_data_limit: StorageDataUnit;
}
/** @name ValuePropositionWithId */
export interface ValuePropositionWithId extends Struct {
  readonly id: ValuePropId;
  readonly value_prop: ValueProposition;
}
export type PHANTOM_STORAGEHUBCLIENT = "storagehubclient";
