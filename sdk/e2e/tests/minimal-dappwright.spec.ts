import { type BrowserContext, type Page, test as baseTest } from "@playwright/test";
import dappwright, { type Dappwright, MetaMaskWallet } from "@tenkeylabs/dappwright";

export { expect } from "@playwright/test";

let sharedBrowserContext: BrowserContext;

export const test = baseTest.extend<{
    context: BrowserContext;
    wallet: Dappwright;
    page: Page;
}>({
    context: async ({ }, use) => {
        if (!sharedBrowserContext) {
            console.log('🚀 Launching dappwright with MetaMask...');

            // Use the exact ZKSync pattern - dappwright.launch instead of bootstrap
            const { browserContext } = await dappwright.launch("", {
                wallet: "metamask",
                version: MetaMaskWallet.recommendedVersion,
                headless: process.env.HEADLESS ? process.env.HEADLESS === "true" : false,
            });

            const wallet = await dappwright.getWallet("metamask", browserContext);
            console.log('✅ MetaMask wallet obtained');

            // Override waitForTimeout to speed up setup (ZKSync approach)
            const originalWaitForTimeout = wallet.page.waitForTimeout;
            wallet.page.waitForTimeout = async (_ms: number) => { };

            // Setup wallet with seed phrase
            await wallet.setup({
                seed: "test test test test test test test test test test test junk",
                password: "password123",
            });
            console.log('✅ Wallet setup with seed phrase');

            try {
                // Add Anvil network (like ZKSync adds Hardhat)
                await wallet.addNetwork({
                    networkName: "Anvil Testnet",
                    rpc: "http://127.0.0.1:8545",
                    chainId: 31337,
                    symbol: "ETH",
                });
                console.log('✅ Anvil network added');

                // Switch to Anvil network
                await wallet.switchNetwork("Anvil Testnet");
                console.log('✅ Switched to Anvil network');
            } catch (e) {
                console.error('❌ Network setup failed:', e);
                throw new Error("Please verify there's an Anvil node running at http://localhost:8545");
            }

            // Restore original waitForTimeout
            wallet.page.waitForTimeout = originalWaitForTimeout;

            // Cache context
            sharedBrowserContext = browserContext;
        }

        await use(sharedBrowserContext);
    },

    page: async ({ context }, use) => {
        // Create a fresh page and go to our local basic dApp
        const page = await context.newPage();
        await page.goto("http://localhost:3000");
        await use(page);
    },

    wallet: async ({ context }, use) => {
        const metamask = await dappwright.getWallet("metamask", context);
        await use(metamask);
    },
});

test("Minimal MetaMask + Anvil Test", async ({ page, wallet, context }) => {
    console.log('🎯 Starting minimal test...');

    // Ensure provider is injected
    await page.waitForLoadState();
    await page.waitForFunction(() => (window as any).ethereum !== undefined, { timeout: 15000 });
    console.log('✅ Provider injected');

    // Click Connect on the basic dApp and approve in MetaMask
    await page.waitForSelector('#connect', { timeout: 15000 });
    await page.click('#connect');
    await wallet.approve();
    console.log('✅ Connection approved');

    // Wait for Sign button to be enabled (basic dApp enables it after connection)
    await page.waitForSelector('#sign:not([disabled])', { timeout: 15000 });
    await page.click('#sign');

    // Add a 3 seconds delay to allow the MetaMask signature popup to fully render
    // await new Promise((resolve) => setTimeout(resolve, 3000));

    // Approve signature in MetaMask
    await wallet.sign();
    // Wait until the dApp exposes the signature and log it
    const signature = await page.waitForFunction(() => (window as any).__lastSignature, { timeout: 15000 });
    const value = await signature.jsonValue();
    console.log(`✅ Message signed: ${value}`);
});
