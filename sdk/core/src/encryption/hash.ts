import { blake2s } from "@noble/hashes/blake2.js";
import { bytesToHex } from "@noble/hashes/utils.js";

/**
 * Compute a BLAKE2s-256 hash of a web ReadableStream.
 *
 * Returns a 0x-prefixed 32-byte hex digest.
 */
export async function blake2s_256(stream: ReadableStream<Uint8Array>): Promise<`0x${string}`> {
  const hasher = blake2s.create({ dkLen: 32 });

  const reader = stream.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!value?.length) continue;
      hasher.update(value);
    }
  } finally {
    reader.releaseLock();
  }

  const digest = hasher.digest();
  return `0x${bytesToHex(digest)}` as `0x${string}`;
}
