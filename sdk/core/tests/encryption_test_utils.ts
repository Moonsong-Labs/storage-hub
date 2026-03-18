import {
  CHUNK_AAD_KIND_COMMIT,
  CHUNK_AAD_SIZE_BYTES,
  CHUNK_AAD_VERSION,
  COMMIT_MAGIC,
  COMMIT_PLAINTEXT_SIZE_BYTES,
  HEADER_HASH_SIZE_BYTES
} from "../src/encryption/consts.js";

export function toReadable(bytes: Uint8Array, frameSize = 13): ReadableStream<Uint8Array> {
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

export function concatChunks(chunks: Uint8Array[]): Uint8Array {
  const total = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.length;
  }
  return out;
}

function createChunkAad(headerHash: Uint8Array, chunkIndex: number, kind: number): Uint8Array {
  if (headerHash.length !== HEADER_HASH_SIZE_BYTES) {
    throw new Error(`createChunkAad: invalid headerHash length ${headerHash.length}`);
  }
  if (!Number.isSafeInteger(chunkIndex) || chunkIndex < 0) {
    throw new Error(`createChunkAad: invalid chunkIndex ${chunkIndex}`);
  }

  const aad = new Uint8Array(CHUNK_AAD_SIZE_BYTES);
  aad[0] = CHUNK_AAD_VERSION;
  aad[1] = kind;
  new DataView(aad.buffer, aad.byteOffset, aad.byteLength).setBigUint64(
    2,
    BigInt(chunkIndex),
    false
  );
  aad.set(headerHash, 10);
  return aad;
}

export function createCommitAad(headerHash: Uint8Array, chunkIndex: number): Uint8Array {
  return createChunkAad(headerHash, chunkIndex, CHUNK_AAD_KIND_COMMIT);
}

export function parseCommitPayload(payload: Uint8Array): {
  totalPlaintextBytes: number;
  totalChunkCount: number;
} {
  if (payload.length !== COMMIT_PLAINTEXT_SIZE_BYTES) {
    throw new Error(`parseCommitPayload: invalid payload length ${payload.length}`);
  }
  for (let i = 0; i < COMMIT_MAGIC.length; i++) {
    if (payload[i] !== COMMIT_MAGIC[i]) {
      throw new Error("parseCommitPayload: invalid commit magic");
    }
  }

  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  const totalPlaintextBytes = Number(view.getBigUint64(COMMIT_MAGIC.length, false));
  const totalChunkCount = Number(view.getBigUint64(COMMIT_MAGIC.length + 8, false));
  return { totalPlaintextBytes, totalChunkCount };
}
