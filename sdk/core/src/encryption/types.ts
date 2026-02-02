import { sha256 } from "@noble/hashes/sha2.js";
import { withUnwrap, type ResultWithUnwrap } from "../types";
import { hexToBytes, utf8ToBytes } from "@noble/ciphers/utils.js";
import { isHexString, removeHexPrefix } from "../utils.js";
import { blake2s_256 } from "./hash.js";

import { hkdf } from "@noble/hashes/hkdf.js";

type Brand<T, B extends string> = T & { readonly __brand: B };

export type DEK = Brand<Uint8Array, "DEK">;
export type AAD = Brand<Uint8Array, "AAD">;
export type Nonce = Brand<Uint8Array, "Nonce">;
export type IKM = Brand<Uint8Array, "IKM">;
export type Salt = Brand<Uint8Array, "Salt">;
export type Info = Brand<Uint8Array, "Info">;

const DEK_INFO = new TextEncoder().encode("storagehub-sdk:dek:v1");
const NONCE_INFO = new TextEncoder().encode("storagehub-sdk:nonce:v1");

// Data Encryption Key
export const DEK = {
  fromBytes(bytes: Uint8Array) {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("DEK must be a Uint8Array"),
      });
    }

    if (bytes.length !== 32) {
      return withUnwrap({
        ok: false,
        error: new Error(`DEK must be 32 bytes (got ${bytes.length})`),
      });
    }

    return withUnwrap({
      ok: true,
      value: bytes as DEK,
    });
  },

  /**
   * Derive a per-file (or per-session) Data Encryption Key (DEK) using HKDF-SHA256.
   *
   * - **ikm**: Input Key Material. This should come from something user-bound (e.g. password)
   *   and/or wallet-bound (e.g. signature), already normalized into an `IKM`.
   *
   * - **salt**: Non-secret salt that provides *domain separation* and prevents
   *   cross-context key reuse. A good choice is a value that changes per file
   *
   * - **info**: Non-secret context string/bytes that identifies *what this derived key is for*.
   *   This is typically a stable label such as `Info.fromBytes(utf8('storagehub-sdk:dek:v1')).unwrap()`.
   *
   * Returns a Result like type with DEK (32 bytes) if everything was ok.
   */
  derive(
    ikm: IKM,
    salt: Salt,
  ) {
    const dekBytes = hkdf(sha256, ikm, salt, DEK_INFO, 32);
    return withUnwrap({
      ok: true,
      value: dekBytes as DEK,
    });
  },
};


export const Nonce = {
  fromBytes(bytes: Uint8Array): ResultWithUnwrap<Nonce, Error> {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("Nonce must be a Uint8Array"),
      });
    }

    if (bytes.length !== 12) {
      return withUnwrap({
        ok: false,
        error: new Error(`Nonce must be 12 bytes (got ${bytes.length})`),
      });
    }

    return withUnwrap({
      ok: true,
      value: bytes as Nonce,
    });
  },
};


export type BaseNonce = {
  readonly bytes: Uint8Array;
  getNonce(chunkIndex: number): ReturnType<typeof Nonce.fromBytes>;
};

/* ---------- constructor ---------- */

export const BaseNonce = {

  derive(
    ikm: IKM,
    salt: Salt,
  ): ResultWithUnwrap<BaseNonce, Error> {
    const nonceBytes = hkdf(sha256, ikm, salt, NONCE_INFO, 12);
    return BaseNonce.fromBytes(nonceBytes);
  },

  fromBytes(bytes: Uint8Array): ResultWithUnwrap<BaseNonce, Error> {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("BaseNonce must be a Uint8Array"),
      });
    }

    if (bytes.length !== 12) {
      return withUnwrap({
        ok: false,
        error: new Error(`BaseNonce must be 12 bytes (got ${bytes.length})`),
      });
    }

    const base = new Uint8Array(bytes);
    const value: BaseNonce = Object.freeze({
      bytes: base,

      getNonce(chunkIndex: number) {
        if (!Number.isSafeInteger(chunkIndex) || chunkIndex < 0) {
          return withUnwrap({
            ok: false,
            error: new Error("chunkIndex must be a non-negative integer"),
          });
        }

        const nonce = new Uint8Array(12);
        nonce.set(base);

        // XOR last 8 bytes with chunk counter (big-endian)
        // Use DataView to make the compiler happy about out-of-bounds checks
        const view = new DataView(nonce.buffer, nonce.byteOffset, nonce.byteLength);
        let counter = BigInt(chunkIndex);
        for (let offset = 0; offset < 8; offset++) {
          const idx = 11 - offset;
          const byte = view.getUint8(idx);
          view.setUint8(idx, byte ^ Number(counter & 0xffn));
          counter >>= 8n;
        }

        return Nonce.fromBytes(nonce);
      },
    });

    return withUnwrap({
      ok: true,
      value,
    });
  },
};


