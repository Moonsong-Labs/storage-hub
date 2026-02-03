import { createWriteStream, existsSync, mkdirSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { dirname } from "node:path";
import { sha256 } from "@noble/hashes/sha2.js";

const WRITE_CHUNK_SIZE = 8 * 1024 * 1024; // 8 MB

export async function generateRandomFile(path: string, sizeMB: number): Promise<void> {
  if (existsSync(path)) return;

  if (existsSync(path)) {
    // Optional but very useful during benchmarks
    console.log(`[test] Skipping existing file: ${path}`);
    return;
  }

  mkdirSync(dirname(path), { recursive: true });

  const totalBytes = sizeMB * 1024 * 1024;
  let written = 0;

  const stream = createWriteStream(path);

  while (written < totalBytes) {
    const remaining = totalBytes - written;
    const chunkSize = Math.min(WRITE_CHUNK_SIZE, remaining);
    const chunk = randomBytes(chunkSize);

    if (!stream.write(chunk)) {
      await new Promise<void>((resolve) => stream.once("drain", () => resolve()));
    }

    written += chunkSize;
  }

  await new Promise<void>((resolve, reject) => {
    stream.end(() => resolve());
    stream.on("error", reject);
  });
}

export async function hashWebStream(
  // Vitest runs in Node where `Readable.toWeb()` returns Node's `stream/web` types,
  // which differ slightly from DOM lib types (esp. with exactOptionalPropertyTypes).
  // Accept both and use runtime-compatible reader access.
  stream: ReadableStream<Uint8Array> | import("node:stream/web").ReadableStream<Uint8Array>
): Promise<Uint8Array> {
  const hasher = sha256.create();
  const reader = (stream as any).getReader() as ReadableStreamDefaultReader<Uint8Array>;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    hasher.update(value);
  }

  return hasher.digest();
}
