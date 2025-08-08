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
            console.log('🚀 Launching browser with MetaMask...');
            const { browserContext } = await dappwright.launch("", {
                wallet: "metamask",
                version: MetaMaskWallet.recommendedVersion,
                headless: process.env.HEADLESS ? process.env.HEADLESS === "true" : false,
            });

            const wallet = await dappwright.getWallet("metamask", browserContext);
            console.log('✅ MetaMask wallet obtained');

            // Setup wallet with seed phrase
            await wallet.setup({
                seed: "test test test test test test test test test test test junk",
                password: "password123",
            });
            console.log('✅ Wallet setup with seed phrase');

            // Cache context
            sharedBrowserContext = browserContext;
        }

        await use(sharedBrowserContext);
    },

    page: async ({ context }, use) => {
        // Create a fresh page and go to our local basic dApp (served from sdk root)
        const page = await context.newPage();
        await page.goto("http://localhost:3000/e2e/page/index.html");
        await use(page);
    },

    wallet: async ({ context }, use) => {
        const metamask = await dappwright.getWallet("metamask", context);
        await use(metamask);
    },
});

test("MetaMask + SDK", async ({ page, wallet, context }) => {
    console.log('🎯 Starting test...');

    // Ensure provider is injected
    await page.waitForLoadState();
    await page.waitForFunction(() => (window as any).ethereum !== undefined, { timeout: 15000 });
    console.log('✅ Provider injected');

    // Click Connect on the basic dApp and approve in MetaMask
    await page.waitForSelector('#connect', { timeout: 15000 });
    await page.click('#connect');
    await wallet.approve();
    console.log('✅ Connection approved');

    // Trigger signing via the dApp's SDK handler by clicking the button
    await page.waitForSelector('#sign:not([disabled])', { timeout: 15000 });
    await page.click('#sign');

    // Approve signature in MetaMask
    await wallet.sign();

    // Wait until the dApp exposes the signature and log it
    const signature = await page.waitForFunction(() => (window as any).__lastSignature, { timeout: 15000 });
    const value = await signature.jsonValue();
    console.log(`✅ Message signed: ${value}`);
});