// Additional Authentication Data
export const AAD = {
  fromBytes(bytes: Uint8Array) {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("AAD must be a Uint8Array"),
      });
    }

    return withUnwrap({
      ok: true,
      value: bytes as AAD,
    });
  },
};


function normalize(
  input: Uint8Array,
  context: string
): Uint8Array {
  return sha256(
    new Uint8Array([
      ...new TextEncoder().encode(context),
      ...input,
    ])
  );
}

export const IKM = {
  fromPassword(password: string): ResultWithUnwrap<IKM, Error> {
    const raw = utf8ToBytes(password);

    if (raw.length < 8) {
      return withUnwrap({
        ok: false,
        error: new Error("Password too short"),
      });
    }

    const normalized = normalize(raw, "ikm:password");
    return withUnwrap({
      ok: true,
      value: normalized as IKM,
    });
  },

  // Make sure to use deterministic ECDSA (e.g. RFC 6979)
  fromSignature(signature: `0x${string}`): ResultWithUnwrap<IKM, Error> {
    if (!isHexString(signature)) {
      return withUnwrap({
        ok: false,
        error: new Error("Signature must be a valid 0x-prefixed hex string"),
      });
    }

    const sigBytes = hexToBytes(removeHexPrefix(signature));
    const normalized = normalize(sigBytes, "ikm:signature");

    return withUnwrap({
      ok: true,
      value: normalized as IKM,
    });
  },

  fromBytes(bytes: Uint8Array) {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("IKM must be Uint8Array"),
      });
    }

    if (bytes.length < 32) {
      return withUnwrap({
        ok: false,
        error: new Error("IKM requires at least 32 bytes"),
      });
    }

    const normalized = normalize(bytes, "ikm:bytes");
    return withUnwrap({
      ok: true,
      value: normalized as IKM,
    });
  },

  /**
   * Create the deterministic message that must be signed to derive the IKM.
   *
   * Keep this stable across SDK versions for recoverability.
   */
  createMessage(
    appName: string,
    domain: string,
    version: number,
    purpose: string,
    chainId: number,
    address: `0x${string}`,
    fileHash: `0x${string}`,
  ): string {
    return [
      `${appName} – Encryption Key Derivation`,
      "",
      `Purpose: ${purpose}`,
      `Version: ${version}`,
      `Domain: ${domain}`,
      `Chain ID: ${chainId}`,
      `Address: ${address}`,
      `File Hash (blake2s-256): ${fileHash}`,
      "",
      "⚠️ SECURITY NOTICE",
      "This signature does NOT authorize any blockchain transaction.",
      "This signature WILL be used to derive encryption keys.",
      "Anyone with access to this signature can decrypt the associated data.",
      "Never share this signature with anyone.",
    ].join("\n");
  },

  /**
   * SDK-owned helper: compute the file hash (BLAKE2s-256) and create the signature message.
   *
   * IMPORTANT: the provided `stream` is consumed (one-shot). If you need to encrypt afterwards,
   * obtain a fresh stream from the same file/source.
   */
  async createEncryptionKeyMessage(
    appName: string,
    domain: string,
    version: number,
    purpose: string,
    chainId: number,
    address: `0x${string}`,
    stream: ReadableStream<Uint8Array>,
  ): Promise<{ message: string; fileHash: `0x${string}` }> {
    const fileHash = await blake2s_256(stream);
    return {
      fileHash,
      message: IKM.createMessage(
        appName,
        domain,
        version,
        purpose,
        chainId,
        address,
        fileHash,
      ),
    };
  },
};

export const Salt = {
  fromBytes(bytes: Uint8Array): ResultWithUnwrap<Salt, Error> {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("Salt must be a Uint8Array"),
      });
    }

    // No fixed length requirement: HKDF salt can be any length (including empty).
    return withUnwrap({
      ok: true,
      value: bytes as Salt,
    });
  },
};

export const Info = {
  fromBytes(bytes: Uint8Array): ResultWithUnwrap<Info, Error> {
    if (!(bytes instanceof Uint8Array)) {
      return withUnwrap({
        ok: false,
        error: new TypeError("Info must be a Uint8Array"),
      });
    }

    // No fixed length requirement: HKDF info can be any length (including empty).
    return withUnwrap({
      ok: true,
      value: bytes as Info,
    });
  },
};