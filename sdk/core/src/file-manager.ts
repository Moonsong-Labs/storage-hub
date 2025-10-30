import { CHUNK_SIZE, MAX_WASM_FINGERPRINT_BYTES } from "./constants";
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
   * Stream the file's contents, feed every 1 kB chunk into a new FileTrie, and
   * return the resulting Merkle root.
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
    // ---
    // Streaming fingerprint algorithm
    // We want to feed the MerkleTrie with **fixed-size** `${CHUNK_SIZE}`-byte chunks
    // but a `ReadableStream` gives us arbitrarily-sized `Uint8Array`s (default ≈64 KiB).
    //
    // Strategy
    // 1. Keep an in-memory sliding buffer (`buffer`).
    // 2. `bufferOffset` marks how much of that buffer has already been
    //    consumed (pushed to the trie).
    // 3. On each `reader.read()` we append the newly-read bytes after the
    //    unconsumed tail.  Then, while we still have ≥ CHUNK_SIZE bytes
    //    available, cut a `${CHUNK_SIZE}`-byte window and push it into the trie.
    // 4. Any leftover ( < CHUNK_SIZE ) stays in `buffer` to be prefixed by
    //    the next read.
    // ---
    const reader = stream.getReader();
    const blobParts: BlobPart[] = [];
    let buffer = new Uint8Array();
    let bufferOffset = 0;

    try {
      while (true) {
        // ── Step-1: pull next blob fragment (≈64 KiB) from the stream
        const { done, value } = await reader.read();
        if (done) break; // EOF ⇒ exit outer loop

        if (value?.length) {
          // Accumulate raw bytes so we can build a Blob after streaming
          blobParts.push(value.slice());
          /*
           * ── Step-2: concatenate the newly-read bytes **after** any leftover
           *            bytes we still haven’t consumed (bufferOffset marks the
           *            start of that tail).  We create a fresh Uint8Array to
           *            avoid costly shifting of the existing buffer.
           */
          const unreadTail = buffer.subarray(bufferOffset);
          const newBuffer = new Uint8Array(unreadTail.length + value.length);
          newBuffer.set(unreadTail, 0);
          newBuffer.set(value, unreadTail.length);
          buffer = newBuffer;
          bufferOffset = 0;

          /*
           * ── Step-3: while the sliding-window holds at least one full
           *            CHUNK_SIZE-byte block, slice it out and push it into the
           *            trie.  We may loop multiple times if the stream chunk was
           *            very large.
           */
          while (buffer.length - bufferOffset >= CHUNK_SIZE) {
            const chunk = buffer.subarray(bufferOffset, bufferOffset + CHUNK_SIZE);
            trie.push_chunk(chunk);
            bufferOffset += CHUNK_SIZE;
          }
        }
      }

      // ── Step-4: push the leftover bytes (< CHUNK_SIZE)
      if (buffer.length - bufferOffset > 0) {
        trie.push_chunk(buffer.subarray(bufferOffset));
      }
    } finally {
      reader.releaseLock();
    }

    // Retrieve Merkle root from the trie and cache it
    const rootHash = trie.get_root();
    const fingerprint = registry.createType("H256", rootHash) as H256;

    this.fingerprint = fingerprint;
    // Build and cache the Blob for later reuse
    this.fileBlob = new Blob(blobParts);
    return fingerprint;
  }

  /**
   * Zero-copy-ish fast fingerprint: avoids Blob creation and large buffer copies.
   * Only copies at most one partial chunk per read, and batches contiguous
   * full 1 KiB windows directly from the read buffer.
   */
  async getFingerprintFastZeroCopy(options?: { batchSizeBytes?: number }): Promise<H256> {
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

    // Fixed-size remainder buffer for at most one partial chunk
    const remainder = new Uint8Array(CHUNK_SIZE);
    let remLen = 0;

    // Batch target rounded down to a multiple of CHUNK_SIZE
    const targetBatchSize =
      Math.max(1, Math.floor((options?.batchSizeBytes ?? 10 * 1024 * 1024) / CHUNK_SIZE)) *
      CHUNK_SIZE;

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (!value || value.length === 0) continue;

        let offset = 0;
        // Complete a pending chunk first, if any
        if (remLen > 0) {
          const need = CHUNK_SIZE - remLen;
          const take = Math.min(need, value.length);
          remainder.set(value.subarray(0, take), remLen);
          remLen += take;
          offset += take;
          if (remLen === CHUNK_SIZE) {
            trie.push_chunk(remainder.subarray(0, CHUNK_SIZE));
            remLen = 0;
          }
        }

        // Push full windows directly from the read buffer in large batches
        const remaining = value.length - offset;
        let pushable = Math.floor(remaining / CHUNK_SIZE) * CHUNK_SIZE;
        // Push as many full batches as possible
        while (pushable >= targetBatchSize) {
          const end = offset + targetBatchSize;
          trie.push_chunks_batched(value.subarray(offset, end));
          offset = end;
          const rest = value.length - offset;
          pushable = Math.floor(rest / CHUNK_SIZE) * CHUNK_SIZE;
        }
        if (pushable > 0) {
          const end = offset + pushable;
          trie.push_chunks_batched(value.subarray(offset, end));
          offset = end;
        }

        // Store small leftover (< CHUNK_SIZE) for the next iteration
        const left = value.length - offset;
        if (left > 0) {
          remainder.set(value.subarray(offset, value.length), 0);
          remLen = left;
        }
      }

      // Flush any remaining partial chunk
      if (remLen > 0) {
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
    await this.getFingerprint();
    if (!this.fileBlob) {
      throw new Error("Failed to create file blob during fingerprint computation.");
    }
    return this.fileBlob;
  }
}
