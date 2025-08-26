import { test, expect } from '@playwright/test';

test('MSP health endpoint returns valid component statuses', async () => {
    const MSP_BASE_URL = process.env.MSP_BASE_URL || 'http://127.0.0.1:8080';

    const { MspClient } = await import('../../../../sdk/msp-client/dist/index.js').catch(() => import('../../../msp-client/dist/index.js'));
    const client = await (MspClient as any).connect({ baseUrl: MSP_BASE_URL, timeoutMs: 5000 });

    const health = await client.getHealth();
    expect(health).toBeDefined();

    const allowed = new Set(['healthy', 'unhealthy', 'degraded', 'unknown']);
    expect(allowed.has(health.status)).toBeTruthy();

    for (const comp of Object.values(health.components || {})) {
        expect(allowed.has((comp as any).status)).toBeTruthy();
    }
});


