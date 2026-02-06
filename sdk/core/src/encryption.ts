import { chacha20poly1305 } from "@noble/ciphers/chacha.js";

import { ENCRYPTION_CHUNK_SIZE } from "./constants.js";

import type { WalletClient } from "viem";
import type { Account } from "viem";
import { DEK, BaseNonce, IKM, Salt } from "./encryption/types.js";
import {
  createEncryptionHeader,
  readEncryptionHeader,
  type EncryptionHeaderParams
} from "./encryption/cbor.js";

const AEAD_TAG_SIZE_BYTES = 16;

/**
 * Minimal-copy buffering for Uint8Array streams.
 *
 * Keeps a queue of segments and supports taking exactly N bytes without
 * repeatedly concatenating buffers.
 */
class ByteQueue {
  private readonly segments: Uint8Array[] = [];
  private headOffset = 0;
  private bufferedBytes = 0;

  get length(): number {
    return this.bufferedBytes;
  }

  push(u8: Uint8Array | undefined): void {
    if (!u8?.length) return;
    this.segments.push(u8);
    this.bufferedBytes += u8.length;
  }

  take(n: number): Uint8Array {
    if (n === 0) return new Uint8Array();
    if (n > this.bufferedBytes) {
      throw new Error(`ByteQueue.take: underflow (need=${n}, have=${this.bufferedBytes})`);
    }

    const out = new Uint8Array(n);
    let outOffset = 0;

    while (outOffset < n) {
      const head = this.segments[0];
      if (!head) throw new Error("ByteQueue.take: internal empty queue");

      const available = head.length - this.headOffset;
      const toCopy = Math.min(available, n - outOffset);
      out.set(head.subarray(this.headOffset, this.headOffset + toCopy), outOffset);
      outOffset += toCopy;
      this.headOffset += toCopy;
      this.bufferedBytes -= toCopy;

      if (this.headOffset === head.length) {
        this.segments.shift();
        this.headOffset = 0;
      }
    }

    return out;
  }

  takeAll(): Uint8Array {
    return this.take(this.bufferedBytes);
  }
}

export type EncryptFileProgress = {
  bytesProcessed: number;
  chunkIndex: number;
};

type EncryptAndWriteChunkParams = {
  writer: WritableStreamDefaultWriter<Uint8Array>;
  dek: DEK;
  baseNonce: BaseNonce;
  chunkIndex: number;
  plaintext: Uint8Array;
  bytesProcessed: number;
  onProgress: ((p: EncryptFileProgress) => void) | undefined;
};

async function encryptAndWriteChunk({
  writer,
  dek,
  baseNonce,
  chunkIndex,
  plaintext,
  bytesProcessed,
  onProgress
}: EncryptAndWriteChunkParams): Promise<number> {
  // Nonce is derived from BaseNonce + chunkIndex.
  const nonce = baseNonce.getNonce(chunkIndex).unwrap();

  const cipher = chacha20poly1305(dek, nonce);
  const encrypted = cipher.encrypt(plaintext);

  // Respect backpressure.
  await writer.ready;
  await writer.write(encrypted);

  const nextBytesProcessed = bytesProcessed + plaintext.length;
  onProgress?.({ bytesProcessed: nextBytesProcessed, chunkIndex });
  return nextBytesProcessed;
}

export type EncryptFileParams = {
  input: ReadableStream<Uint8Array>;
  output: WritableStream<Uint8Array>;
  dek: DEK;
  baseNonce: BaseNonce;
  /**
   * CBOR header fields written before any ciphertext.
   *
   * This allows the SDK to later parse the file and know *how* the IKM was obtained
   * and what salt was used, so it can deterministically re-derive DEK/BaseNonce.
   */
  header: EncryptionHeaderParams;
  onProgress?: (p: EncryptFileProgress) => void;
  /**
   * Plaintext chunk size used for encryption framing.
   * Defaults to `ENCRYPTION_CHUNK_SIZE` (16 MiB).
   */
  chunkSizeBytes?: number;
};

/**
 * Encrypt a byte stream into a byte stream using chunked ChaCha20-Poly1305.
 *
 * - Plaintext is framed into fixed-size chunks (except the last chunk).
 * - Each chunk uses a distinct nonce from `baseNonce.getNonce(chunkIndex)`.
 */
