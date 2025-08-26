import { describe, it, expect } from 'vitest';
import { MspClient } from '../src/index.js';
import { HealthState } from '../src/types.js';

// Simple e2e-style test. Requires an MSP backend running and reachable.
// Configure base URL via MSP_BASE_URL env var; defaults to http://localhost:8080

describe('MspClient.getHealth (e2e)', () => {
    it('should fetch health from running MSP backend', async () => {
        const allowed = [
            HealthState.Healthy,
            HealthState.Unhealthy,
            HealthState.Degraded,
            HealthState.Unknown,
        ];
        const expectState = (state: unknown) => expect(allowed).toContain(state);

        const baseUrl = process.env.MSP_BASE_URL ?? 'http://localhost:8080';
        const client = await MspClient.connect({ baseUrl, timeoutMs: 5000 });
        const health = await client.getHealth();

        expect(health).toBeDefined();
        expectState(health.status);

        for (const [_, comp] of Object.entries(health.components || {})) {
            expectState((comp as any)?.status);
        }

        console.log('MSP Health:');
        console.dir(health, { depth: null, colors: true });
    }, 15000);
});


