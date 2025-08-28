import { MspClient } from '../index.js';
import { describe, expect, it } from 'vitest';

// Dummy test to satisfy Vitest until real client-side tests are implemented.
describe('MspClient', () => {
  it('connect() should return an MspClient instance', async () => {
    const client = await MspClient.connect();
    expect(client).toBeInstanceOf(MspClient);
  });
});
