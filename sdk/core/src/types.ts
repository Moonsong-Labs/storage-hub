export interface FileInfo {
  /** Unique file identifier (32-byte hex string) */
  fileKey: `0x${string}`;
  /** File content hash/fingerprint (32-byte hex string) */
  fingerprint: `0x${string}`;
  /** Bucket identifier containing the file (32-byte hex string) */
  bucketId: `0x${string}`;
  /** File location/path within the bucket */
  location: string;
  /** File size in bytes (using bigint for blockchain compatibility) */
  size: bigint;
}