export async function encryptFile({
  input,
  output,
  dek,
  baseNonce,
  header,
  onProgress,
  chunkSizeBytes
}: EncryptFileParams): Promise<void> {
  // -------- parameters / defaults --------
  const chunkSize = chunkSizeBytes ?? ENCRYPTION_CHUNK_SIZE;
  if (!Number.isSafeInteger(chunkSize) || chunkSize <= 0) {
    throw new Error(`encryptFile: invalid chunkSizeBytes=${chunkSize}`);
  }

  const reader = input.getReader();
  const writer = output.getWriter();

  let chunkIndex = 0;
  let bytesProcessed = 0;

  // Buffer stream reads into fixed-size plaintext chunks.
  const buffer = new ByteQueue();

  let ok = false;
  try {
    // -------- header --------
    // Header is written in plaintext before any ciphertext chunks.
    const headerBytes = createEncryptionHeader(header);
    await writer.ready;
    await writer.write(headerBytes);

    // -------- body (chunked encryption) --------
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer.push(value);

      // Encrypt as many full plaintext chunks as we have buffered.
      while (buffer.length >= chunkSize) {
        const plaintextChunk = buffer.take(chunkSize);
        bytesProcessed = await encryptAndWriteChunk({
          writer,
          dek,
          baseNonce,
          chunkIndex,
          plaintext: plaintextChunk,
          bytesProcessed,
          onProgress
        });
        chunkIndex++;
      }
    }

    // Final partial chunk (may be empty).
    if (buffer.length > 0) {
      const lastPlaintext = buffer.takeAll();
      bytesProcessed = await encryptAndWriteChunk({
        writer,
        dek,
        baseNonce,
        chunkIndex,
        plaintext: lastPlaintext,
        bytesProcessed,
        onProgress
      });
    }

    ok = true;
  } catch (err) {
    // Abort/cancel on failure so downstream knows this is not a clean EOF.
    try {
      await writer.abort(err);
    } catch {
      // ignore
    }
    try {
      await reader.cancel(err);
    } catch {
      // ignore
    }
    throw err;
  } finally {
    if (ok) {
      await writer.close();
    }
    reader.releaseLock();
  }
}

type EncryptionKeySource =
  | { kind: "password"; password: string }
  | {
      kind: "signature";
      walletClient: WalletClient;
      account: Account | `0x${string}`;
      message: string;
      challenge: Uint8Array;
    };

export type GeneratedEncryptionKey = {
  dek: DEK;
  baseNonce: BaseNonce;
  header: EncryptionHeaderParams;
};

function randomSaltBytes(length = 32): Uint8Array {
  const cryptoObj = globalThis.crypto as Crypto | undefined;
  if (!cryptoObj?.getRandomValues) {
    throw new Error("crypto.getRandomValues is not available to generate salt");
  }
  const salt = new Uint8Array(length);
  cryptoObj.getRandomValues(salt);
  return salt;
}

export async function generateEncryptionKey(
  source: EncryptionKeySource
): Promise<GeneratedEncryptionKey> {
  // Public, random salt stored in the CBOR header.
  const saltBytes = randomSaltBytes(32);
  const salt = Salt.fromBytes(saltBytes).unwrap();
  const header: EncryptionHeaderParams =
    source.kind === "signature"
      ? {
          ikm: source.kind,
          salt: saltBytes,
          challenge: source.challenge
        }
      : {
          ikm: source.kind,
          salt: saltBytes
        };

  switch (source.kind) {
    case "password": {
      const ikm = IKM.fromPassword(source.password).unwrap();
      const dek = DEK.derive(ikm, salt).unwrap();
      const baseNonce = BaseNonce.derive(ikm, salt).unwrap();

      return {
        dek,
        baseNonce,
        header
      };
    }
    case "signature": {
      const signature = await source.walletClient.signMessage({
        account: source.account,
        message: source.message
      });

      const ikm = IKM.fromSignature(signature).unwrap();
      const dek = DEK.derive(ikm, salt).unwrap();
      const baseNonce = BaseNonce.derive(ikm, salt).unwrap();

      return {
        dek,
        baseNonce,
        header
      };
    }
  }
  throw new Error("Unknown EncryptionKeySource");
}

export type DecryptFileProgress = {
  bytesProcessed: number;
  chunkIndex: number;
};

export type DecryptFileParams = {
  input: ReadableStream<Uint8Array>;
  output: WritableStream<Uint8Array>;
  /**
   * Provide the Input Key Material based on the header.
   *
   * - If header.ikm === "password": prompt for password and return `IKM.fromPassword(password).unwrap()`
   * - If header.ikm === "signature": produce the file-specific signature again and return `IKM.fromSignature(signature).unwrap()`
   */
  getIkm: (header: EncryptionHeaderParams) => Promise<IKM>;
  onProgress?: (p: DecryptFileProgress) => void;
  /**
   * Plaintext chunk size used during encryption framing.
   * Must match the value used during encryption. Defaults to `ENCRYPTION_CHUNK_SIZE` (16 MiB).
   */
  chunkSizeBytes?: number;
};

