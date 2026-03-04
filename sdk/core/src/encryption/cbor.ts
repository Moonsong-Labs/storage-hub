import { decode, encode, rfc8949EncodeOptions } from "cborg";
import {
  EncryptionHeaderVersion as HeaderVersion,
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

const HEADER_MAGIC = new TextEncoder().encode("SHF"); // StorageHub File

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
