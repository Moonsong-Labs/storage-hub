import { describe, it, expect, beforeAll } from 'vitest';
import { MspClient } from '../src/MspClient';

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

const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
const MSP_TOKEN = process.env.MSP_TOKEN || 'test';

describe('MspClient.download (e2e)', () => {
    let client: MspClient;

    beforeAll(async () => {
        client = await MspClient.connect({ baseUrl: MSP_BASE_URL });
        client.setToken(MSP_TOKEN);
    });

    it('downloads mock content from backend (verifies endpoint works)', async () => {
        // Test the download endpoint with mock backend
        const downloadResult = await client.downloadByKey('test-bucket', 'any-file-key');

        expect(downloadResult.status).toBe(200);
        expect(downloadResult.stream).toBeInstanceOf(ReadableStream);
        expect(downloadResult.contentType).toBeTruthy();

        // Verify we can stream the mock content
        const arrayBuffer = await streamToArrayBuffer(downloadResult.stream);
        const downloadedContent = new TextDecoder().decode(arrayBuffer);

        // Backend is in mock mode, so expect mock content
        expect(downloadedContent).toBe('Mock file content for download');
        expect(arrayBuffer.byteLength).toBe(30); // Length of mock content

        console.log('Mock download successful:', downloadedContent);
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

        // Test streaming by reading chunks
        const reader = downloadResult.stream.getReader();
        const chunks: Uint8Array[] = [];
        let totalBytes = 0;

        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                expect(value).toBeInstanceOf(Uint8Array);
                chunks.push(value);
                totalBytes += value.length;
                console.log(`Downloaded chunk: ${value.length} bytes`);
            }
        } finally {
            reader.releaseLock();
        }

        // Verify we got some data
        expect(totalBytes).toBeGreaterThan(0);
        expect(chunks.length).toBeGreaterThan(0);

        // Combine chunks and verify content
        const result = new Uint8Array(totalBytes);
        let offset = 0;
        for (const chunk of chunks) {
            result.set(chunk, offset);
            offset += chunk.length;
        }

        const downloadedContent = new TextDecoder().decode(result);
        expect(downloadedContent).toBe(testContent);
    });

    it.skip('downloads a file using streamToArrayBuffer helper', async () => {
        const testContent = 'Test content for ArrayBuffer conversion';
        const testData = new TextEncoder().encode(testContent);
        const testBlob = new Blob([testData]);

        const bucketId = 'test-bucket';
        const fileKey = 'test-arraybuffer-file';

        // Upload the file first
        await client.uploadFile(bucketId, fileKey, testBlob);

        // Download using streaming, then convert to ArrayBuffer
        const downloadResult = await client.downloadByKey(bucketId, fileKey);
        const arrayBuffer = await streamToArrayBuffer(downloadResult.stream);

        expect(arrayBuffer).toBeInstanceOf(ArrayBuffer);
        expect(arrayBuffer.byteLength).toBeGreaterThan(0);

        const downloadedContent = new TextDecoder().decode(arrayBuffer);
        expect(downloadedContent).toBe(testContent);
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

    it.skip('downloads full file content correctly', async () => {
        const testContent = 'This is a longer test file for range download testing with multiple bytes';
        const testData = new TextEncoder().encode(testContent);
        const testBlob = new Blob([testData]);

        const bucketId = 'test-bucket';
        const fileKey = 'test-range-file';

        // Upload the file first
        await client.uploadFile(bucketId, fileKey, testBlob);

        // Download the file (range functionality will be added later)
        const downloadResult = await client.downloadByKey(bucketId, fileKey);

        expect(downloadResult.status).toBe(200);
        expect(downloadResult.stream).toBeInstanceOf(ReadableStream);

        const arrayBuffer = await streamToArrayBuffer(downloadResult.stream);
        const downloadedContent = new TextDecoder().decode(arrayBuffer);

        expect(downloadedContent).toBe(testContent);
        expect(arrayBuffer.byteLength).toBe(testData.length);
    });
});
