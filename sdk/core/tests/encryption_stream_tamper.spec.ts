import { describe, it, expect } from "vitest";

import { decryptFile, encryptFile, generateEncryptionKey } from "../src/encryption.js";
import { readEncryptionHeader } from "../src/encryption/cbor.js";
import { AEAD_TAG_SIZE_BYTES, COMMIT_CIPHERTEXT_SIZE_BYTES } from "../src/encryption/consts.js";
import { IKM } from "../src/encryption/types.js";

const STREAM_TEST_TIMEOUT = 20_000;

describe("stream tamper detection", () => {
  const password = "correct horse battery staple";
  const chunkSize = 32;
  const plaintext = Uint8Array.from({ length: 96 }, (_, i) => i);
  const dataCipherChunkSize = chunkSize + AEAD_TAG_SIZE_BYTES;

  function toReadable(bytes: Uint8Array, frameSize = 13): ReadableStream<Uint8Array> {
    let offset = 0;
    return new ReadableStream<Uint8Array>({
      pull(controller) {
        if (offset >= bytes.length) {
          controller.close();
          return;
        }
        const end = Math.min(offset + frameSize, bytes.length);
        controller.enqueue(bytes.subarray(offset, end));
        offset = end;
      }
    });
  }

  function concatChunks(chunks: Uint8Array[]): Uint8Array {
    const total = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  }

  async function encryptToBytes(): Promise<Uint8Array> {
    const encryptedChunks: Uint8Array[] = [];
    const { dek, baseNonce, header } = await generateEncryptionKey({
      kind: "password",
      password
    });

    await encryptFile({
      input: toReadable(plaintext),
      output: new WritableStream<Uint8Array>({
        write(chunk) {
          encryptedChunks.push(chunk);
        }
      }),
      dek,
      baseNonce,
      header: {
        ...header,
        chunk_size: chunkSize
      }
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

  it("fails decryption when a header bit is flipped", async () => {
    const encrypted = await encryptToBytes();
    const { headerLength } = readEncryptionHeader(encrypted);

    const tampered = encrypted.slice();
    const flipIndex = 7 + Math.floor((headerLength - 7) / 2);
    tampered[flipIndex] ^= 0x01;

    await expect(decryptToBytes(tampered)).rejects.toThrow();
  }, STREAM_TEST_TIMEOUT);

  it("fails decryption when ciphertext chunks are reordered", async () => {
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
  }, STREAM_TEST_TIMEOUT);

  it("fails decryption when the whole final data chunk is dropped", async () => {
    const encrypted = await encryptToBytes();
    const { headerLength } = readEncryptionHeader(encrypted);

    const dropStart = headerLength + dataCipherChunkSize * 2;
    const dropEnd = dropStart + dataCipherChunkSize;
    const tampered = new Uint8Array(encrypted.length - dataCipherChunkSize);
    tampered.set(encrypted.subarray(0, dropStart), 0);
    tampered.set(encrypted.subarray(dropEnd), dropStart);

    await expect(decryptToBytes(tampered)).rejects.toThrow();
  }, STREAM_TEST_TIMEOUT);
});
