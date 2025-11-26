/**
 * Utilities for generating large test files.
 */

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { createWriteStream } from "node:fs";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

/**
 * Generate a large file filled with a repeating byte pattern.
 */
export async function generateLargeFile(
  sizeInGB: number,
  outputPath: string,
  fillByte: number = 0x01
): Promise<void> {
  const sizeInBytes = Math.floor(sizeInGB * 1024 * 1024 * 1024);
  // Use 1MB chunks for small files, 10MB for larger
  const chunkSize = sizeInBytes < 100 * 1024 * 1024 ? 1024 * 1024 : 10 * 1024 * 1024;

  console.log(`📦 Generating ${sizeInGB}GB test file at ${outputPath}...`);

  await fs.mkdir(path.dirname(outputPath), { recursive: true });

  const reusableChunk = Buffer.alloc(chunkSize, fillByte);
  let bytesWritten = 0;

  const dataGenerator = new Readable({
    read() {
      if (bytesWritten >= sizeInBytes) {
        this.push(null);
        return;
      }

      const remaining = sizeInBytes - bytesWritten;
      const chunk = remaining >= chunkSize ? reusableChunk : reusableChunk.subarray(0, remaining);

      bytesWritten += chunk.length;
      this.push(chunk);
    }
  });

  const startTime = Date.now();
  await pipeline(dataGenerator, createWriteStream(outputPath));
  const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);

  console.log(`✅ File generated in ${elapsed}s`);
}

/**
 * Get file size in bytes.
 */
export async function getFileSize(filePath: string): Promise<number> {
  return (await fs.stat(filePath)).size;
}

/**
 * Delete a file if it exists.
 */
export async function deleteFileIfExists(filePath: string): Promise<void> {
  try {
    await fs.unlink(filePath);
  } catch (error: any) {
    if (error.code !== "ENOENT") throw error;
  }
}
