import { test, expect } from '@playwright/test';

test('MSP auth with LocalWallet and protected upload', async () => {
    const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
    const MSP_CHAIN_ID = Number(process.env.MSP_CHAIN_ID || 1);

    const { MspClient } = await import('@storagehub-sdk/msp-client');
    const { LocalWallet } = await import('@storagehub-sdk/core');
    const client = await (MspClient as any).connect({ baseUrl: MSP_BASE_URL, timeoutMs: 15000 });

    const mnemonic = 'test test test test test test test test test test test junk';
    const wallet = (LocalWallet as any).fromMnemonic(mnemonic);
    const address = await wallet.getAddress();

    // Get nonce & message to sign
    const { message, nonce } = await client.getNonce(address, MSP_CHAIN_ID);
    expect(typeof message).toBe('string');
    expect(typeof nonce).toBe('string');

    // Sign with LocalWallet
    const signature = await wallet.signMessage(message);
    expect(typeof signature).toBe('string');

    // Verify and get token
    const { token, user } = await client.verify(message, signature);
    expect(typeof token).toBe('string');
    expect(user?.address).toBe(address);

    client.setToken(token);

    // Protected endpoint: small upload with generated data
    const bucketId = 'wallet-test-bucket';
    const fileKey = `localwallet-file-${Date.now()}`;
    const bytes = new TextEncoder().encode('local wallet auth flow upload');
    const res = await client.uploadFile(bucketId, fileKey, bytes);
    expect(res?.status).toBe('upload_successful');
    expect(res.bucketId).toBe(bucketId);
    expect(res.fileKey).toBe(fileKey);
});


