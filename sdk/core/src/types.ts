export interface FileInfo {
  /** Unique file identifier (32-byte hex string) */
  fileKey: `0x${string}`;
  /** Root of the file content trie (32-byte hex string) */
  fingerprint: `0x${string}`;
  /** Identifier of the bucket that contains the file (32-byte hex string) */
  bucketId: `0x${string}`;
  /** File location/path within the bucket */
  location: string;
  /** File size in bytes (using bigint for blockchain compatibility) */
  size: bigint;
  /**
   * Block hash where the file was created (32-byte hex string).
   * Contains the block hash where the NewStorageRequest event was emitted.
   */
  blockHash: `0x${string}`;
  /**
   * EVM transaction hash that created this file (32-byte hex string).
   * Only present for files created via EVM transactions on EVM-enabled runtimes.
   * Will be undefined for files created via native Substrate extrinsics.
   */
  txHash?: `0x${string}`;
}
