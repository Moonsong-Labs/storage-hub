import { describe, it, expect } from "vitest";
import { randomBytes } from "node:crypto";
import { hexToBytes, equalBytes, bytesToHex } from "@noble/ciphers/utils.js";

import { decryptFile, encryptFile, generateEncryptionKey } from "../src/encryption.js";
import { isEncrypted, readEncryptionHeader } from "../src/encryption/cbor.js";
import { AEAD_TAG_SIZE_BYTES, COMMIT_CIPHERTEXT_SIZE_BYTES } from "../src/encryption/consts.js";
import { AAD, BaseNonce, DEK, IKM, Nonce, Salt } from "../src/encryption/types.js";
import { privateKeyToAccount } from "viem/accounts";
import { createWalletClient, defineChain, http } from "viem";
import { TEST_PRIVATE_KEY_12 } from "./consts.js";
import { ensure0xPrefix } from "../src/utils.js";

describe("encryption types sanity check", () => {
  function u64be(n: number): Uint8Array {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigUint64(0, BigInt(n));
    return b;
  }

  it("DEK/Nonce/AAD constructors: ok + error + unwrap", () => {
    // ok paths
    const dekRes = DEK.fromBytes(randomBytes(32));
    expect(dekRes.ok).toBe(true);
    const dek = dekRes.unwrap();
    expect(dek).toBeInstanceOf(Uint8Array);
    expect(dek.length).toBe(32);

    const nonceRes = Nonce.fromBytes(randomBytes(12));
    expect(nonceRes.ok).toBe(true);
    const nonce = nonceRes.unwrap();
    expect(nonce).toBeInstanceOf(Uint8Array);
    expect(nonce.length).toBe(12);

    const chunkIndex = 0;
    const aadRes = AAD.fromBytes(u64be(chunkIndex));
    expect(aadRes.ok).toBe(true);
    const aad = aadRes.unwrap();
    expect(aad).toBeInstanceOf(Uint8Array);
    expect(aad.length).toBe(8);

    // error paths
    const dekBad = DEK.fromBytes(randomBytes(31));
    expect(dekBad.ok).toBe(false);
    expect(dekBad.error).toBeInstanceOf(Error);
    expect(() => dekBad.unwrap()).toThrow();

    const nonceBad = Nonce.fromBytes(randomBytes(11));
    expect(nonceBad.ok).toBe(false);
    expect(nonceBad.error).toBeInstanceOf(Error);
    expect(() => nonceBad.unwrap()).toThrow();
  });
});

describe("DEK derivation sanity check", () => {
  it("derives a 32-byte DEK from password (IKM from password)", () => {
    const password = "correct horse battery staple";
    const ikmSalt = Salt.fromBytes(hexToBytes("aa".repeat(32))).unwrap();
    const ikmRes = IKM.fromPassword(password, ikmSalt);
    expect(ikmRes.ok).toBe(true);
    const ikm = ikmRes.unwrap();

    const salt = Salt.fromBytes(hexToBytes("11".repeat(32))).unwrap();
    const dekRes1 = DEK.derive(ikm, salt);
    expect(dekRes1.ok).toBe(true);
    const dek1 = dekRes1.unwrap();
    expect(dek1.length).toBe(32);

    const dekRes2 = DEK.derive(ikm, salt);
    expect(dekRes2.ok).toBe(true);
    const dek2 = dekRes2.unwrap();
    expect(equalBytes(dek1, dek2)).toBe(true);

    // Error path: password too short
    const short = IKM.fromPassword("short", ikmSalt);
    expect(short.ok).toBe(false);
    expect(() => short.unwrap()).toThrow();
  });

  it("derives a 32-byte DEK from signature (IKM from signature)", () => {
    const signature = `0x${"a1".repeat(65)}` as `0x${string}`; // emulate signature hex
    const ikmRes = IKM.fromSignature(signature);
    expect(ikmRes.ok).toBe(true);
    const ikm = ikmRes.unwrap();

    const salt = Salt.fromBytes(hexToBytes("22".repeat(32))).unwrap();
    const dekRes1 = DEK.derive(ikm, salt);
    expect(dekRes1.ok).toBe(true);
    const dek1 = dekRes1.unwrap();
    expect(dek1.length).toBe(32);

    // Error path: signature must be a valid hex string
    const badSig = IKM.fromSignature("not-bytes" as any);
    expect(badSig.ok).toBe(false);
    expect(() => badSig.unwrap()).toThrow();
  });
});

