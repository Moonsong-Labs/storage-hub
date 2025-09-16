import { describe, it, expect } from 'vitest';
import { createPublicClient, createWalletClient, defineChain, http, parseGwei } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { StorageHubClient } from '../src/evm/storageHubClient.js';
import { ALITH } from './consts.js';

const RPC_URL = 'http://127.0.0.1:9888' as const;

// Test constants from network bootstrap
const TEST_MSP_ID = '0x0000000000000000000000000000000000000000000000000000000000000300' as `0x${string}`;
const TEST_VALUE_PROP_ID = '0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b' as `0x${string}`;

// Test timeout for EVM operations (60 seconds)
const EVM_TEST_TIMEOUT = 60_000;

// Common test setup
const createTestSetup = () => {
  const chain = defineChain({
    id: 181222,
    name: 'SH-EVM_SOLO',
    nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
    rpcUrls: { default: { http: [RPC_URL] } },
  });

  const account = privateKeyToAccount(ALITH.privateKey);
  const walletClient = createWalletClient({ chain, account, transport: http(RPC_URL) });
  const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });

  const hub = new StorageHubClient({
    rpcUrl: RPC_URL,
    chain,
    walletClient
  });

  return {
    chain,
    account,
    walletClient,
    publicClient,
    hub,
    mspId: TEST_MSP_ID,
    valuePropId: TEST_VALUE_PROP_ID
  };
};

