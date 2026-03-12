import { beforeAll, describe, it, expect } from "vitest";
import { chacha20poly1305 } from "@noble/ciphers/chacha.js";
import { sha256 } from "@noble/hashes/sha2.js";

import { decryptFile, encryptFile, generateEncryptionKey } from "../src/encryption.js";
import { readEncryptionHeader } from "../src/encryption/cbor.js";
import { AEAD_TAG_SIZE_BYTES, COMMIT_CIPHERTEXT_SIZE_BYTES } from "../src/encryption/consts.js";
import { IKM, type DEK, type BaseNonce } from "../src/encryption/types.js";
import type { EncryptionHeaderParams } from "../src/encryption/header.js";
import {
  concatChunks,
  createCommitAad,
  parseCommitPayload,
  toReadable
} from "./encryption_test_utils.js";

const STREAM_TEST_TIMEOUT = 60_000;

describe("stream tamper detection", () => {
  const password = "correct horse battery staple";
  const chunkSize = 32;
  const plaintext = Uint8Array.from({ length: 96 }, (_, i) => i);
  const dataCipherChunkSize = chunkSize + AEAD_TAG_SIZE_BYTES;

  let dek: DEK;
  let baseNonce: BaseNonce;
  let header: EncryptionHeaderParams;

  beforeAll(async () => {
    const keys = await generateEncryptionKey({
      kind: "password",
      password
    });
    dek = keys.dek;
    baseNonce = keys.baseNonce;
    header = { ...keys.header, chunk_size: chunkSize };
  }, STREAM_TEST_TIMEOUT);

  async function encryptToBytes(): Promise<Uint8Array> {
    const encryptedChunks: Uint8Array[] = [];

    await encryptFile({
      input: toReadable(plaintext),
      output: new WritableStream<Uint8Array>({
        write(chunk) {
          encryptedChunks.push(chunk);
        }
      }),
      dek,
      baseNonce,
      header
    });

    return concatChunks(encryptedChunks);
  }

  async function decryptToBytes(ciphertext: Uint8Array): Promise<Uint8Array> {
    const plaintextChunks: Uint8Array[] = [];
    await decryptFile({
      input: toReadable(ciphertext, 17),
      output: new WritableStream<Uint8Array>({
        write(chunk) {
          plaintextChunks.push(chunk);
        }
      }),
      getIkm: async (header) => {
        if (header.ikm !== "password") {
          throw new Error(`expected password header, got ${header.ikm}`);
        }
        return IKM.fromPassword(password, header.ikm_salt).unwrap();
      }
    });
    return concatChunks(plaintextChunks);
  }

  it(
    "encrypts/decrypts empty input and commits with nonce index 0",
    async () => {
      const encryptedChunks: Uint8Array[] = [];
      const decryptedChunks: Uint8Array[] = [];

      // Encrypt empty stream: output should be header + commit trailer only.
      await encryptFile({
        input: toReadable(new Uint8Array(), 1),
        output: new WritableStream<Uint8Array>({
          write(chunk) {
            encryptedChunks.push(chunk);
          }
        }),
        dek,
        baseNonce,
        header
      });

      const encrypted = concatChunks(encryptedChunks);
      const { headerLength } = readEncryptionHeader(encrypted);
      expect(encrypted.length).toBe(headerLength + COMMIT_CIPHERTEXT_SIZE_BYTES);

      const headerBytes = encrypted.subarray(0, headerLength);
      const commitCiphertext = encrypted.subarray(headerLength);
      // Empty input means chunkIndex never increments, so commit nonce index is 0.
      const commitAad = createCommitAad(sha256(headerBytes), 0);
      const commitNonce = baseNonce.getNonce(0).unwrap();
      const commitPayload = chacha20poly1305(dek, commitNonce, commitAad).decrypt(commitCiphertext);
      const commit = parseCommitPayload(commitPayload);
      expect(commit.totalPlaintextBytes).toBe(0);
      expect(commit.totalChunkCount).toBe(0);

      // Full decrypt should emit no plaintext bytes.
      await decryptFile({
        input: toReadable(encrypted, 19),
        output: new WritableStream<Uint8Array>({
          write(chunk) {
            decryptedChunks.push(chunk);
          }
        }),
        getIkm: async (hdr) => {
          if (hdr.ikm !== "password") {
            throw new Error(`expected password header, got ${hdr.ikm}`);
          }
          return IKM.fromPassword(password, hdr.ikm_salt).unwrap();
        }
      });

      expect(concatChunks(decryptedChunks).length).toBe(0);
    },
    STREAM_TEST_TIMEOUT
  );

  it(
    "fails decryption when a header bit is flipped",
    async () => {
      const encrypted = await encryptToBytes();
      const { headerLength } = readEncryptionHeader(encrypted);

      const tampered = encrypted.slice();
      const flipIndex = 7 + Math.floor((headerLength - 7) / 2);
      tampered[flipIndex] ^= 0x01;

      await expect(decryptToBytes(tampered)).rejects.toThrow();
    },
    STREAM_TEST_TIMEOUT
  );

  it(
    "fails decryption when ciphertext chunks are reordered",
    async () => {
      const encrypted = await encryptToBytes();
      const { headerLength } = readEncryptionHeader(encrypted);
      const bodyLength = encrypted.length - headerLength;
      const expectedBodyLength = dataCipherChunkSize * 3 + COMMIT_CIPHERTEXT_SIZE_BYTES;
      expect(bodyLength).toBe(expectedBodyLength);

      const tampered = encrypted.slice();
      const c0Start = headerLength;
      const c1Start = headerLength + dataCipherChunkSize;

      const chunk0 = tampered.slice(c0Start, c0Start + dataCipherChunkSize);
      const chunk1 = tampered.slice(c1Start, c1Start + dataCipherChunkSize);
      tampered.set(chunk1, c0Start);
      tampered.set(chunk0, c1Start);

      await expect(decryptToBytes(tampered)).rejects.toThrow();
    },
    STREAM_TEST_TIMEOUT
  );

  it(
    "fails decryption when the whole final data chunk is dropped",
    async () => {
      const encrypted = await encryptToBytes();
      const { headerLength } = readEncryptionHeader(encrypted);

      const dropStart = headerLength + dataCipherChunkSize * 2;
      const dropEnd = dropStart + dataCipherChunkSize;
      const tampered = new Uint8Array(encrypted.length - dataCipherChunkSize);
      tampered.set(encrypted.subarray(0, dropStart), 0);
      tampered.set(encrypted.subarray(dropEnd), dropStart);

      await expect(decryptToBytes(tampered)).rejects.toThrow();
    },
    STREAM_TEST_TIMEOUT
  );
});
