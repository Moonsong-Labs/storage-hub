import { statSync, existsSync } from 'node:fs';
import { FileTrie, FileMetadata } from '@storagehub/wasm';
import { CHUNK_SIZE } from './constants';
import { TypeRegistry } from '@polkadot/types';
import type { AccountId, H256 } from '@polkadot/types/interfaces';

import { open } from 'fs/promises';

export class FileManager {
  constructor(private readonly filePath: string) {
    if (!existsSync(this.filePath)) {
      throw new Error(`File not found: ${this.filePath}`);
    }

    this.fileSize = statSync(this.filePath).size;
  }

  private fingerprint?: H256;
  private fileKey?: H256;
  private fileSize: number;

  /**
   * Stream the file from disk, feed every 1 KiB block into a new FileTrie and
   * return the resulting Merkle root.
   */
  async getFingerprint(): Promise<H256> {
    if (this.fingerprint) {
      return this.fingerprint;
    }

    const registry = new TypeRegistry();
    const trie = new FileTrie();

    const buffer = new Uint8Array(CHUNK_SIZE);

    const fileHandle = await open(this.filePath, 'r');

    try {
      while (true) {
        const { bytesRead } = await fileHandle.read(buffer, 0, CHUNK_SIZE, null);
        if (bytesRead === 0) break;

        trie.push_chunk(buffer.subarray(0, bytesRead));
      }
    } finally {
      await fileHandle.close();
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
      this.fileSize,
      fp.toU8a(),
    );

    const fileKey = metadata.getFileKey();
    const registry = new TypeRegistry();
    this.fileKey = registry.createType('H256', fileKey) as H256;
    return this.fileKey;
  }
}
