import { describe, it, expect } from 'vitest';
import { StorageHubClient } from '../index.js';

// Dummy test to satisfy Vitest until real client-side tests are implemented.
describe('StorageHubClient', () => {
  it('connect() should return a StorageHubClient instance', async () => {
    const client = await StorageHubClient.connect();
    expect(client).toBeInstanceOf(StorageHubClient);
  });
});
