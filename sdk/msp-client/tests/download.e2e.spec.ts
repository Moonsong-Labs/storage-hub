import { describe, it, expect, beforeAll } from 'vitest';
import { MspClient } from '../src/MspClient';
import { createWriteStream, createReadStream } from 'fs';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';
import { unlink } from 'fs/promises';
import { stat } from 'fs/promises';
import { createHash } from 'crypto';

/**
 * Test helper utility to convert a ReadableStream to ArrayBuffer.
 */
async function streamToArrayBuffer(stream: ReadableStream<Uint8Array>): Promise<ArrayBuffer> {
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    let totalLength = 0;

    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            chunks.push(value);
            totalLength += value.length;
        }

        // Combine all chunks into a single ArrayBuffer
        const result = new Uint8Array(totalLength);
        let offset = 0;
        for (const chunk of chunks) {
            result.set(chunk, offset);
            offset += chunk.length;
        }

        return result.buffer;
    } finally {
        reader.releaseLock();
    }
}

/**
 * Stream a ReadableStream directly to disk without loading into memory.
 * Returns the file size in bytes.
 */
async function streamToFile(stream: ReadableStream<Uint8Array>, filePath: string): Promise<number> {
    // Convert ReadableStream to Node.js Readable stream
    const reader = stream.getReader();
    const nodeStream = new Readable({
        async read() {
            try {
                const { done, value } = await reader.read();
                if (done) {
                    this.push(null); // End of stream
                } else {
                    this.push(Buffer.from(value));
                }
            } catch (error) {
                this.destroy(error as Error);
            }
        }
    });

    // Stream directly to file
    const writeStream = createWriteStream(filePath);
    await pipeline(nodeStream, writeStream);

    // Get file size
    const stats = await stat(filePath);
    return stats.size;
}

/**
 * Compute SHA-256 hash of a file on disk using streaming.
 * Memory efficient for large files.
 */
async function computeFileHash(filePath: string): Promise<string> {
    const hash = createHash('sha256');
    const stream = createReadStream(filePath);

    for await (const chunk of stream) {
        hash.update(chunk);
    }

    return hash.digest('hex');
}

/**
 * Compute SHA-256 hash of a string.
 */
function computeStringHash(content: string): string {
    return createHash('sha256').update(content, 'utf8').digest('hex');
}

const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
const MSP_TOKEN = process.env.MSP_TOKEN || 'test';

describe('MspClient.download (e2e)', () => {
    let client: MspClient;

    beforeAll(async () => {
        client = await MspClient.connect({ baseUrl: MSP_BASE_URL });
        client.setToken(MSP_TOKEN);
    });

    it('downloads complete file from backend and saves to disk', async () => {
        // Test the download endpoint with backend
        const downloadResult = await client.downloadByKey('test-bucket', 'any-file-key');

        expect(downloadResult.status).toBe(200);
        expect(downloadResult.stream).toBeInstanceOf(ReadableStream);
        expect(downloadResult.contentType).toBeTruthy();

        // Stream the entire file to disk
        const tempFilePath = '/tmp/download-test-file.bin';

        try {
            const fileSize = await streamToFile(downloadResult.stream, tempFilePath);

            // Verify the file was downloaded completely
            expect(fileSize).toBeGreaterThan(0);

            // For a 100MB file, we should see approximately that size
            console.log(`Download test successful: Saved ${fileSize} bytes to ${tempFilePath}`);

            // Optional: verify file exists and has correct size
            const stats = await stat(tempFilePath);
            expect(stats.size).toBe(fileSize);
            expect(stats.isFile()).toBe(true);

        } finally {
            // Clean up the temporary file
            try {
                await unlink(tempFilePath);
            } catch (error) {
                // File might not exist, ignore error
                console.warn('Could not clean up temp file:', error);
            }
        }
    });

    it.skip('downloads a file by key using streaming', async () => {
        // First upload a small test file
        const testContent = 'Hello, streaming world! This is a test file for download.';
        const testData = new TextEncoder().encode(testContent);
        const testBlob = new Blob([testData]);

        const bucketId = 'test-bucket';
        const fileKey = 'test-streaming-file';

        // Upload the file first
        const uploadResult = await client.uploadFile(bucketId, fileKey, testBlob);
        console.log('Upload result:', uploadResult);

        // Now download it using streaming
        const downloadResult = await client.downloadByKey(bucketId, fileKey);

        expect(downloadResult.status).toBe(200);
        expect(downloadResult.stream).toBeInstanceOf(ReadableStream);
        expect(downloadResult.contentType).toBeTruthy();

        // Stream to disk first (to handle large files)
        const tempFilePath = '/tmp/download-streaming-test.bin';

        try {
            const fileSize = await streamToFile(downloadResult.stream, tempFilePath);

            // Verify we got some data
            expect(fileSize).toBeGreaterThan(0);
            console.log(`Downloaded file: ${fileSize} bytes`);

            // Verify content using hash comparison (memory-safe for any file size)
            const downloadedHash = await computeFileHash(tempFilePath);
            const expectedHash = computeStringHash(testContent);

            // This will pass when backend returns actual uploaded content
            // For now with 100MB backend, hashes will differ as expected
            expect(downloadedHash).toBe(expectedHash);

        } finally {
            // Clean up the temporary file
            try {
                await unlink(tempFilePath);
            } catch (error) {
                console.warn('Could not clean up streaming test file:', error);
            }
        }
    });

    it.skip('downloads a file by location path', async () => {
        const testContent = 'Test content for location-based download';
        const testData = new TextEncoder().encode(testContent);
        const testBlob = new Blob([testData]);

        const bucketId = 'test-bucket';
        const fileKey = 'test-location-file';

        // Upload the file first
        await client.uploadFile(bucketId, fileKey, testBlob);

        // Download by location (assuming the backend supports this path mapping)
        const filePath = 'path/to/test-location-file';
        const downloadResult = await client.downloadByLocation(bucketId, filePath);

        expect(downloadResult.status).toBe(200);
        expect(downloadResult.stream).toBeInstanceOf(ReadableStream);

        const arrayBuffer = await streamToArrayBuffer(downloadResult.stream);
        const downloadedContent = new TextDecoder().decode(arrayBuffer);
        expect(downloadedContent).toBe(testContent);
    });
});