describe("BaseNonce + chunked file nonces", () => {
  it("derives identical per-chunk nonces for same BaseNonce", () => {
    // Deterministic base nonce bytes (12 bytes)
    const baseBytes = hexToBytes("ab".repeat(12));

    const base1 = BaseNonce.fromBytes(baseBytes).unwrap();
    const base2 = BaseNonce.fromBytes(baseBytes).unwrap();

    const derived1: Uint8Array[] = [];
    const derived2: Uint8Array[] = [];

    // Simulate chunked file: derive a nonce per chunk index
    for (let chunkIndex = 0; chunkIndex < 10; chunkIndex++) {
      const n1 = base1.getNonce(chunkIndex);
      expect(n1.ok).toBe(true);
      derived1.push(n1.unwrap());

      const n2 = base2.getNonce(chunkIndex);
      expect(n2.ok).toBe(true);
      derived2.push(n2.unwrap());
    }

    expect(derived1.length).toBe(derived2.length);
    for (let i = 0; i < derived1.length; i++) {
      const a = derived1[i];
      const b = derived2[i];
      if (!a || !b) throw new Error("missing derived nonce");
      expect(equalBytes(a, b)).toBe(true);
    }

    // sanity: different chunk indices should generally produce different nonces
    const d0 = derived1[0];
    const d1 = derived1[1];
    if (!d0 || !d1) throw new Error("missing derived nonce");
    expect(equalBytes(d0, d1)).toBe(false);
  });
});

describe("Generate DEK and base Nonce", () => {
  it("Generate DEK & Nonce from Signature", async () => {
    const rpcUrl = "http://127.0.0.1:8545" as const;
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpcUrl] } }
    });

    const account = privateKeyToAccount(TEST_PRIVATE_KEY_12);
    const walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });

    const fingerprint = "Fingerprint from unencrypted file";
    const message = `Sign this message to generate the encryption key for filekey: ${fingerprint}`;
    const signature = await walletClient.signMessage({
      account,
      message
    });

    // Make sure to use deterministic ECDSA (e.g. RFC 6979)
    const expectedSignature =
      "0xf57db1fe2cbf3fe9ba789975126bcdf679394a70363758e429271b0323fc85f265db0fb27a66b8457c5b1138747a5ebefcd5756e8bcabe110c24cffab6ff118b1b";
    expect(signature).toBe(expectedSignature);

    // Normalize input
    const ikm = IKM.fromSignature(signature).unwrap();
    const salt = Salt.fromBytes(hexToBytes("33".repeat(32))).unwrap();

    // Compute DEK and Base nonce (salted HKDF)
    const dek = DEK.derive(ikm, salt).unwrap();
    expect(dek.length).toBe(32);

    const baseNonce = BaseNonce.derive(ikm, salt).unwrap();
    expect(baseNonce.bytes.length).toBe(12);

    // Basic nonce sanity: chunk 0 nonce equals base bytes; later nonces differ
    const n0 = baseNonce.getNonce(0).unwrap();
    expect(ensure0xPrefix(bytesToHex(n0))).toBe(ensure0xPrefix(bytesToHex(baseNonce.bytes)));
    const n1 = baseNonce.getNonce(1).unwrap();
    expect(equalBytes(n0, n1)).toBe(false);
  }, 30_000);

  it("Generate DEK & Nonce from Password", async () => {
    // Normalize input
    const ikmSalt = Salt.fromBytes(hexToBytes("66".repeat(32))).unwrap();
    const ikm = IKM.fromPassword("User sets some really strong password", ikmSalt).unwrap();
    const salt = Salt.fromBytes(hexToBytes("44".repeat(32))).unwrap();

    // Compute DEK and Base nonce (salted HKDF)
    const dek = DEK.derive(ikm, salt).unwrap();
    expect(dek.length).toBe(32);

    const baseNonce = BaseNonce.derive(ikm, salt).unwrap();
    expect(baseNonce.bytes.length).toBe(12);

    const n0 = baseNonce.getNonce(0).unwrap();
    expect(ensure0xPrefix(bytesToHex(n0))).toBe(ensure0xPrefix(bytesToHex(baseNonce.bytes)));
    const n1 = baseNonce.getNonce(1).unwrap();
    expect(equalBytes(n0, n1)).toBe(false);
  }, 30_000);
});

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
  }, 20_000);

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
  }, 20_000);

  it("fails decryption when the whole final data chunk is dropped", async () => {
    const encrypted = await encryptToBytes();
    const { headerLength } = readEncryptionHeader(encrypted);

    const dropStart = headerLength + dataCipherChunkSize * 2;
    const dropEnd = dropStart + dataCipherChunkSize;
    const tampered = new Uint8Array(encrypted.length - dataCipherChunkSize);
    tampered.set(encrypted.subarray(0, dropStart), 0);
    tampered.set(encrypted.subarray(dropEnd), dropStart);

    await expect(decryptToBytes(tampered)).rejects.toThrow();
  }, 20_000);
});

