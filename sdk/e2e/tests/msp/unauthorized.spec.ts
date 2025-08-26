import { test, expect } from '@playwright/test';

test('MSP unauthorized upload and download fail', async () => {
    const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
    const { MspClient } = await import('@storagehub-sdk/msp-client');
    const client = await (MspClient as any).connect({ baseUrl: MSP_BASE_URL, timeoutMs: 15000 });

    // Upload without token
    const bucketId = 'e2e-bucket';
    const fileKey = `unauth-${Date.now()}`;
    const bytes = new TextEncoder().encode('unauthorized attempt');

    let uploadErr: any = null;
    try {
        await client.uploadFile(bucketId, fileKey, bytes);
    } catch (e) {
        uploadErr = e;
    }
    expect(uploadErr).toBeTruthy();

    // Download without token (expect error or non-200)
    let dlStatus = 0;
    try {
        const dl = await client.downloadByKey(bucketId, fileKey);
        dlStatus = dl?.status || 0;
    } catch {
        dlStatus = 0;
    }
    expect(dlStatus).not.toBe(200);
});


