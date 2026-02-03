import { decode, encodeCanonical } from "cbor";

export const EncryptionHeaderVersion = {
  V1: 1
  // V2: 2,
} as const;
export type EncryptionHeaderVersion =
  (typeof EncryptionHeaderVersion)[keyof typeof EncryptionHeaderVersion];

export type IKMType = "password" | "signature";

export type EncryptionHeaderParams = {
  ikm: IKMType;
  salt: Uint8Array;
};

export type EncryptionHeaderV1 = {
  // Versioning to keep backward compatibility
  v: typeof EncryptionHeaderVersion.V1;
  // Method to generate the IKM
  ikm: IKMType;
  // hash(file | random number)
  salt: Uint8Array;
};

const HEADER_MAGIC = new TextEncoder().encode("SHF"); // StorageHub File

function isIKMType(x: unknown): x is IKMType {
  return x === "password" || x === "signature";
}

function isEncryptionHeaderV1(x: unknown): x is EncryptionHeaderV1 {
  // `decode()` returns `unknown`, so we must validate before using fields.
  if (typeof x !== "object" || x === null) return false;
  const obj = x as Record<string, unknown>;

  // V1 header requires v===1 (even if the value is provided externally).
  if (typeof obj.v !== "number" || obj.v !== EncryptionHeaderVersion.V1) return false;
  if (!isIKMType(obj.ikm)) return false;
  if (!(obj.salt instanceof Uint8Array)) return false;

  return true;
}

/**
 * Create a CBOR-encoded encryption header.
 *
 * Layout:
 *   [ magic (3) ][ u32be header_len ][ cbor_header ]
 */
export function createEncryptionHeader(params: EncryptionHeaderParams): Uint8Array {
  const header: EncryptionHeaderV1 = {
    v: EncryptionHeaderVersion.V1,
    ikm: params.ikm,
    salt: params.salt
  };

  // NOTE: `cbor.encode()` is variadic; passing `{ canonical: true }` would encode
  // it as a *second* CBOR item. Use `encodeCanonical()` instead.
  const cborHeader = encodeCanonical(header);
  const cborBytes = cborHeader instanceof Uint8Array ? cborHeader : new Uint8Array(cborHeader);

  const out = new Uint8Array(HEADER_MAGIC.length + 4 + cborBytes.length);
  let offset = 0;

  out.set(HEADER_MAGIC, offset);
  offset += HEADER_MAGIC.length;

  new DataView(out.buffer, out.byteOffset, out.byteLength).setUint32(
    offset,
    cborBytes.length,
    false
  );
  offset += 4;

  out.set(cborBytes, offset);
  return out;
}

export function readEncryptionHeader(input: Uint8Array): {
  header: EncryptionHeaderV1;
  headerLength: number;
} {
  if (!(input instanceof Uint8Array)) {
    throw new TypeError("input must be Uint8Array");
  }
  if (input.length < HEADER_MAGIC.length + 4) {
    throw new Error("Invalid file format (truncated header)");
  }

  let offset = 0;

  // Magic check
  for (let i = 0; i < HEADER_MAGIC.length; i++) {
    if (input[i] !== HEADER_MAGIC[i]) {
      throw new Error("Invalid file format (magic mismatch)");
    }
  }
  offset += HEADER_MAGIC.length;

  const view = new DataView(input.buffer, input.byteOffset, input.byteLength);
  const headerLen = view.getUint32(offset, false);
  offset += 4;

  if (headerLen > input.length - offset) {
    throw new Error("Invalid file format (header length out of bounds)");
  }

  const headerBytes = input.subarray(offset, offset + headerLen);
  offset += headerLen;

  const decoded = decode(headerBytes);
  if (!isEncryptionHeaderV1(decoded)) {
    throw new Error("Invalid encryption header");
  }

  return {
    header: decoded,
    headerLength: offset
  };
}
