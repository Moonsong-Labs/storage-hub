import { beforeAll, describe, it, expect } from "vitest";

import { decryptFile, encryptFile, generateEncryptionKey } from "../src/encryption.js";
import { isEncrypted, readEncryptionHeader } from "../src/encryption/cbor.js";
import { IKM } from "../src/encryption/types.js";
import { privateKeyToAccount } from "viem/accounts";
import { createWalletClient, defineChain, http } from "viem";
import { TEST_PRIVATE_KEY_12 } from "./consts.js";
import { concatChunks, toReadable } from "./encryption_test_utils.js";

const SLOW_TEST_TIMEOUT = 20_000;

describe("wrong credential handling", () => {
  const chunkSize = 64;
  const plaintext = new TextEncoder().encode("storagehub-sdk wrong-credentials test payload");

  const appName = "StorageHub";
  const domain = "https://storagehub.app/testnet";
  const version = 1;
  const purpose = "In order to generate the encryption key, we need you to sign this message";
  const chainId = 181222;

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
      input: toReadable(plaintext, 19),
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
      input: toReadable(plaintext, 19),
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
        input: toReadable(encrypted, 19),
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
  let encrypted: Uint8Array;
  let encryptedHeaderLength: number;

  async function encryptToBytes(): Promise<Uint8Array> {
    const encryptedChunks: Uint8Array[] = [];
    const { dek, baseNonce, header } = await generateEncryptionKey({
      kind: "password",
      password
    });

    await encryptFile({
      input: toReadable(plaintext, 23),
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

  beforeAll(async () => {
    encrypted = await encryptToBytes();
    encryptedHeaderLength = readEncryptionHeader(encrypted).headerLength;
  }, SLOW_TEST_TIMEOUT);

  it("returns encrypted state with parsed info for a real encrypted file", () => {
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

  it("returns not_encrypted when SHF magic is incomplete", () => {
    const truncatedMagic = encrypted.subarray(0, 2);
    const res = isEncrypted(truncatedMagic);
    expect(res).toEqual({ state: "not_encrypted" });
  });

  it("returns not_encrypted when the encrypted file is truncated before the full header", () => {
    const incompleteHeader = encrypted.subarray(0, encryptedHeaderLength - 1);
    const res = isEncrypted(incompleteHeader);
    expect(res).toEqual({ state: "not_encrypted" });
  });

  it("returns invalid_header when the encrypted file header is corrupted", () => {
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
