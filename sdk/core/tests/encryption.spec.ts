import { beforeAll, describe, it, expect } from 'vitest';
import { chacha20poly1305 } from '@noble/ciphers/chacha.js';
import { blake2s } from '@noble/hashes/blake2.js';
import { bytesToHex as bytesToHexHash } from '@noble/hashes/utils.js';

import { decryptFile, encryptFile, generateEncryptionKey } from '../src/encryption.js';
import { ENCRYPTION_CHUNK_SIZE } from '../src/constants.js';
import { IKM } from '../src/encryption/types.js';

import { hexToBytes, bytesToHex, equalBytes } from '@noble/ciphers/utils.js';
import { createReadStream, createWriteStream, statSync, mkdirSync } from 'fs';
import { Readable, Writable } from 'stream';

import { generateRandomFile } from './utils.js';
import { join } from 'path';
import { readFileSync } from 'node:fs';
import { createWalletClient, defineChain, http } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { TEST_PRIVATE_KEY_12 } from './consts.js';
import { readEncryptionHeader } from '../src/encryption/cbor.js';
import { blake2s_256 } from '../src/encryption/hash.js';

const RESOURCE_DIR = join(__dirname, 'resources');

const FILE_SIZES_MB = [50, 100, 500, 1000, 2000];

beforeAll(async () => {
  for (const size of FILE_SIZES_MB) {
    const path = join(RESOURCE_DIR, `random-${size}mb.bin`);
    console.log(`[setup] ensuring ${size}MB file`);
    await generateRandomFile(path, size);
  }
});

// ---- helpers -----------------------------------------------------

const LOREN_IPSUM =
  'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.';

const LOREN_BYTES = new TextEncoder().encode(LOREN_IPSUM);
// Repeat so we exercise multi-chunk behavior deterministically
const ORIGINAL = new Uint8Array(LOREN_BYTES.length * 600);
for (let i = 0; i < 600; i++) {
  ORIGINAL.set(LOREN_BYTES, i * LOREN_BYTES.length);
}

function bufferToWebReadable(buf: Uint8Array, chunkSize = ENCRYPTION_CHUNK_SIZE) {
  let offset = 0;

  return new ReadableStream<Uint8Array>({
    pull(controller) {
      if (offset >= buf.length) {
        controller.close();
        return;
      }

      const end = Math.min(offset + chunkSize, buf.length);
      controller.enqueue(buf.slice(offset, end));
      offset = end;
    },
  });
}

async function webReadableToBuffer(
  stream: ReadableStream<Uint8Array>
): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    chunks.push(value);
    total += value.length;
  }

  const result = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    result.set(c, offset);
    offset += c.length;
  }

  return result;
}

function createBlake2s256Sink(): { writable: WritableStream<Uint8Array>; digest: Promise<`0x${string}`> } {
  const hasher = blake2s.create({ dkLen: 32 });
  let resolveDigest: ((d: `0x${string}`) => void) | undefined;
  const digest = new Promise<`0x${string}`>((resolve) => {
    resolveDigest = resolve;
  });

  return {
    digest,
    writable: new WritableStream<Uint8Array>({
      write(chunk) {
        hasher.update(chunk);
      },
      close() {
        resolveDigest?.(`0x${bytesToHexHash(hasher.digest())}` as `0x${string}`);
      },
    }),
  };
}

describe('stream encryption / decryption benchmarks', () => {
  for (const size of FILE_SIZES_MB) {
    it(`benchmarks ${size}MB file`, async () => {
      const path = join(RESOURCE_DIR, `random-${size}mb.bin`);
      const fileSizeBytes = statSync(path).size;

      // Use password flow for benchmarks to avoid requiring a wallet.
      const password = 'benchmark password (do not use)';
      const { dek, baseNonce, header } = await generateEncryptionKey({
        kind: 'password',
        password,
      });

      // Output folder for benchmark artifacts
      const outDir = join(RESOURCE_DIR, 'benchmarks');
      mkdirSync(outDir, { recursive: true });
      const encryptedPath = join(outDir, `random-${size}mb.bin.enc`);

      // ── Original hash (BLAKE2s-256) ───────────────
      const originalHash = await blake2s_256(
        Readable.toWeb(createReadStream(path)) as unknown as ReadableStream<Uint8Array>,
      );

      // ── Encrypt (stream -> file) ──────────────────
      const encStart = performance.now();
      await encryptFile({
        input: Readable.toWeb(createReadStream(path)) as unknown as ReadableStream<Uint8Array>,
        output: Writable.toWeb(createWriteStream(encryptedPath)) as unknown as WritableStream<Uint8Array>,
        dek,
        baseNonce,
        header,
      });
      const encTime = performance.now() - encStart;

      // ── Decrypt (file -> BLAKE2s sink) ────────────
      const decStart = performance.now();
      const { writable: hashSink, digest: decryptedHashP } = createBlake2s256Sink();
      await decryptFile({
        input: Readable.toWeb(createReadStream(encryptedPath)) as unknown as ReadableStream<Uint8Array>,
        output: hashSink,
        getIkm: async (hdr) => {
          if (hdr.ikm !== 'password') {
            throw new Error(`benchmark expected password header, got ${hdr.ikm}`);
          }
          return IKM.fromPassword(password).unwrap();
        },
      });
      const decTime = performance.now() - decStart;
      const decryptedHash = await decryptedHashP;

      // ── Validate ──────────────────────────────────
      expect(decryptedHash).toBe(originalHash);

      // ── Report ────────────────────────────────────
      const sizeMB = fileSizeBytes / (1024 * 1024);
      console.log(`
📦 File: ${sizeMB.toFixed(0)} MB
🔐 Encrypt: ${(encTime / 1000).toFixed(2)} s  (${(sizeMB / (encTime / 1000)).toFixed(1)} MB/s)
🔓 Decrypt + Hashing: ${(decTime / 1000).toFixed(2)} s  (${(sizeMB / (decTime / 1000)).toFixed(1)} MB/s)
`);
    }, 60_000);
  }
}, 60_000);