async function readHeaderFromStream(
  reader: ReadableStreamDefaultReader<Uint8Array>
): Promise<{ header: EncryptionHeaderParams; headerLength: number; remainder: Uint8Array }> {
  // Layout: [ magic (3) ][ u32be header_len ][ cbor_header ]
  const MAGIC_PLUS_LEN = 7;
  const MAX_HEADER_LEN_BYTES = 64 * 1024;

  const chunks: Uint8Array[] = [];
  let total = 0;

  const pull = async (minBytes: number) => {
    while (total < minBytes) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!value?.length) continue;
      chunks.push(value);
      total += value.length;
    }
  };

  await pull(MAGIC_PLUS_LEN);
  if (total < MAGIC_PLUS_LEN) {
    throw new Error("decryptFile: input too short to contain header");
  }

  // Flatten what we have so far to read headerLen (offset 3..6).
  const prefix = new Uint8Array(total);
  {
    let off = 0;
    for (const c of chunks) {
      prefix.set(c, off);
      off += c.length;
    }
  }

  const headerLen = new DataView(prefix.buffer, prefix.byteOffset, prefix.byteLength).getUint32(
    3,
    false
  );
  if (headerLen > MAX_HEADER_LEN_BYTES) {
    throw new Error(`decryptFile: header too large (${headerLen} bytes)`);
  }

  const totalHeaderBytes = MAGIC_PLUS_LEN + headerLen;
  await pull(totalHeaderBytes);
  if (total < totalHeaderBytes) {
    throw new Error("decryptFile: input truncated while reading header");
  }

  const buf = new Uint8Array(total);
  {
    let off = 0;
    for (const c of chunks) {
      buf.set(c, off);
      off += c.length;
    }
  }

  const { header, headerLength } = readEncryptionHeader(buf);
  const remainder = buf.subarray(headerLength);
  return { header, headerLength, remainder };
}

type DecryptAndWriteChunkParams = {
  writer: WritableStreamDefaultWriter<Uint8Array>;
  dek: DEK;
  baseNonce: BaseNonce;
  chunkIndex: number;
  ciphertext: Uint8Array;
  bytesProcessed: number;
  onProgress: ((p: DecryptFileProgress) => void) | undefined;
};

async function decryptAndWriteChunk({
  writer,
  dek,
  baseNonce,
  chunkIndex,
  ciphertext,
  bytesProcessed,
  onProgress
}: DecryptAndWriteChunkParams): Promise<number> {
  const nonce = baseNonce.getNonce(chunkIndex).unwrap();
  const cipher = chacha20poly1305(dek, nonce);
  const plaintext = cipher.decrypt(ciphertext);

  await writer.ready;
  await writer.write(plaintext);

  const nextBytesProcessed = bytesProcessed + plaintext.length;
  onProgress?.({ bytesProcessed: nextBytesProcessed, chunkIndex });
  return nextBytesProcessed;
}

/**
 * Decrypt a byte stream produced by `encryptFile()`.
 *
 * This reads and validates the CBOR header, re-derives keys using the header salt
 * and a caller-provided IKM source, then chunk-decrypts the ciphertext stream.
 */
export async function decryptFile({
  input,
  output,
  getIkm,
  onProgress,
  chunkSizeBytes
}: DecryptFileParams): Promise<void> {
  const chunkSize = chunkSizeBytes ?? ENCRYPTION_CHUNK_SIZE;
  if (!Number.isSafeInteger(chunkSize) || chunkSize <= 0) {
    throw new Error(`decryptFile: invalid chunkSizeBytes=${chunkSize}`);
  }

  const reader = input.getReader();
  const writer = output.getWriter();

  let ok = false;
  try {
    // -------- header --------
    const { header, remainder } = await readHeaderFromStream(reader);

    // -------- derive keys --------
    const ikm = await getIkm(header);
    const salt = Salt.fromBytes(header.salt).unwrap();
    const dek = DEK.derive(ikm, salt).unwrap();
    const baseNonce = BaseNonce.derive(ikm, salt).unwrap();

    // -------- body (chunked decryption) --------
    const ciphertextBuf = new ByteQueue();
    ciphertextBuf.push(remainder);

    const fullCipherChunkSize = chunkSize + AEAD_TAG_SIZE_BYTES;
    let chunkIndex = 0;
    let bytesProcessed = 0;

    while (true) {
      while (ciphertextBuf.length >= fullCipherChunkSize) {
        const ciphertextChunk = ciphertextBuf.take(fullCipherChunkSize);
        bytesProcessed = await decryptAndWriteChunk({
          writer,
          dek,
          baseNonce,
          chunkIndex,
          ciphertext: ciphertextChunk,
          bytesProcessed,
          onProgress
        });
        chunkIndex++;
      }

      const { done, value } = await reader.read();
      if (done) break;
      ciphertextBuf.push(value);
    }

    // Final chunk (if any) must include at least an AEAD tag.
    if (ciphertextBuf.length > 0) {
      if (ciphertextBuf.length <= AEAD_TAG_SIZE_BYTES) {
        throw new Error("decryptFile: truncated final chunk (missing AEAD tag)");
      }
      const lastCiphertext = ciphertextBuf.takeAll();
      bytesProcessed = await decryptAndWriteChunk({
        writer,
        dek,
        baseNonce,
        chunkIndex,
        ciphertext: lastCiphertext,
        bytesProcessed,
        onProgress
      });
    }

    ok = true;
  } catch (err) {
    try {
      await writer.abort(err);
    } catch {
      // ignore
    }
    try {
      await reader.cancel(err);
    } catch {
      // ignore
    }
    throw err;
  } finally {
    if (ok) {
      await writer.close();
    }
    reader.releaseLock();
  }
}
