import { describe, it, expect } from 'vitest';
import { createPublicClient, createWalletClient, defineChain, http, getContract, parseGwei, toHex } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { FileSystemClient } from '../src/evm/filesystemClient.js';
import { ALITH } from './consts.js';

// const RPC_URL = 'http://127.0.0.1:9944' as const;
const RPC_URL = 'http://127.0.0.1:9888' as const;

// Dev keys for SH local dev (same pair used in evmTransfer test)
const ALITH_PRIVATE_KEY = '0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133' as `0x${string}`;

describe('FileSystem precompile - createBucket', () => {
  it.skip('creates a bucket from ALITH (manual gas override)', async () => {
    const chain = defineChain({
      id: 181222,
      name: 'SH-EVM_SOLO',
      nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
      rpcUrls: { default: { http: [RPC_URL] } },
    });

    const account = privateKeyToAccount(ALITH_PRIVATE_KEY);
    const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
    const walletClient = createWalletClient({ chain, account, transport: http(RPC_URL) });

    const fs = new FileSystemClient({ client: walletClient });

    // Hardcoded from network bootstrap
    const mspId = '0x0000000000000000000000000000000000000000000000000000000000000300' as `0x${string}`;
    const valuePropId = '0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b' as `0x${string}`;
    const bucketName = new TextEncoder().encode(`sdk-bucket-${Math.floor(Math.random() * 1e6)}`);

    const isPrivate: boolean = false;

    // Bypass wrapper for this call to control gas/fees explicitly
    const contract = getContract({
      address: '0x0000000000000000000000000000000000000064',
      abi: (await import('../src/abi/filesystem.js')).filesystemAbi,
      client: walletClient,
    });

    const nameHex = toHex(bucketName);
    const args = [mspId, nameHex, isPrivate, valuePropId] as const;
    const gasEst = await publicClient.estimateContractGas({
      address: contract.address,
      abi: (await import('../src/abi/filesystem.js')).filesystemAbi,
      functionName: 'createBucket',
      args,
      account: account.address,
    });
    const gas = gasEst * 5n; // headroom for Frontier weight→gas mapping

    // @ts-ignore viem write is present at runtime
    const txHash = await contract.write.createBucket(args, {
      gas,
      gasPrice: parseGwei('1'),
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
    expect(receipt.status).toBe('success');
  }, 60_000);

  // Test SDK with automatic gas estimation (no options needed)
  it.only('creates a bucket from ALITH (automatic gas)', async () => {
    const chain = defineChain({
      id: 181222,
      name: 'SH-EVM_SOLO',
      nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
      rpcUrls: { default: { http: [RPC_URL] } },
    });

    const account = privateKeyToAccount(ALITH_PRIVATE_KEY);
    const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
    const walletClient = createWalletClient({ chain, account, transport: http(RPC_URL) });

    const fs = new FileSystemClient({ client: walletClient, publicClient });

    const mspId = '0x0000000000000000000000000000000000000000000000000000000000000300' as `0x${string}`;
    const valuePropId = '0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b' as `0x${string}`;
    const bucketName = new TextEncoder().encode(`sdk-bucket-${Math.floor(Math.random() * 1e6)}`);

    console.log(`[TEST DEBUG] Using SDK with ZERO gas parameters - pure automatic mode...`);

    // Cleanest possible call - SDK handles everything automatically
    const txHash = await fs.createBucket(mspId, bucketName, false, valuePropId);
    const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
    expect(receipt.status).toBe('success');
  }, 60_000);

  // Test SDK with custom gas options (user override)
  it.skip('creates a bucket from ALITH (custom gas options)', async () => {
    const chain = defineChain({
      id: 181222,
      name: 'SH-EVM_SOLO',
      nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
      rpcUrls: { default: { http: [RPC_URL] } },
    });

    const account = privateKeyToAccount(ALITH_PRIVATE_KEY);
    const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
    const walletClient = createWalletClient({ chain, account, transport: http(RPC_URL) });

    const fs = new FileSystemClient({ client: walletClient, publicClient });

    const mspId = '0x0000000000000000000000000000000000000000000000000000000000000300' as `0x${string}`;
    const valuePropId = '0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b' as `0x${string}`;
    const bucketName = new TextEncoder().encode(`sdk-bucket-${Math.floor(Math.random() * 1e6)}`);

    console.log(`[TEST DEBUG] Using SDK with CUSTOM gas options...`);

    // Custom gas multiplier and pricing
    const txHash = await fs.createBucket(mspId, bucketName, false, valuePropId, {
      gasMultiplier: 8,  // Higher safety margin
      gasPrice: parseGwei('2')  // Higher gas price
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
    expect(receipt.status).toBe('success');
  }, 60_000);
});


