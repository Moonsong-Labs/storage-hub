import { describe, it, expect } from 'vitest';
import { MspClient } from '../src/index.js';

describe('MspClient.uploadFile (e2e)', () => {
    it('uploads small file to backend successfully', async () => {
        const baseUrl = process.env.MSP_BASE_URL ?? 'http://127.0.0.1:8080';
        const token = process.env.MSP_TOKEN ?? 'test-token-123';

        const client = await MspClient.connect({
            baseUrl,
            defaultHeaders: { Authorization: `Bearer ${token}` },
            timeoutMs: 15000,
        });

        const bytes = new TextEncoder().encode('hello from test');
        const res = await client.uploadFile('my-test-bucket', 'my-test-file', bytes);
        console.log('Upload response from backend:', res);

        expect(res).toBeDefined();
        expect(res.status).toBe('upload_successful');
        expect(res.bucketId).toBe('my-test-bucket');
        expect(res.fileKey).toBe('my-test-file');
        expect(typeof res.fingerprint).toBe('string');
    }, 30000);

    it('uploads large file (50MB) to backend successfully', async () => {
        const baseUrl = process.env.MSP_BASE_URL ?? 'http://127.0.0.1:8080';
        const token = process.env.MSP_TOKEN ?? 'test-token-123';

        const client = await MspClient.connect({
            baseUrl,
            defaultHeaders: { Authorization: `Bearer ${token}` },
            timeoutMs: 120000, // 2 minutes for large file upload
        });

        // Backend supports large files - testing with 50MB to verify large file upload capability
        const targetSizeMB = 50;
        const chunkSizeKB = 1024; // 1MB chunks for memory efficiency
        const chunksNeeded = targetSizeMB;

        console.log(`Creating ${targetSizeMB}MB test file...`);

        // Create file using simple pattern repetition
        const chunkData = new Uint8Array(chunkSizeKB * 1024).fill(65); // Fill with 'A' (ASCII 65)
        const fileChunks = Array(chunksNeeded).fill(chunkData);
        const largeFile = new Blob(fileChunks);

        console.log(`Created ${(largeFile.size / (1024 * 1024)).toFixed(1)}MB file (${largeFile.size} bytes)`);
        console.log('Starting upload...');

        const startTime = Date.now();
        const res = await client.uploadFile('large-test-bucket', 'large-test-file-50mb', largeFile);
        const uploadTime = Date.now() - startTime;

        console.log(`Upload completed in ${uploadTime}ms (${(uploadTime / 1000).toFixed(1)}s)`);
        console.log('Large file upload response:', res);

        // Verify successful upload
        expect(res).toBeDefined();
        expect(res.status).toBe('upload_successful');
        expect(res.bucketId).toBe('large-test-bucket');
        expect(res.fileKey).toBe('large-test-file-50mb');
        expect(typeof res.fingerprint).toBe('string');
        expect(res.fingerprint.length).toBeGreaterThan(0);

        // Performance check - should complete within reasonable time (5 seconds)
        expect(uploadTime).toBeLessThan(5000);
    }, 60000); // 1 minute timeout for large file test
});


