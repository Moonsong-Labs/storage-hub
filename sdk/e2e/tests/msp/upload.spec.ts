import { test, expect } from '@playwright/test';

async function getClient() {
    const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
    const { MspClient } = await import('@storagehub-sdk/msp-client');
    return (MspClient as any).connect({ baseUrl: MSP_BASE_URL, timeoutMs: 120000 });
}

async function getAuthToken() {
    const MSP_CHAIN_ID = Number(process.env.MSP_CHAIN_ID || 1);
    const { LocalWallet } = await import('@storagehub-sdk/core');
    const mnemonic = 'test test test test test test test test test test test junk';
    const wallet = (LocalWallet as any).fromMnemonic(mnemonic);
    const address = await wallet.getAddress();
    const client = await getClient();
    const { message } = await client.getNonce(address, MSP_CHAIN_ID);
    const signature = await wallet.signMessage(message);
    const { token } = await client.verify(message, signature);
    return token as string;
}

test('MSP upload small generated file', async () => {
    const client = await getClient();
    const token = await getAuthToken();
    client.setToken(token);

    const bucketId = 'e2e-bucket';
    const fileKey = `small-${Date.now()}`;
    const data = new TextEncoder().encode('hello from e2e upload');

    const res = await client.uploadFile(bucketId, fileKey, data);
    expect(res?.status).toBe('upload_successful');
    expect(res.bucketId).toBe(bucketId);
    expect(res.fileKey).toBe(fileKey);
    expect(typeof res.fingerprint).toBe('string');
});

test('MSP upload large 50MB generated Blob', async () => {
    const client = await getClient();
    const token = await getAuthToken();
    client.setToken(token);

    const bucketId = 'e2e-bucket';
    const fileKey = `large-${Date.now()}`;

    const targetSizeMB = 50;
    const chunkSizeMB = 1;
    const chunk = new Uint8Array(chunkSizeMB * 1024 * 1024).fill(65);
    const chunks: Uint8Array[] = Array.from({ length: targetSizeMB }, () => chunk);
    const largeBlob = new Blob(chunks);

    const start = Date.now();
    const res = await client.uploadFile(bucketId, fileKey, largeBlob);
    const elapsed = Date.now() - start;
    console.log(`Large upload completed in ${elapsed}ms`);

    expect(res?.status).toBe('upload_successful');
    expect(res.bucketId).toBe(bucketId);
    expect(res.fileKey).toBe(fileKey);
    expect(typeof res.fingerprint).toBe('string');
});


