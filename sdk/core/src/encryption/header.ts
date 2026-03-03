export const EncryptionHeaderVersion = {
  V1: 1
  // V2: 2,
} as const;
export type EncryptionHeaderVersion =
  (typeof EncryptionHeaderVersion)[keyof typeof EncryptionHeaderVersion];

export type IKMType = "password" | "signature";

export type EncryptionHeaderParams = {
  ikm: IKMType;
  dek_salt: Uint8Array;
  // Input key material salt, always required and always 32 bytes.
  ikm_salt: Uint8Array;
};

export type EncryptionHeaderV1 = {
  // Versioning to keep backward compatibility
  v: typeof EncryptionHeaderVersion.V1;
  // Method to generate the IKM
  ikm: IKMType;
  dek_salt: Uint8Array;
  ikm_salt: Uint8Array;
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
  if (!(obj.ikm_salt instanceof Uint8Array)) return false;
  if (obj.ikm_salt.length !== 32) return false;

  return true;
}
