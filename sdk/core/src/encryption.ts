import { chacha20poly1305 } from "@noble/ciphers/chacha.js";
import { sha256 } from "@noble/hashes/sha2.js";

import {
  ENCRYPTION_CHUNK_SIZE,
  SALT_SIZE,
  AEAD_TAG_SIZE_BYTES,
  HEADER_HASH_SIZE_BYTES,
  CHUNK_AAD_SIZE_BYTES,
  CHUNK_AAD_VERSION,
  CHUNK_AAD_KIND_DATA,
  CHUNK_AAD_KIND_COMMIT,
  COMMIT_MAGIC,
  COMMIT_PLAINTEXT_SIZE_BYTES,
  COMMIT_CIPHERTEXT_SIZE_BYTES,
  MAX_SAFE_INTEGER_BIGINT
} from "./encryption/consts.js";

import type { WalletClient } from "viem";
import type { Account } from "viem";
import { DEK, BaseNonce, IKM, Salt } from "./encryption/types.js";
import { createEncryptionHeader, readEncryptionHeader } from "./encryption/cbor.js";
import type { EncryptionHeaderParams } from "./encryption/header.js";

function createChunkAAD(headerHash: Uint8Array, chunkIndex: number, kind: number): Uint8Array {
  if (headerHash.length !== HEADER_HASH_SIZE_BYTES) {
    throw new Error(
      `createChunkAAD: headerHash must be ${HEADER_HASH_SIZE_BYTES} bytes (got ${headerHash.length})`
    );
  }
  if (!Number.isSafeInteger(chunkIndex) || chunkIndex < 0) {
    throw new Error(`createChunkAAD: invalid chunkIndex=${chunkIndex}`);
  }
  if (kind !== CHUNK_AAD_KIND_DATA && kind !== CHUNK_AAD_KIND_COMMIT) {
    throw new Error(`createChunkAAD: invalid kind=${kind}`);
  }

  const aad = new Uint8Array(CHUNK_AAD_SIZE_BYTES);
  aad[0] = CHUNK_AAD_VERSION;
  aad[1] = kind;
  new DataView(aad.buffer, aad.byteOffset, aad.byteLength).setBigUint64(2, BigInt(chunkIndex), false);
  aad.set(headerHash, 10);
  return aad;
}

function createCommitPayload(totalPlaintextBytes: number, totalChunkCount: number): Uint8Array {
  if (!Number.isSafeInteger(totalPlaintextBytes) || totalPlaintextBytes < 0) {
    throw new Error(`createCommitPayload: invalid totalPlaintextBytes=${totalPlaintextBytes}`);
  }
  if (!Number.isSafeInteger(totalChunkCount) || totalChunkCount < 0) {
    throw new Error(`createCommitPayload: invalid totalChunkCount=${totalChunkCount}`);
  }

  const payload = new Uint8Array(COMMIT_PLAINTEXT_SIZE_BYTES);
  payload.set(COMMIT_MAGIC, 0);
  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  view.setBigUint64(COMMIT_MAGIC.length, BigInt(totalPlaintextBytes), false);
  view.setBigUint64(COMMIT_MAGIC.length + 8, BigInt(totalChunkCount), false);
  return payload;
}

function parseCommitPayload(payload: Uint8Array): { totalPlaintextBytes: number; totalChunkCount: number } {
  if (payload.length !== COMMIT_PLAINTEXT_SIZE_BYTES) {
    throw new Error(
      `decryptFile: invalid commit payload size (expected ${COMMIT_PLAINTEXT_SIZE_BYTES}, got ${payload.length})`
    );
  }

  for (let i = 0; i < COMMIT_MAGIC.length; i++) {
    if (payload[i] !== COMMIT_MAGIC[i]) {
      throw new Error("decryptFile: invalid commit payload magic");
    }
  }

  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  const totalPlaintextBytesBig = view.getBigUint64(COMMIT_MAGIC.length, false);
  const totalChunkCountBig = view.getBigUint64(COMMIT_MAGIC.length + 8, false);
  if (totalPlaintextBytesBig > MAX_SAFE_INTEGER_BIGINT) {
    throw new Error("decryptFile: commit total_plaintext_size exceeds MAX_SAFE_INTEGER");
  }
  if (totalChunkCountBig > MAX_SAFE_INTEGER_BIGINT) {
    throw new Error("decryptFile: commit total_chunk_count exceeds MAX_SAFE_INTEGER");
  }

  return {
    totalPlaintextBytes: Number(totalPlaintextBytesBig),
    totalChunkCount: Number(totalChunkCountBig)
  };
}

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
  headerHash: Uint8Array;
  chunkIndex: number;
  plaintext: Uint8Array;
  bytesProcessed: number;
  onProgress: ((p: EncryptFileProgress) => void) | undefined;
};