describe('E2E encryption / decryption', () => {
  it('encrypts/decrypts adolphus.jpg from signature', async () => {
    const path = join(__dirname, "../../../docker/resource", "adolphus.jpg");
    const original = new Uint8Array(readFileSync(path));

    console.log(`Adolphus path: ${path}`)

    // Create owner's account
    const rpcUrl = "http://127.0.0.1:8545" as const;
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpcUrl] } }
    });

    const account = privateKeyToAccount(TEST_PRIVATE_KEY_12);
    const walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });

    //dApp params:
    const appName = "StorageHub";
    const domain = "https://storagehub.app/testnet";
    const version = 1;
    const purpose = "In order to generate the encryption key, we need you to sign this message";
    const chainId = 181222;
    const { message, fileHash } = await IKM.createEncryptionKeyMessage(
      appName,
      domain,
      version,
      purpose,
      chainId,
      account.address,
      bufferToWebReadable(original),
    );

    console.log(`Message to sign: ${message}`);
    const { dek, baseNonce, header } = await generateEncryptionKey({
      kind: "signature",
      walletClient,
      account,
      message,
    });

    // Quick sanity checks before proceeding to encryption/decryption.
    expect(dek.length).toBe(32);
    expect(baseNonce.bytes.length).toBe(12);
    expect(header.ikm).toBe("signature");
    expect(header.salt.length).toBe(32);

    // ── Encrypt to a repo-local folder (easy to find) ────────────────
    const outDir = join(RESOURCE_DIR, "encrypted");
    mkdirSync(outDir, { recursive: true });
    const encryptedPath = join(outDir, "adolphus.jpg.enc");
    console.log(`Encrypted output: ${encryptedPath}`);

    await encryptFile({
      input: bufferToWebReadable(original, ENCRYPTION_CHUNK_SIZE),
      output: Writable.toWeb(createWriteStream(encryptedPath)) as unknown as WritableStream<Uint8Array>,
      dek,
      baseNonce,
      header,
    });

    // ── Validate header was written correctly ─────────
    const encryptedBytes = new Uint8Array(readFileSync(encryptedPath));
    const { header: parsedHeader, headerLength } = readEncryptionHeader(encryptedBytes);
    expect(parsedHeader.ikm).toBe("signature");
    expect(Buffer.compare(Buffer.from(parsedHeader.salt), Buffer.from(header.salt))).toBe(0);
    expect(headerLength).toBeGreaterThan(0);
    expect(encryptedBytes.length).toBeGreaterThan(headerLength);

    // ── Decrypt + hash + compare ─────────────────────
    const decryptedPath = join(outDir, "adolphus.jpg.dec");
    await decryptFile({
      input: Readable.toWeb(createReadStream(encryptedPath)) as unknown as ReadableStream<Uint8Array>,
      output: Writable.toWeb(createWriteStream(decryptedPath)) as unknown as WritableStream<Uint8Array>,
      getIkm: async () => {
        // In tests we can re-sign the same message. In real usage, the SDK will need
        // enough info persisted to reproduce the message to sign.
        const signature = await walletClient.signMessage({ account, message });
        return IKM.fromSignature(signature).unwrap();
      },
    });

    const decryptedHash = await blake2s_256(
      Readable.toWeb(createReadStream(decryptedPath)) as unknown as ReadableStream<Uint8Array>,
    );
    console.log(`Decrypted file hash: ${decryptedHash}`);
    expect(decryptedHash).toBe(fileHash);
  });
})

// ---- tests -------------------------------------------------------
describe('ChaCha20-Poly1305 RFC 8439 test vector (noble)', () => {
  // Test extracted from ChaCha20-Poly1305 test vectors from RFC 8439
  // https://datatracker.ietf.org/doc/html/rfc8439#section-2.8.2
  it('encrypts and decrypts exactly as RFC 8439 specifies', () => {
    const key = hexToBytes(
      '808182838485868788898a8b8c8d8e8f' +
      '909192939495969798999a9b9c9d9e9f'
    );

    const nonce = hexToBytes('070000004041424344454647');

    const aad = hexToBytes('50515253c0c1c2c3c4c5c6c7');

    const plaintext = hexToBytes(
      '4c616469657320616e642047656e746c656d656e206f662074686520636c61737320' +
      '6f66202739393a204966204920636f756c64206f6666657220796f75206f6e6c7920' +
      '6f6e652074697020666f7220746865206675747572652c2073756e73637265656e20' +
      '776f756c642062652069742e'
    );

    const expectedCiphertextAndTag = hexToBytes(
      'd31a8d34648e60db7b86afbc53ef7ec2' +
      'a4aded51296e08fea9e2b5a736ee62d6' +
      '3dbea45e8ca9671282fafb69da92728b' +
      '1a71de0a9e060b2905d6a5b67ecd3b36' +
      '92ddbd7f2d778b8c9803aee328091b58' +
      'fab324e4fad675945585808b4831d7bc' +
      '3ff4def08e4b7a9de576d26586cec64b' +
      '6116' +
      // auth tag
      '1ae10b594f09e26a7e902ecbd0600691'
    );

    const aead = chacha20poly1305(key, nonce, aad);

    const encrypted = aead.encrypt(plaintext);
    expect(bytesToHex(encrypted)).toBe(bytesToHex(expectedCiphertextAndTag));

    const decrypted = aead.decrypt(encrypted);
    expect(bytesToHex(decrypted)).toBe(bytesToHex(plaintext));
  });

});

