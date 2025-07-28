import { describe, it, expect } from 'vitest';
import { MspClient } from '../index.js';

// Dummy test to satisfy Vitest until real client-side tests are implemented.
describe('MspClient', () => {
  it('connect() should return an MspClient instance', async () => {
    const client = await MspClient.connect();
    expect(client).toBeInstanceOf(MspClient);
  });
});