async function encryptAndWriteChunk({
  writer,
  dek,
  baseNonce,
  headerHash,
  chunkIndex,
  plaintext,
  bytesProcessed,
  onProgress
}: EncryptAndWriteChunkParams): Promise<number> {
  // Nonce is derived from BaseNonce + chunkIndex.
  const nonce = baseNonce.getNonce(chunkIndex).unwrap();
  const aad = createChunkAAD(headerHash, chunkIndex, CHUNK_AAD_KIND_DATA);
  const cipher = chacha20poly1305(dek, nonce, aad);
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
   * This allows the SDK to later parse the file and know *how* the IKM was obtained,
   * what salts were used, and what chunk size was used, so it can deterministically
   * re-derive DEK/BaseNonce and decrypt with the correct framing.
   */
  header: EncryptionHeaderParams;
  onProgress?: (p: EncryptFileProgress) => void;
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
  onProgress
}: EncryptFileParams): Promise<void> {
  const chunkSize = header.chunk_size;
  if (!Number.isSafeInteger(chunkSize) || chunkSize <= 0) {
    throw new Error(`encryptFile: invalid header chunk_size=${chunkSize}`);
  }

  const reader = input.getReader();
  const writer = output.getWriter();

  let chunkIndex = 0;
  let totalChunkCount = 0;
  let bytesProcessed = 0;

  // Buffer stream reads into fixed-size plaintext chunks.
  const buffer = new ByteQueue();

  let ok = false;
  try {
    // -------- header --------
    // Header is written in plaintext before any ciphertext chunks.
    const headerBytes = createEncryptionHeader(header);
    const headerHash = sha256(headerBytes);
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
          headerHash,
          chunkIndex,
          plaintext: plaintextChunk,
          bytesProcessed,
          onProgress
        });
        chunkIndex++;
        totalChunkCount++;
      }
    }

    // Final partial chunk (may be empty).
    if (buffer.length > 0) {
      const lastPlaintext = buffer.takeAll();
      bytesProcessed = await encryptAndWriteChunk({
        writer,
        dek,
        baseNonce,
        headerHash,
        chunkIndex,
        plaintext: lastPlaintext,
        bytesProcessed,
        onProgress
      });
      chunkIndex++;
      totalChunkCount++;
    }

    // Commit trailer: authenticated file totals to detect full-chunk truncation/reordering.
    const commitPayload = createCommitPayload(bytesProcessed, totalChunkCount);
    const commitNonce = baseNonce.getNonce(chunkIndex).unwrap();
    const commitAad = createChunkAAD(headerHash, chunkIndex, CHUNK_AAD_KIND_COMMIT);
    const commitCipher = chacha20poly1305(dek, commitNonce, commitAad).encrypt(commitPayload);
    await writer.ready;
    await writer.write(commitCipher);

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
      createMessage: (ikm_salt: Salt) => string;
    };

export type GeneratedEncryptionKey = {
  dek: DEK;
  baseNonce: BaseNonce;
  header: EncryptionHeaderParams;
};

