import type { Bytes, Enum, Struct, U8aFixed, u32, u64 } from '@polkadot/types-codec';
import type { AccountId, BlockNumber, H256 } from '@polkadot/types/interfaces/runtime';
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
export interface BackupStorageProviderId extends H256 {
}
/** @name ChunkId */
export interface ChunkId extends u64 {
}
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
    readonly type: 'BspNotRegistered' | 'InternalApiError';
}
/** @name IncompleteFileStatus */
export interface IncompleteFileStatus extends Struct {
    readonly file_metadata: FileMetadata;
    readonly stored_chunks: u64;
    readonly total_chunks: u64;
}
/** @name MerklePatriciaRoot */
export interface MerklePatriciaRoot extends H256 {
}
/** @name QueryBspConfirmChunksToProveForFileError */
export interface QueryBspConfirmChunksToProveForFileError extends Enum {
    readonly isStorageRequestNotFound: boolean;
    readonly isInternalError: boolean;
    readonly type: 'StorageRequestNotFound' | 'InternalError';
}
/** @name QueryFileEarliestVolunteerBlockError */
export interface QueryFileEarliestVolunteerBlockError extends Enum {
    readonly isFailedToEncodeFingerprint: boolean;
    readonly isFailedToEncodeBsp: boolean;
    readonly isThresholdArithmeticError: boolean;
    readonly isStorageRequestNotFound: boolean;
    readonly isInternalError: boolean;
    readonly type: 'FailedToEncodeFingerprint' | 'FailedToEncodeBsp' | 'ThresholdArithmeticError' | 'StorageRequestNotFound' | 'InternalError';
}
/** @name SaveFileToDisk */
export interface SaveFileToDisk extends Enum {
    readonly isFileNotFound: boolean;
    readonly isSuccess: boolean;
    readonly asSuccess: FileMetadata;
    readonly isIncompleteFile: boolean;
    readonly asIncompleteFile: IncompleteFileStatus;
    readonly type: 'FileNotFound' | 'Success' | 'IncompleteFile';
}
/** @name StorageData */
export interface StorageData extends u32 {
}
export type PHANTOM_STORAGEHUBCLIENT = 'storagehubclient';
