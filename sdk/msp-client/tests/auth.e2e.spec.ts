import { describe, it, expect } from 'vitest';
import { MspClient } from '../src/index.js';
import { LocalWallet } from '@storagehub-sdk/core';

describe('MspClient.auth (e2e)', () => {
    it('gets nonce, signs with LocalWallet, verifies, and uploads a file', async () => {
        const baseUrl = process.env.MSP_BASE_URL ?? 'http://127.0.0.1:8080';
        const client = await MspClient.connect({ baseUrl, timeoutMs: 15000 });

        const testMnemonic = "test test test test test test test test test test test junk";
        const localWallet = LocalWallet.fromMnemonic(testMnemonic);
        const address = localWallet.address;
        const chainId = Number(process.env.MSP_CHAIN_ID ?? 1);

        console.log(`Using local wallet address: ${address}`);

        // Step 1: Get nonce
        const { message, nonce } = await client.getNonce(address, chainId);
        expect(typeof message).toBe('string');
        expect(typeof nonce).toBe('string');

        console.log("-------------------------------")
        console.log('Message to sign:');
        console.log(message);
        console.log("-------------------------------")
        console.log(`Nonce: ${nonce}`);

        // Step 2: Sign the message 
        const signature = await localWallet.signMessage(message);
        console.log(`Signature: ${signature}`);

        // Step 3: Verify signature and get token
        const { token, user } = await client.verify(message, signature);
        expect(typeof token).toBe('string');
        expect(typeof user?.address).toBe('string');

        console.log('Auth token:', token);
        client.setToken(token);

        // Step 4: Use token on a protected endpoint: upload
        const bytes = new TextEncoder().encode('local wallet auth flow upload');
        const res = await client.uploadFile('wallet-test-bucket', 'local-wallet-file', bytes);
        expect(res?.status).toBe('upload_successful');
        expect(res.bucketId).toBe('wallet-test-bucket');
        expect(res.fileKey).toBe('local-wallet-file');

        console.log('Upload successful with local wallet auth!');
    }, 30000);
});