function randomSaltBytes(length = SALT_SIZE): Uint8Array {
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
  // Public, random DEK salt stored in the CBOR header.
  const dekSaltBytes = randomSaltBytes(SALT_SIZE);
  const dekSalt = Salt.fromBytes(dekSaltBytes).unwrap();
  const ikmSalt = Salt.fromBytes(randomSaltBytes(SALT_SIZE)).unwrap();
  const header: EncryptionHeaderParams = {
    ikm: source.kind,
    dek_salt: dekSalt,
    ikm_salt: ikmSalt,
    // Encryption chunk size is currently fixed to 16 MiB, but because it is encoded
    // in the file header, this value can change in future versions without a protocol break.
    chunk_size: ENCRYPTION_CHUNK_SIZE
  };

  switch (source.kind) {
    case "password": {
      const ikm = IKM.fromPassword(source.password, ikmSalt).unwrap();
      const dek = DEK.derive(ikm, dekSalt).unwrap();
      const baseNonce = BaseNonce.derive(ikm, dekSalt).unwrap();

      return {
        dek,
        baseNonce,
        header
      };
    }
    case "signature": {
      const message = source.createMessage(ikmSalt);
      const signature = await source.walletClient.signMessage({
        account: source.account,
        message
      });

      const ikm = IKM.fromSignature(signature).unwrap();
      const dek = DEK.derive(ikm, dekSalt).unwrap();
      const baseNonce = BaseNonce.derive(ikm, dekSalt).unwrap();

      return {
        dek,
        baseNonce,
        header
      };
    }
  }

  const _exhaustive: never = source;
  throw new Error(
    `Unknown EncryptionKeySource kind: ${(source as { kind?: string }).kind ?? "undefined"}`
  );
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
   * - If header.ikm === "password": prompt for password and return
   *   `IKM.fromPassword(password, header.ikm_salt).unwrap()`
   * - If header.ikm === "signature": produce the file-specific signature again and return
   *   `IKM.fromSignature(signature).unwrap()`
   */
  getIkm: (header: EncryptionHeaderParams) => Promise<IKM>;
  onProgress?: (p: DecryptFileProgress) => void;
};

async function readHeaderFromStream(
  reader: ReadableStreamDefaultReader<Uint8Array>
): Promise<{
  header: EncryptionHeaderParams;
  headerLength: number;
  headerBytes: Uint8Array;
  remainder: Uint8Array;
}> {
  // Layout: [ magic (3) ][ u32be header_len ][ cbor_header ]
  const MAGIC_PLUS_LEN = 7;
  const MAX_HEADER_LEN_BYTES = 64 * 1024;

  const queue = new ByteQueue();

  const pull = async (minBytes: number) => {
    while (queue.length < minBytes) {
      const { done, value } = await reader.read();
      if (done) break;
      queue.push(value);
    }
  };

  // Phase 1: read fixed header prefix to obtain CBOR header length.
  await pull(MAGIC_PLUS_LEN);
  if (queue.length < MAGIC_PLUS_LEN) {
    throw new Error("decryptFile: input too short to contain header");
  }

  const prefix = queue.take(MAGIC_PLUS_LEN);
  const headerLen = new DataView(prefix.buffer, prefix.byteOffset, prefix.byteLength).getUint32(
    3,
    false
  );
  if (headerLen > MAX_HEADER_LEN_BYTES) {
    throw new Error(`decryptFile: header too large (${headerLen} bytes)`);
  }

  // Phase 2: read exactly the CBOR header payload bytes.
  await pull(headerLen);
  if (queue.length < headerLen) {
    throw new Error("decryptFile: input truncated while reading header");
  }
  const cborHeaderBytes = queue.take(headerLen);

  // Phase 3: decode the full header and preserve already-read body bytes.
  const fullHeader = new Uint8Array(MAGIC_PLUS_LEN + headerLen);
  fullHeader.set(prefix, 0);
  fullHeader.set(cborHeaderBytes, MAGIC_PLUS_LEN);

  const { header, headerLength } = readEncryptionHeader(fullHeader);
  const remainder = queue.takeAll();
  return { header, headerLength, headerBytes: fullHeader, remainder };
}

type DecryptAndWriteChunkParams = {
  writer: WritableStreamDefaultWriter<Uint8Array>;
  dek: DEK;
  baseNonce: BaseNonce;
  headerHash: Uint8Array;
  chunkIndex: number;
  ciphertext: Uint8Array;
  bytesProcessed: number;
  onProgress: ((p: DecryptFileProgress) => void) | undefined;
};

