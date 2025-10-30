import { CHUNK_SIZE, MAX_WASM_FINGERPRINT_BYTES, BATCH_SIZE_BYTES } from "./constants";

import { initWasm } from "./init.js";
import { FileMetadata, FileTrie } from "./wasm.js";
import { TypeRegistry } from "@polkadot/types";
import type { AccountId20, H256 } from "@polkadot/types/interfaces";

export class FileManager {
  constructor(
    private readonly file: {
      size: number;
      stream: () => ReadableStream<Uint8Array>;
    }
  ) {}

  private fingerprint?: H256;
  private fileKey?: H256;
  private fileBlob?: Blob;

  /**
   * Compute the file fingerprint (Merkle root)
   */
  async getFingerprint(): Promise<H256> {
    if (this.fingerprint) {
      return this.fingerprint;
    }

    await initWasm();

    if (this.file.size > MAX_WASM_FINGERPRINT_BYTES) {
      throw new Error(
        `File too large for WASM fingerprint calculation. size=${this.file.size}B limit=${MAX_WASM_FINGERPRINT_BYTES}B`
      );
    }

    const registry = new TypeRegistry();
    const trie = new FileTrie();

    const stream = this.file.stream();
    const reader = stream.getReader();

    // Fixed-size carry buffer for at most one partial chunk
    const remainder = new Uint8Array(CHUNK_SIZE);
    let remLen = 0;

    // Batch size (already aligned to CHUNK_SIZE)

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (!value?.length) continue;

        let offset = 0;

        // 1) Complete a pending partial 1 KiB chunk (preserves chunk order)
        if (remLen) {
          const needed = CHUNK_SIZE - remLen;
          const toCopy = Math.min(needed, value.length);
          remainder.set(value.subarray(0, toCopy), remLen);
          remLen += toCopy;
          offset += toCopy;
          if (remLen === CHUNK_SIZE) {
            trie.push_chunk(remainder.subarray(0, CHUNK_SIZE));
            remLen = 0;
          }
        }

        // 2) Push full 1 KiB windows from this read in at most two batched calls
        const bytesLeft = value.length - offset;
        const fullBytes = bytesLeft - (bytesLeft % CHUNK_SIZE);
        if (fullBytes) {
          const endFull = offset + fullBytes;
          const fullBatchesLen = endFull - offset - ((endFull - offset) % BATCH_SIZE_BYTES);
          if (fullBatchesLen) {
            trie.push_chunks_batched(value.subarray(offset, offset + fullBatchesLen));
            offset += fullBatchesLen;
          }
          if (offset < endFull) {
            trie.push_chunks_batched(value.subarray(offset, endFull));
            offset = endFull;
          }
        }

        // 3) Carry leftover (< 1 KiB) to next iteration
        const tail = value.length - offset;
        if (tail) {
          remainder.set(value.subarray(offset), 0);
          remLen = tail;
        }
      }

      // Flush any remaining partial chunk
      if (remLen) {
        trie.push_chunk(remainder.subarray(0, remLen));
        remLen = 0;
      }
    } finally {
      reader.releaseLock();
    }

    const rootHash = trie.get_root();
    const fingerprint = registry.createType("H256", rootHash) as H256;
    this.fingerprint = fingerprint;
    return fingerprint;
  }

  getFileSize(): number {
    return this.file.size;
  }

  /**
   * Compute the FileKey for this file.
   *
   * The caller must provide:
   *   • owner – 32-byte AccountId (Uint8Array or 0x-prefixed hex string)
   *   • bucketId – 32-byte BucketId (Uint8Array or 0x-prefixed hex string)
   *   • location – path string (encoded to bytes as-is)
   */
  async computeFileKey(owner: AccountId20, bucketId: H256, location: string): Promise<H256> {
    if (this.fileKey) {
      return this.fileKey;
    }

    const fp = await this.getFingerprint();

    const metadata = new FileMetadata(
      owner.toU8a(),
      bucketId.toU8a(),
      new TextEncoder().encode(location),
      BigInt(this.file.size),
      fp.toU8a()
    );

    const fileKey = metadata.getFileKey();
    const registry = new TypeRegistry();
    this.fileKey = registry.createType("H256", fileKey) as H256;
    return this.fileKey;
  }

  /**
   * Retrieve the file as a Blob. If not already available, this will
   * compute it by streaming the file (also computing and caching the fingerprint).
   */
  async getFileBlob(): Promise<Blob> {
    if (this.fileBlob) {
      return this.fileBlob;
    }
    const reader = this.file.stream().getReader();
    const parts: BlobPart[] = [];
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (value?.length) parts.push(value.slice());
      }
    } finally {
      reader.releaseLock();
    }
    this.fileBlob = new Blob(parts);
    return this.fileBlob;
  }
}
