import { describe, it, expect } from 'vitest';
import { MspClient } from '../src/index.js';

describe('MspClient.uploadFile (e2e)', () => {
    it('uploads to real backend and returns success JSON', async () => {
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
});


