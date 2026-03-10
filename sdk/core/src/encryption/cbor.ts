import { decode, encode, rfc8949EncodeOptions } from "cborg";
import {
  EncryptionHeaderVersion as HeaderVersion,
  type IKMType,
  isEncryptionHeaderV1,
  type EncryptionHeaderParams,
  type EncryptionHeaderV1
} from "./header.js";
export { EncryptionHeaderVersion } from "./header.js";
export type {
  IKMType,
  EncryptionHeaderParams,
  EncryptionHeaderV1
} from "./header.js";

export const HEADER_MAGIC: Readonly<Uint8Array> = new TextEncoder().encode("SHF"); // StorageHub File
const HEADER_PREFIX_LENGTH = HEADER_MAGIC.length + 4;

export type EncryptedFileInfo = {
  version: number;
  ikm: IKMType;
  chunk_size: number;
};

export type EncryptedFileState =
  | {
      state: "encrypted";
      header: EncryptionHeaderV1;
      headerLength: number;
      info: EncryptedFileInfo;
    }
  | { state: "not_encrypted" }
  | { state: "invalid_header"; reason: string };

function parseEncryptionHeaderOrThrow(input: Uint8Array): {
  header: EncryptionHeaderV1;
  headerLength: number;
} {
  if (!(input instanceof Uint8Array)) {
    throw new TypeError("input must be Uint8Array");
  }
  if (input.length < HEADER_PREFIX_LENGTH) {
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

function hasEncryptionMagic(input: Uint8Array): boolean {
  if (input.length < HEADER_MAGIC.length) {
    return false;
  }
  for (let i = 0; i < HEADER_MAGIC.length; i++) {
    if (input[i] !== HEADER_MAGIC[i]) {
      return false;
    }
  }
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
    v: HeaderVersion.V1,
    ikm: params.ikm,
    dek_salt: params.dek_salt,
    ikm_salt: params.ikm_salt,
    chunk_size: params.chunk_size
  };

  // Deterministic encoding: RFC 8949 "deterministic mode" map sorting.
  const cborBytes = encode(header, rfc8949EncodeOptions);

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

export function isEncrypted(input: Uint8Array): EncryptedFileState {
  if (!(input instanceof Uint8Array)) {
    throw new TypeError("input must be Uint8Array");
  }

  if (!hasEncryptionMagic(input)) {
    return { state: "not_encrypted" };
  }

  // The caller wants a simple encrypted/not-encrypted flag. If we cannot read a complete
  // header yet, do not classify it as invalid_header.
  if (input.length < HEADER_PREFIX_LENGTH) {
    return { state: "not_encrypted" };
  }

  const declaredHeaderLength = new DataView(
    input.buffer,
    input.byteOffset,
    input.byteLength
  ).getUint32(HEADER_MAGIC.length, false);
  if (declaredHeaderLength > input.length - HEADER_PREFIX_LENGTH) {
    return { state: "not_encrypted" };
  }

  try {
    const { header, headerLength } = parseEncryptionHeaderOrThrow(input);
    return {
      state: "encrypted",
      header,
      headerLength,
      info: {
        version: header.v,
        ikm: header.ikm,
        chunk_size: header.chunk_size
      }
    };
  } catch (error) {
    return {
      state: "invalid_header",
      reason: error instanceof Error ? error.message : String(error)
    };
  }
}

export function readEncryptionHeader(input: Uint8Array): {
  header: EncryptionHeaderV1;
  headerLength: number;
} {
  return parseEncryptionHeaderOrThrow(input);
}
