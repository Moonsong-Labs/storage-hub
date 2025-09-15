import { describe, it, expect } from 'vitest';
import { createPublicClient, createWalletClient, defineChain, http, parseEther } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';

// RPC endpoint for your local EVM node
const RPC_URL = 'http://127.0.0.1:9944' as const;

const ALITH_PRIVATE_KEY = '0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133' as `0x${string}`;
const CHARLETH_PRIVATE_KEY = '0x0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b' as `0x${string}`;

describe('EVM fund transfer (local node @ 127.0.0.1:9944)', () => {
  it('transfers 100 tokens from ALITH to CHARLETH', async () => {
    const chain = defineChain({
      id: 181222,
      name: 'SH-EVM_SOLO',
      nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
      rpcUrls: { default: { http: [RPC_URL] } },
    });

    const sender = privateKeyToAccount(CHARLETH_PRIVATE_KEY);
    const receiver = privateKeyToAccount(ALITH_PRIVATE_KEY);

    const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
    const walletClient = createWalletClient({ chain, account: sender, transport: http(RPC_URL) });

    const value = parseEther('100');

    const receiverBefore = await publicClient.getBalance({ address: receiver.address });

    const txHash = await walletClient.sendTransaction({ to: receiver.address, value });
    const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
    expect(receipt.status).toBe('success');

    const receiverAfter = await publicClient.getBalance({ address: receiver.address });
    expect(receiverAfter - receiverBefore >= value).toBe(true);
  }, 60_000);
});