async function decryptAndWriteChunk({
  writer,
  dek,
  baseNonce,
  headerHash,
  chunkIndex,
  ciphertext,
  bytesProcessed,
  onProgress
}: DecryptAndWriteChunkParams): Promise<number> {
  const nonce = baseNonce.getNonce(chunkIndex).unwrap();
  const aad = createChunkAAD(headerHash, chunkIndex, CHUNK_AAD_KIND_DATA);
  const cipher = chacha20poly1305(dek, nonce, aad);
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
 * This reads and validates the CBOR header, re-derives keys using `header.dek_salt`
 * and a caller-provided IKM source, then chunk-decrypts the ciphertext stream.
 */
export async function decryptFile({
  input,
  output,
  getIkm,
  onProgress
}: DecryptFileParams): Promise<void> {
  const reader = input.getReader();
  const writer = output.getWriter();

  let ok = false;
  try {
    // -------- header --------
    const { header, headerBytes, remainder } = await readHeaderFromStream(reader);
    const chunkSize = header.chunk_size;
    if (!Number.isSafeInteger(chunkSize) || chunkSize <= 0) {
      throw new Error(`decryptFile: invalid header chunk_size=${chunkSize}`);
    }
    const headerHash = sha256(headerBytes);

    // -------- derive keys --------
    const ikm = await getIkm(header);
    const dek = DEK.derive(ikm, header.dek_salt).unwrap();
    const baseNonce = BaseNonce.derive(ikm, header.dek_salt).unwrap();

    // -------- body (chunked decryption) --------
    const ciphertextBuf = new ByteQueue();
    ciphertextBuf.push(remainder);

    const fullCipherChunkSize = chunkSize + AEAD_TAG_SIZE_BYTES;
    let chunkIndex = 0;
    let totalChunkCount = 0;
    let bytesProcessed = 0;

    while (true) {
      while (ciphertextBuf.length >= fullCipherChunkSize + COMMIT_CIPHERTEXT_SIZE_BYTES) {
        const ciphertextChunk = ciphertextBuf.take(fullCipherChunkSize);
        bytesProcessed = await decryptAndWriteChunk({
          writer,
          dek,
          baseNonce,
          headerHash,
          chunkIndex,
          ciphertext: ciphertextChunk,
          bytesProcessed,
          onProgress
        });
        chunkIndex++;
        totalChunkCount++;
      }

      const { done, value } = await reader.read();
      if (done) break;
      ciphertextBuf.push(value);
    }

    // Split remaining bytes into [optional final data chunk][fixed-size commit trailer].
    const tail = ciphertextBuf.takeAll();
    if (tail.length < COMMIT_CIPHERTEXT_SIZE_BYTES) {
      throw new Error("decryptFile: truncated commit trailer");
    }
    const dataTailCiphertextLength = tail.length - COMMIT_CIPHERTEXT_SIZE_BYTES;
    const dataTailCiphertext = tail.subarray(0, dataTailCiphertextLength);
    const commitCiphertext = tail.subarray(dataTailCiphertextLength);

    // Optional final data chunk (if any) must include at least an AEAD tag.
    if (dataTailCiphertext.length > 0) {
      if (dataTailCiphertext.length <= AEAD_TAG_SIZE_BYTES) {
        throw new Error("decryptFile: truncated final chunk (missing AEAD tag)");
      }
      bytesProcessed = await decryptAndWriteChunk({
        writer,
        dek,
        baseNonce,
        headerHash,
        chunkIndex,
        ciphertext: dataTailCiphertext,
        bytesProcessed,
        onProgress
      });
      chunkIndex++;
      totalChunkCount++;
    }

    // Decrypt and verify authenticated totals commit trailer.
    const commitNonce = baseNonce.getNonce(chunkIndex).unwrap();
    const commitAad = createChunkAAD(headerHash, chunkIndex, CHUNK_AAD_KIND_COMMIT);
    const commitPayload = chacha20poly1305(dek, commitNonce, commitAad).decrypt(commitCiphertext);
    const commit = parseCommitPayload(commitPayload);
    if (commit.totalChunkCount !== totalChunkCount) {
      throw new Error(
        `decryptFile: chunk count mismatch (expected ${commit.totalChunkCount}, got ${totalChunkCount})`
      );
    }
    if (commit.totalPlaintextBytes !== bytesProcessed) {
      throw new Error(
        `decryptFile: plaintext size mismatch (expected ${commit.totalPlaintextBytes}, got ${bytesProcessed})`
      );
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
