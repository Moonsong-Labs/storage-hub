import { test as base } from '@playwright/test';
import { bootstrap, MetaMaskWallet } from '@tenkeylabs/dappwright';
import path from 'node:path';

export const test = base.extend<{ wallet: MetaMaskWallet }>({
    wallet: [
        async ({ browser }, use) => {
            const [wallet] = await bootstrap(browser, {
                wallet: 'metamask',
                version: MetaMaskWallet.recommendedVersion,
                seed: process.env.METAMASK_SEED || // Hardhat default account 0
                    'test test test test test test test test test test test junk',
                headless: true,
                defaultNetwork: {
                    name: 'Hardhat',
                    rpc: 'http://127.0.0.1:8545',
                    chainId: 31337,
                },
                bypassWelcomeScreen: true,
            });
            await use(wallet as MetaMaskWallet);
        },
        { scope: 'worker' },
    ],
});

export { expect } from '@playwright/test';