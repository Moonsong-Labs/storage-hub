import { test as base } from '@playwright/test';
import { bootstrap, MetaMaskWallet } from '@tenkeylabs/dappwright';
import type { BrowserContext, Page } from '@playwright/test';

// Hardhat's default mnemonic: https://hardhat.org/hardhat-network/docs/reference#accounts
const DEFAULT_MNEMONIC = 'test test test test test test test test test test test junk';

export const test = base.extend<
  {
    wallet: MetaMaskWallet;
    page: Page;
  },
  {
    walletContext: BrowserContext;
  }
>({
  // Worker fixture - creates MetaMask context once per worker
  walletContext: [
    async ({ }, use: (context: BrowserContext) => Promise<void>) => {
      console.log('🚀 Setting up MetaMask with dappwright...');

      const [wallet, initialPage, context] = await bootstrap('chromium', {
        wallet: 'metamask',
        version: MetaMaskWallet.recommendedVersion,
        seed: process.env.METAMASK_SEED ?? DEFAULT_MNEMONIC,
        headless: false,
        defaultNetwork: {
          networkName: 'Hardhat',
          rpcUrl: 'http://127.0.0.1:8545',
          chainId: 31337,
          symbol: 'ETH',
        },
        bypassWelcomeScreen: true,
      });

      console.log('✅ MetaMask bootstrap complete');

      // Store wallet instance on context (simple approach)
      (context as any)._walletInstance = wallet;

      await use(context);
      await context.close();
    },
    { scope: 'worker' },
  ],

  // Test fixture - get wallet from context
  wallet: async ({ walletContext }, use) => {
    const wallet = (walletContext as any)._walletInstance;
    await use(wallet);
  },

  // Test fixture - create new page from MetaMask context
  page: async ({ walletContext }, use) => {
    const page = await walletContext.newPage();
    await use(page);
    await page.close();
  },
});

export { expect } from '@playwright/test';