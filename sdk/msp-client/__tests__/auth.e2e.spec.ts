import { describe, it, expect } from 'vitest';
import { MspClient } from '../src/index.js';

describe('MspClient.auth (e2e)', () => {
    it('gets nonce, verifies signature, sets token, and uploads a file', async () => {
        const baseUrl = process.env.MSP_BASE_URL ?? 'http://127.0.0.1:8080';
        const client = await MspClient.connect({ baseUrl, timeoutMs: 15000 });

        const address = '0x1234567890123456789012345678901234567890';
        const chainId = Number(process.env.MSP_CHAIN_ID ?? 1);

        const { message, nonce } = await client.getNonce(address, chainId);
        expect(typeof message).toBe('string');
        expect(typeof nonce).toBe('string');

        // Mock signature accepted by backend in mock mode: 132-char 0x-hex string
        const mockSignature = '0x' + '1234567890abcdef'.repeat(8) + '12';
        const { token, user } = await client.verify(message, mockSignature);
        expect(typeof token).toBe('string');
        expect(user?.address?.toLowerCase?.()).toBe(address.toLowerCase());

        console.log('Auth token:', token);
        client.setToken(token);

        // Use token on a protected endpoint: upload
        const bytes = new TextEncoder().encode('auth flow upload');
        const res = await client.uploadFile('my-test-bucket', 'auth-test-file', bytes);
        expect(res?.status).toBe('upload_successful');
    }, 30000);
});


