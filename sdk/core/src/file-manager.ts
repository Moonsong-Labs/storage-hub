import { FileTrie, FileMetadata } from '@storagehub/wasm';
import { TypeRegistry } from '@polkadot/types';
import type { AccountId, H256 } from '@polkadot/types/interfaces';
import { CHUNK_SIZE } from './constants';

export class FileManager {
  constructor(private readonly file: { size: number; stream: () => ReadableStream<Uint8Array> }) {}

  private fingerprint?: H256;
  private fileKey?: H256;

  /**
   * Stream the file's contents, feed every 1 kB chunk into a new FileTrie, and
   * return the resulting Merkle root.
   */
  async getFingerprint(): Promise<H256> {
    if (this.fingerprint) {
      return this.fingerprint;
    }

    const registry = new TypeRegistry();
    const trie = new FileTrie();

    const stream = this.file.stream();
    const reader = stream.getReader();
    let buffer = new Uint8Array();
    let bufferOffset = 0;

    try {
      while (true) {
        // The default chunk size for a Blob stream is typically 64 KiB
        const { done, value } = await reader.read();
        if (done) break;

        if (value) {
          const newBuffer = new Uint8Array(buffer.length - bufferOffset + value.length);
          newBuffer.set(buffer.subarray(bufferOffset));
          newBuffer.set(value, buffer.length - bufferOffset);
          buffer = newBuffer;
          bufferOffset = 0;

          while (buffer.length - bufferOffset >= CHUNK_SIZE) {
            const chunk = buffer.subarray(bufferOffset, bufferOffset + CHUNK_SIZE);
            trie.push_chunk(chunk);
            bufferOffset += CHUNK_SIZE;
          }
        }
      }

      if (buffer.length - bufferOffset > 0) {
        trie.push_chunk(buffer.subarray(bufferOffset));
      }
    } finally {
      reader.releaseLock();
    }

    const rootHash = trie.get_root();
    const fingerprint = registry.createType('H256', rootHash) as H256;

    this.fingerprint = fingerprint;
    return fingerprint;
  }

  /**
   * Compute the FileKey for this file.
   *
   * The caller must provide:
   *   • owner – 32-byte AccountId (Uint8Array or 0x-prefixed hex string)
   *   • bucketId – 32-byte BucketId (Uint8Array or 0x-prefixed hex string)
   *   • location – path string (encoded to bytes as-is)
   */
  async computeFileKey(owner: AccountId, bucketId: H256, location: string): Promise<H256> {
    if (this.fileKey) {
      return this.fileKey;
    }

    // Ensure fingerprint is computed
    const fp = this.fingerprint ?? (await this.getFingerprint());

    const metadata = new FileMetadata(
      owner.toU8a(),
      bucketId.toU8a(),
      new TextEncoder().encode(location),
      this.file.size,
      fp.toU8a(),
    );

    const fileKey = metadata.getFileKey();
    const registry = new TypeRegistry();
    this.fileKey = registry.createType('H256', fileKey) as H256;
    return this.fileKey;
  }
}
