import type { Bytes, Enum, Struct, U8aFixed, u32, u64 } from "@polkadot/types-codec";
import type { AccountId, BlockNumber, H256 } from "@polkadot/types/interfaces/runtime";
/** @name BackupStorageProvider */
export interface BackupStorageProvider extends Struct {
  readonly capacity: StorageData;
  readonly data_used: StorageData;
  readonly multiaddresses: Bytes;
  readonly root: MerklePatriciaRoot;
  readonly last_capacity_change: BlockNumber;
  readonly owner_account: AccountId;
  readonly payment_account: AccountId;
}
/** @name BackupStorageProviderId */
export interface BackupStorageProviderId extends H256 {}
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
/** @name GetBspInfoError */
export interface GetBspInfoError extends Enum {
  readonly isBspNotRegistered: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "BspNotRegistered" | "InternalApiError";
}
/** @name GetChallengePeriodError */
export interface GetChallengePeriodError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly type: "ProviderNotRegistered";
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
/** @name GetLastTickProviderSubmittedProofError */
export interface GetLastTickProviderSubmittedProofError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isProviderNeverSubmittedProof: boolean;
  readonly isInternalApiError: boolean;
  readonly type: "ProviderNotRegistered" | "ProviderNeverSubmittedProof" | "InternalApiError";
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
/** @name GetUsersWithDebtOverThresholdError */
export interface GetUsersWithDebtOverThresholdError extends Enum {
  readonly isProviderNotRegistered: boolean;
  readonly isProviderWithoutPaymentStreams: boolean;
  readonly isAmountToChargeOverflow: boolean;
  readonly isDebtOverflow: boolean;
  readonly isInternalApiError: boolean;
  readonly type:
    | "ProviderNotRegistered"
    | "ProviderWithoutPaymentStreams"
    | "AmountToChargeOverflow"
    | "DebtOverflow"
    | "InternalApiError";
}
/** @name IncompleteFileStatus */
export interface IncompleteFileStatus extends Struct {
  readonly file_metadata: FileMetadata;
  readonly stored_chunks: u64;
  readonly total_chunks: u64;
}
/** @name Key */
export interface Key extends H256 {}
/** @name MainStorageProviderId */
export interface MainStorageProviderId extends H256 {}
/** @name MerklePatriciaRoot */
export interface MerklePatriciaRoot extends H256 {}
/** @name ProviderId */
export interface ProviderId extends H256 {}
/** @name QueryBspConfirmChunksToProveForFileError */
export interface QueryBspConfirmChunksToProveForFileError extends Enum {
  readonly isStorageRequestNotFound: boolean;
  readonly isInternalError: boolean;
  readonly type: "StorageRequestNotFound" | "InternalError";
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
/** @name RandomnessOutput */
export interface RandomnessOutput extends H256 {}
/** @name SaveFileToDisk */
export interface SaveFileToDisk extends Enum {
  readonly isFileNotFound: boolean;
  readonly isSuccess: boolean;
  readonly asSuccess: FileMetadata;
  readonly isIncompleteFile: boolean;
  readonly asIncompleteFile: IncompleteFileStatus;
  readonly type: "FileNotFound" | "Success" | "IncompleteFile";
}
/** @name StorageData */
export interface StorageData extends u32 {}
/** @name StorageProviderId */
export interface StorageProviderId extends Enum {
  readonly isBackupStorageProvider: boolean;
  readonly asBackupStorageProvider: BackupStorageProviderId;
  readonly isMainStorageProvider: boolean;
  readonly asMainStorageProvider: MainStorageProviderId;
  readonly type: "BackupStorageProvider" | "MainStorageProvider";
}
/** @name TrieRemoveMutation */
export interface TrieRemoveMutation extends Struct {}
export type PHANTOM_STORAGEHUBCLIENT = "storagehubclient";
