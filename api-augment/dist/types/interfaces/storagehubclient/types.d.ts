import type { Bytes, Enum, Struct, U8aFixed, u64 } from '@polkadot/types-codec';
/** @name FileMetadata */
export interface FileMetadata extends Struct {
    readonly owner: Bytes;
    readonly bucket_id: Bytes;
    readonly location: Bytes;
    readonly file_size: u64;
    readonly fingerprint: U8aFixed;
}
/** @name IncompleteFileStatus */
export interface IncompleteFileStatus extends Struct {
    readonly file_metadata: FileMetadata;
    readonly stored_chunks: u64;
    readonly total_chunks: u64;
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
export type PHANTOM_STORAGEHUBCLIENT = 'storagehubclient';