describe('StorageHub EVM Integration', () => {
  describe('createBucket', () => {

    it.only('should create bucket with automatic gas estimation', async () => {
      const { hub, publicClient, mspId, valuePropId } = createTestSetup();
      const bucketName = `auto-${Math.floor(Math.random() * 1e6)}`;

      console.log(`[TEST] Creating bucket with automatic gas estimation...`);

      // Cleanest possible call - SDK handles everything automatically
      const txHash = await hub.createBucket(mspId, bucketName, false, valuePropId);
      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });

      expect(receipt.status).toBe('success');
      console.log(`[TEST] Bucket created successfully! TxHash: ${txHash}`);
    }, EVM_TEST_TIMEOUT);

    it.skip('should create bucket with custom gas options', async () => {
      const { hub, publicClient, mspId, valuePropId } = createTestSetup();
      const bucketName = `custom-${Math.floor(Math.random() * 1e6)}`;

      console.log(`[TEST] Creating bucket with custom gas options...`);

      // Custom gas multiplier and pricing
      const txHash = await hub.createBucket(mspId, bucketName, false, valuePropId, {
        gasMultiplier: 8,  // Higher safety margin
        gasPrice: parseGwei('2')  // Higher gas price
      });

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      expect(receipt.status).toBe('success');
      console.log(`[TEST] Bucket created with custom gas! TxHash: ${txHash}`);
    }, EVM_TEST_TIMEOUT);

  });

  describe('updateBucketPrivacy', () => {
    it.skip('should update bucket privacy setting', async () => {
      const { hub, publicClient, mspId, valuePropId } = createTestSetup();

      // First create a bucket
      const bucketName = `privacy-${Math.floor(Math.random() * 1e6)}`;
      const initialBucketPrivacy: boolean = false;
      const createTxHash = await hub.createBucket(mspId, bucketName, initialBucketPrivacy, valuePropId);
      await publicClient.waitForTransactionReceipt({ hash: createTxHash });

      // Get the bucket ID
      const bucketId = await hub.deriveBucketId(ALITH.address, bucketName);

      console.log(`[TEST] Updating bucket privacy...`);

      // Toggle privacy
      const updateTxHash = await hub.updateBucketPrivacy(bucketId, !initialBucketPrivacy);
      const receipt = await publicClient.waitForTransactionReceipt({ hash: updateTxHash });

      expect(receipt.status).toBe('success');
      console.log(`[TEST] Bucket privacy updated! TxHash: ${updateTxHash}`);
    }, EVM_TEST_TIMEOUT);
  });

  describe('deleteBucket', () => {
    it.skip('should delete an empty bucket', async () => {
      const { hub, publicClient, mspId, valuePropId } = createTestSetup();

      // First create a bucket
      const bucketName = `delete-${Math.floor(Math.random() * 1e6)}`;
      const createTxHash = await hub.createBucket(mspId, bucketName, false, valuePropId);
      await publicClient.waitForTransactionReceipt({ hash: createTxHash });

      // Get the bucket ID
      const bucketId = await hub.deriveBucketId(ALITH.address, bucketName);

      console.log(`[TEST] Deleting bucket...`);

      // Delete the bucket
      const deleteTxHash = await hub.deleteBucket(bucketId);
      const receipt = await publicClient.waitForTransactionReceipt({ hash: deleteTxHash });

      expect(receipt.status).toBe('success');
      console.log(`[TEST] Bucket deleted! TxHash: ${deleteTxHash}`);
    }, EVM_TEST_TIMEOUT);
  });

  describe('createAndAssociateCollectionWithBucket', () => {
    it.skip('should create and associate collection with bucket', async () => {
      const { hub, publicClient, mspId, valuePropId } = createTestSetup();

      // First create a bucket
      const bucketName = `collection-${Math.floor(Math.random() * 1e6)}`;
      const createTxHash = await hub.createBucket(mspId, bucketName, false, valuePropId);
      await publicClient.waitForTransactionReceipt({ hash: createTxHash });

      // Get the bucket ID
      const bucketId = await hub.deriveBucketId(ALITH.address, bucketName);

      console.log(`[TEST] Creating collection for bucket...`);

      // Create and associate collection
      const collectionTxHash = await hub.createAndAssociateCollectionWithBucket(bucketId);
      const receipt = await publicClient.waitForTransactionReceipt({ hash: collectionTxHash });

      expect(receipt.status).toBe('success');
      console.log(`[TEST] Collection created and associated! TxHash: ${collectionTxHash}`);
    }, EVM_TEST_TIMEOUT);
  });

  describe('read operations', () => {
    it.skip('should derive bucket ID from owner and name', async () => {
      const { hub } = createTestSetup();
      const bucketName = 'test-bucket';

      console.log(`[TEST] Deriving bucket ID...`);

      const bucketId = await hub.deriveBucketId(ALITH.address, bucketName);

      expect(bucketId).toMatch(/^0x[a-fA-F0-9]{64}$/); // 32-byte hex string
      console.log(`[TEST] Bucket ID derived: ${bucketId}`);
    });

    it.skip('should get pending file deletion requests count', async () => {
      const { hub } = createTestSetup();

      console.log(`[TEST] Getting pending deletion requests...`);

      const count = await hub.getPendingFileDeletionRequestsCount(ALITH.address);

      expect(typeof count).toBe('number');
      expect(count).toBeGreaterThanOrEqual(0);
      console.log(`[TEST] Pending deletion requests: ${count}`);
    });
  });

  describe('error handling', () => {
    it.skip('should validate bucket name length', async () => {
      const { hub, mspId, valuePropId } = createTestSetup();

      // Create a bucket name that exceeds 100 bytes
      const longName = 'a'.repeat(101);

      console.log(`[TEST] Testing bucket name validation...`);

      await expect(
        hub.createBucket(mspId, longName, false, valuePropId)
      ).rejects.toThrow('exceeds maximum length of 100 bytes');

      console.log(`[TEST] Bucket name validation works correctly`);
    });

    it.skip('should handle invalid hex addresses', async () => {
      const { hub, valuePropId } = createTestSetup();
      const bucketName = 'test';

      console.log(`[TEST] Testing invalid address handling...`);

      // TypeScript should catch this at compile time, but test runtime behavior
      const invalidMspId = 'not-a-hex-address' as `0x${string}`;

      await expect(
        hub.createBucket(invalidMspId, bucketName, false, valuePropId)
      ).rejects.toThrow();

      console.log(`[TEST] Invalid address handling works correctly`);
    });
  });
});


