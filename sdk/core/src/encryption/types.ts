import { withUnwrap } from "../types";

type Brand<T, B extends string> = T & { readonly __brand: B };

export type DEK = Brand<Uint8Array, "DEK">;
export type AAD = Brand<Uint8Array, "AAD">;
export type Nonce = Brand<Uint8Array, "Nonce">;


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
};


export const Nonce = {
  fromBytes(bytes: Uint8Array) {
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