describe("wrong credential handling", () => {
  const chunkSize = 64;
  const plaintext = new TextEncoder().encode("storagehub-sdk wrong-credentials test payload");

  const appName = "StorageHub";
  const domain = "https://storagehub.app/testnet";
  const version = 1;
  const purpose = "In order to generate the encryption key, we need you to sign this message";
  const chainId = 181222;

  function toReadable(bytes: Uint8Array, frameSize = 19): ReadableStream<Uint8Array> {
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

  function mutateHexSignature(signature: `0x${string}`): `0x${string}` {
    const mutatedNibble = signature[2] === "a" ? "b" : "a";
    return `0x${mutatedNibble}${signature.slice(3)}` as `0x${string}`;
  }

  it("rejects decryption with wrong password and emits no plaintext", async () => {
    const encryptedChunks: Uint8Array[] = [];
    const decryptedChunks: Uint8Array[] = [];
    const correctPassword = "correct horse battery staple";

    const { dek, baseNonce, header } = await generateEncryptionKey({
      kind: "password",
      password: correctPassword
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

    const encrypted = concatChunks(encryptedChunks);
    await expect(
      decryptFile({
        input: toReadable(encrypted),
        output: new WritableStream<Uint8Array>({
          write(chunk) {
            decryptedChunks.push(chunk);
          }
        }),
        getIkm: async (hdr) => {
          if (hdr.ikm !== "password") {
            throw new Error(`expected password header, got ${hdr.ikm}`);
          }
          return IKM.fromPassword("definitely the wrong password", hdr.ikm_salt).unwrap();
        }
      })
    ).rejects.toThrow("authentication failed");

    expect(concatChunks(decryptedChunks).length).toBe(0);
  }, 20_000);

  it("rejects decryption with wrong signature and emits no plaintext", async () => {
    const encryptedChunks: Uint8Array[] = [];
    const decryptedChunks: Uint8Array[] = [];

    const rpcUrl = "http://127.0.0.1:8545" as const;
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpcUrl] } }
    });
    const account = privateKeyToAccount(TEST_PRIVATE_KEY_12);
    const walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });

    const { dek, baseNonce, header } = await generateEncryptionKey({
      kind: "signature",
      walletClient,
      account,
      createMessage: (ikm_salt) =>
        IKM.createEncryptionKeyMessage(
          appName,
          domain,
          version,
          purpose,
          chainId,
          account.address,
          ikm_salt
        ).message
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

    const encrypted = concatChunks(encryptedChunks);
    await expect(
      decryptFile({
        input: toReadable(encrypted),
        output: new WritableStream<Uint8Array>({
          write(chunk) {
            decryptedChunks.push(chunk);
          }
        }),
        getIkm: async (hdr) => {
          if (hdr.ikm !== "signature") {
            throw new Error(`expected signature header, got ${hdr.ikm}`);
          }

          const { message } = IKM.createEncryptionKeyMessage(
            appName,
            domain,
            version,
            purpose,
            chainId,
            account.address,
            hdr.ikm_salt
          );
          const signature = await walletClient.signMessage({ account, message });
          const wrongSignature = mutateHexSignature(signature);
          return IKM.fromSignature(wrongSignature).unwrap();
        }
      })
    ).rejects.toThrow("authentication failed");

    expect(concatChunks(decryptedChunks).length).toBe(0);
  }, 30_000);
});

describe("encrypted file detection", () => {
  const password = "correct horse battery staple";
  const chunkSize = 64;
  const plaintext = new TextEncoder().encode("storagehub-sdk encrypted detection test payload");

  function toReadable(bytes: Uint8Array, frameSize = 23): ReadableStream<Uint8Array> {
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

  it("returns encrypted state with parsed info for a real encrypted file", async () => {
    const encrypted = await encryptToBytes();
    const res = isEncrypted(encrypted);
    expect(res.state).toBe("encrypted");
    if (res.state !== "encrypted") {
      throw new Error(`expected encrypted state, got ${res.state}`);
    }
    expect(res.info.version).toBe(1);
    expect(res.info.ikm).toBe("password");
    expect(res.info.chunk_size).toBe(chunkSize);
    expect(res.headerLength).toBeGreaterThan(0);
  });

  it("returns not_encrypted when magic does not match", () => {
    const res = isEncrypted(new TextEncoder().encode("not encrypted payload"));
    expect(res).toEqual({ state: "not_encrypted" });
  });

  it("returns not_encrypted when SHF magic is incomplete", async () => {
    const encrypted = await encryptToBytes();
    const truncatedMagic = encrypted.subarray(0, 2);
    const res = isEncrypted(truncatedMagic);
    expect(res).toEqual({ state: "not_encrypted" });
  });

  it("returns not_encrypted when the encrypted file is truncated before the full header", async () => {
    const encrypted = await encryptToBytes();
    const { headerLength } = readEncryptionHeader(encrypted);
    const incompleteHeader = encrypted.subarray(0, headerLength - 1);
    const res = isEncrypted(incompleteHeader);
    expect(res).toEqual({ state: "not_encrypted" });
  });

  it("returns invalid_header when the encrypted file header is corrupted", async () => {
    const encrypted = await encryptToBytes();
    const malformed = encrypted.slice();

    // Corrupt the declared CBOR header length while keeping the SHF magic intact and
    // still pointing inside the available bytes, so we exercise invalid_header instead
    // of the truncated/not_encrypted paths.
    malformed[3] = 0x00;
    malformed[4] = 0x00;
    malformed[5] = 0x00;
    malformed[6] = 0x01;

    const res = isEncrypted(malformed);
    expect(res.state).toBe("invalid_header");
    if (res.state !== "invalid_header") {
      throw new Error(`expected invalid_header state, got ${res.state}`);
    }
    expect(res.reason.length).toBeGreaterThan(0);
  });
});
