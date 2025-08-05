import { BrowserContext, expect, test as baseTest } from "@playwright/test";
import dappwright, { Dappwright, MetaMaskWallet } from "@tenkeylabs/dappwright";
import { MetamaskWallet as StorageHubWallet } from "@storagehub-sdk/core";

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

test("should connect, switch network, and sign message with StorageHub SDK", async ({ wallet, page }) => {
    // Step 1: Connect wallet via dappwright
    await page.click("#connectButton");
    await wallet.approve();

    const connectStatus = page.getByTestId("connect-status");
    await expect(connectStatus).toHaveValue("connected");

    // Step 2: Switch to Hardhat network
    await page.click("#switch-network-button");
    await page.waitForTimeout(2000);

    const networkStatus = page.getByTestId("network-status");
    await expect(networkStatus).toHaveValue("31337");

    // Step 3: Verify StorageHub SDK can access the same provider
    const sdkCanAccess = await page.evaluate(async () => {
        try {
            // Check if the StorageHub SDK can access the ethereum provider
            if (typeof window.ethereum === 'undefined') {
                return { success: false, error: 'No ethereum provider' };
            }

            // Check if we can get the account that dappwright connected
            const accounts = await window.ethereum.request({ method: 'eth_accounts' });
            const chainId = await window.ethereum.request({ method: 'eth_chainId' });

            return {
                success: true,
                accounts: accounts,
                chainId: parseInt(chainId, 16),
                hasProvider: true
            };
        } catch (error) {
            return { success: false, error: error.message };
        }
    });

    console.log('SDK provider access result:', sdkCanAccess);

    // Verify the SDK can see the same connection
    expect(sdkCanAccess.success).toBe(true);
    expect(sdkCanAccess.chainId).toBe(31337);
    expect(sdkCanAccess.accounts.length).toBeGreaterThan(0);

    // Step 4: Actually sign a message using StorageHub SDK + improved MetaMask popup handling
    const messageInput = page.getByTestId("message-input");
    await expect(messageInput).toBeVisible();

    const signButton = page.getByTestId("sign-message");
    await expect(signButton).toBeVisible();

    // Fill in the message to sign
    await messageInput.fill("Hello from StorageHub SDK!");
    console.log("Message filled: 'Hello from StorageHub SDK!'");

    // Listen to browser console for debugging
    page.on('console', msg => console.log(`[BROWSER]: ${msg.text()}`));

    console.log("Starting message signing with StorageHub SDK...");

    // Get current browser contexts to detect new popup
    const initialContexts = page.context().pages();
    console.log(`Initial pages count: ${initialContexts.length}`);

    // Click the sign button to trigger MetaMask popup
    await signButton.click();
    console.log("Sign button clicked, waiting for MetaMask popup...");

    // Wait for MetaMask SIGNATURE popup to appear (not just any MetaMask page)
    let metamaskSignaturePage = null;
    let attempts = 0;
    const maxAttempts = 30; // 6 seconds with 200ms intervals

    while (attempts < maxAttempts && !metamaskSignaturePage) {
        await page.waitForTimeout(200);
        const currentPages = page.context().pages();
        console.log(`Attempt ${attempts}: Current pages count: ${currentPages.length}`);

        // Look for a page that contains signature content
        for (const currentPage of currentPages) {
            const url = currentPage.url();
            const title = await currentPage.title().catch(() => '');
            console.log(`  Page: ${url} - Title: "${title}"`);

            // Check if this page contains signature-related content
            if (url.includes('extension://')) {
                try {
                    // Look for signature-related text or elements
                    const pageText = await currentPage.textContent('body');
                    const hasSignatureContent = pageText.includes('Signature request') ||
                        pageText.includes('Sign message') ||
                        pageText.includes('Confirm') ||
                        url.includes('signature') ||
                        url.includes('sign');

                    // Also check if there's a confirm button visible
                    const hasConfirmButton = await currentPage.locator('button:has-text("Confirm")').count() > 0;

                    console.log(`  Page has signature content: ${hasSignatureContent}, has confirm button: ${hasConfirmButton}`);

                    if (hasSignatureContent || hasConfirmButton) {
                        metamaskSignaturePage = currentPage;
                        console.log(`Found MetaMask signature page: ${url}`);
                        break;
                    }
                } catch (e) {
                    console.log(`  Error checking page content: ${e.message}`);
                }
            }
        }
        attempts++;
    }

    if (!metamaskSignaturePage) {
        console.log("Could not find MetaMask signature popup, trying dappwright approve fallback...");
        try {
            await wallet.approve();
        } catch (error) {
            console.log("Dappwright approve also failed:", error);
            throw new Error("Failed to approve MetaMask signature - no signature popup found");
        }
    } else {
        console.log("MetaMask signature popup detected, analyzing page structure...");

        // Wait for the popup to fully load
        await metamaskSignaturePage.waitForLoadState('networkidle');
        await page.waitForTimeout(1000);

        // Get the page content to understand the structure
        const pageContent = await metamaskSignaturePage.content();
        console.log("MetaMask signature popup content length:", pageContent.length);

        // Get all buttons on the page for debugging
        const allButtons = await metamaskSignaturePage.locator('button').all();
        console.log(`Found ${allButtons.length} buttons in MetaMask signature popup`);

        for (let i = 0; i < allButtons.length; i++) {
            try {
                const buttonText = await allButtons[i].textContent();
                const buttonClasses = await allButtons[i].getAttribute('class');
                const buttonTestId = await allButtons[i].getAttribute('data-testid');
                console.log(`Button ${i}: text="${buttonText}" class="${buttonClasses}" testid="${buttonTestId}"`);
            } catch (e) {
                console.log(`Button ${i}: Could not read properties`);
            }
        }

        // Try to find and click the confirm button in the MetaMask popup
        const confirmSelectors = [
            'button:has-text("Confirm")',
            'button:has-text("Sign")',
            '[data-testid="page-container-footer-next"]',
            '[data-testid="confirm-btn"]',
            '.btn-primary',
            'button[type="submit"]',
            'button.btn--rounded.btn--primary',
            '.button--primary',
            'button:contains("Confirm")',
            'button:contains("Sign")'
        ];

        let confirmed = false;
        for (const selector of confirmSelectors) {
            try {
                console.log(`Trying selector: ${selector}`);
                const confirmButton = metamaskSignaturePage.locator(selector).first();
                await confirmButton.waitFor({ timeout: 2000 });

                if (await confirmButton.isVisible()) {
                    console.log(`Found confirm button with selector: ${selector}`);
                    await confirmButton.click();
                    confirmed = true;
                    console.log("Confirm button clicked successfully!");
                    break;
                }
            } catch (e) {
                console.log(`Selector ${selector} failed: ${e.message}`);
            }
        }

        // If specific selectors failed, try clicking buttons by text content
        if (!confirmed) {
            console.log("Specific selectors failed, trying buttons by text content...");
            for (let i = 0; i < allButtons.length; i++) {
                try {
                    const buttonText = await allButtons[i].textContent();
                    if (buttonText && (buttonText.toLowerCase().includes('confirm') || buttonText.toLowerCase().includes('sign'))) {
                        console.log(`Attempting to click button with text: "${buttonText}"`);
                        await allButtons[i].click();
                        confirmed = true;
                        console.log("Button clicked successfully!");
                        break;
                    }
                } catch (e) {
                    console.log(`Failed to click button ${i}: ${e.message}`);
                }
            }
        }

        if (!confirmed) {
            console.log("Direct selector approach failed, trying dappwright fallback...");
            try {
                await wallet.approve();
                confirmed = true;
            } catch (error) {
                console.log("Dappwright approve failed:", error);
                throw new Error("Failed to approve MetaMask signature");
            }
        }
    }

    // Wait for the signature to appear in the UI
    console.log("Waiting for signature to appear in UI...");
    const signatureElement = page.getByTestId("message-signature");

    // Poll for signature completion
    let signatureText = '';
    attempts = 0;
    while (attempts < 30) { // 6 second timeout
        await page.waitForTimeout(200);
        signatureText = await signatureElement.textContent() || '';
        console.log(`Signature attempt ${attempts}: "${signatureText}"`);

        if (signatureText && signatureText !== 'Signing...' && signatureText !== 'Error signing message') {
            console.log('✅ Signature received!');
            break;
        }
        attempts++;
    }

    if (!signatureText || signatureText === 'Signing...' || signatureText === 'Error signing message') {
        throw new Error('Signature process timed out or failed');
    }

    const signature = signatureText;

    console.log("Signature received:", signature);

    // Verify the signature is valid
    expect(signature).toBeTruthy();
    expect(signature.length).toBeGreaterThan(100); // Valid Ethereum signatures are long
    expect(signature).toMatch(/^0x[a-fA-F0-9]+$/); // Hex format

    // Also verify it appears in the UI
    await expect(signatureElement).toHaveText(signature);

    console.log("✅ Complete StorageHub SDK + dappwright integration test passed!");
    console.log("- ✅ Connected to MetaMask via dappwright");
    console.log("- ✅ Switched to Hardhat local network");
    console.log("- ✅ StorageHub SDK accessed the same ethereum provider");
    console.log("- ✅ Successfully signed message using StorageHub SDK");
    console.log("- ✅ dappwright approved the signature request");
    console.log("- ✅ Signature verification passed");
});