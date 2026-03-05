import { SALT_SIZE } from "./consts.js";
import type { Salt } from "./types.js";

export const EncryptionHeaderVersion = {
  V1: 1
  // V2: 2,
} as const;
export type EncryptionHeaderVersion =
  (typeof EncryptionHeaderVersion)[keyof typeof EncryptionHeaderVersion];

export type IKMType = "password" | "signature";

export type EncryptionHeaderParams = {
  ikm: IKMType;
  dek_salt: Salt;
  // Input key material salt, always required and always SALT_SIZE bytes.
  ikm_salt: Salt;
  // Plaintext chunk size used for encryption framing.
  chunk_size: number;
};

export type EncryptionHeaderV1 = {
  // Versioning to keep backward compatibility
  v: typeof EncryptionHeaderVersion.V1;
  // Method to generate the IKM
  ikm: IKMType;
  dek_salt: Salt;
  ikm_salt: Salt;
  chunk_size: number;
};

function isIKMType(x: unknown): x is IKMType {
  return x === "password" || x === "signature";
}

export function isEncryptionHeaderV1(x: unknown): x is EncryptionHeaderV1 {
  // `decode()` returns `unknown`, so we must validate before using fields.
  if (typeof x !== "object" || x === null) return false;
  const obj = x as Record<string, unknown>;

  // V1 header requires v===1 (even if the value is provided externally).
  if (typeof obj.v !== "number" || obj.v !== EncryptionHeaderVersion.V1) return false;
  if (!isIKMType(obj.ikm)) return false;
  if (!(obj.dek_salt instanceof Uint8Array)) return false;
  if (obj.dek_salt.length !== SALT_SIZE) return false;
  if (!(obj.ikm_salt instanceof Uint8Array)) return false;
  if (obj.ikm_salt.length !== SALT_SIZE) return false;
  if (typeof obj.chunk_size !== "number") return false;
  if (!Number.isSafeInteger(obj.chunk_size) || obj.chunk_size <= 0) return false;

  return true;
}
