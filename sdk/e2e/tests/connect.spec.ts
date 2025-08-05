import { BrowserContext, expect, test as baseTest } from "@playwright/test";
import dappwright, { Dappwright, MetaMaskWallet } from "@tenkeylabs/dappwright";

export const test = baseTest.extend<{
    context: BrowserContext;
    wallet: Dappwright;
}>({
    context: async ({ }, use) => {
        // Launch context with extension
        const [wallet, _, context] = await dappwright.bootstrap("", {
            wallet: "metamask",
            version: MetaMaskWallet.recommendedVersion,
            seed: "test test test test test test test test test test test junk", // Hardhat's default https://hardhat.org/hardhat-network/docs/reference#accounts
            headless: false,
        });

        // Add Hardhat as a custom network
        await wallet.addNetwork({
            networkName: "Hardhat",
            rpc: "http://localhost:8545", // Fixed: using 8545 (Anvil default) instead of 8546
            chainId: 31337,
            symbol: "ETH",
        });

        await use(context);
    },

    wallet: async ({ context }, use) => {
        const metamask = await dappwright.getWallet("metamask", context);

        await use(metamask);
    },
});

test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173/basic.html"); // Using the correct URL for your setup
});

test("should be able to connect", async ({ wallet, page }) => {
    await page.click("#connectButton");
    await wallet.approve();

    const connectStatus = page.getByTestId("connect-status");
    await expect(connectStatus).toHaveValue("connected");

    await page.click("#switch-network-button");

    // Wait a moment for the network switch to complete
    await page.waitForTimeout(2000);

    const networkStatus = page.getByTestId("network-status");
    await expect(networkStatus).toHaveValue("31337");
});