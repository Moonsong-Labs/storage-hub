import { test, expect } from '@playwright/test';

async function getClient() {
    const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';
    const { MspClient } = await import('@storagehub-sdk/msp-client');
    return (MspClient as any).connect({ baseUrl: MSP_BASE_URL, timeoutMs: 60000 });
}

async function authClient(client: any) {
    const MSP_CHAIN_ID = Number(process.env.MSP_CHAIN_ID || 1);
    const { LocalWallet } = await import('@storagehub-sdk/core');
    const mnemonic = 'test test test test test test test test test test test junk';
    const wallet = (LocalWallet as any).fromMnemonic(mnemonic);
    const address = await wallet.getAddress();
    const { message } = await client.getNonce(address, MSP_CHAIN_ID);
    const signature = await wallet.signMessage(message);
    const { token } = await client.verify(message, signature);
    client.setToken(token);
}

test('MSP upload then download by key (stream to /tmp and verify)', async () => {
    const client = await getClient();
    await authClient(client);

    const bucketId = 'e2e-bucket';
    const fileKey = `dl-${Date.now()}`;
    const bytes = new TextEncoder().encode('downloadable content');
    const up = await client.uploadFile(bucketId, fileKey, bytes);
    expect(up?.status).toBe('upload_successful');

    const dl = await client.downloadByKey(bucketId, fileKey);
    expect(dl.status).toBe(200);
    expect(dl.stream).toBeInstanceOf(ReadableStream);

    const tempPath = `/tmp/${fileKey}.bin`;
    const reader = dl.stream.getReader();
    const file = await (await import('fs/promises')).open(tempPath, 'w');
    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            await file.write(Buffer.from(value));
        }
    } finally {
        await file.close();
    }

    const { stat, unlink } = await import('fs/promises');
    const stats = await stat(tempPath);
    expect(stats.isFile()).toBe(true);
    expect(stats.size).toBeGreaterThan(0);

    await unlink(tempPath).catch(() => { });
});


