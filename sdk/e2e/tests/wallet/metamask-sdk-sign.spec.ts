import { testWithSynpress } from "@synthetixio/synpress";
import { MetaMask, metaMaskFixtures } from "@synthetixio/synpress/playwright";
import { expect } from "@playwright/test";
import basicSetup from "../../wallet-setup/basic.setup";

// Fingerprint taken from StorageHub node E2E tests
// See: test/util/bspNet/consts.ts → TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint
const EXPECTED_FINGERPRINT_HEX =
  "0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970";

const test = testWithSynpress(metaMaskFixtures(basicSetup));

test("MetaMask + SDK", async ({ context, page, metamaskPage, extensionId }) => {
  const metamask = new MetaMask(context, metamaskPage, basicSetup.walletPassword, extensionId);

  console.log("🎯 Starting test...");

  // Navigate to test page
  await page.goto("http://localhost:3000/e2e/page/index.html", { waitUntil: "domcontentloaded" });

  // Ensure provider is injected
  await page.waitForLoadState();
  await page.waitForFunction(() => (window as any).ethereum !== undefined, { timeout: 15000 });
  console.log("✅ Provider injected");

  // Click Connect on the basic dApp and approve in MetaMask
  await page.waitForSelector("#connect", { timeout: 60000 });
  await page.click("#connect");
  await metamask.connectToDapp();
  console.log("✅ Connection approved");

  // Trigger signing via the dApp's SDK handler by clicking the button
  await page.waitForSelector("#sign:not([disabled])", { timeout: 60000 });
  await page.click("#sign");

  // Approve signature in MetaMask
  await metamask.confirmSignature();

  // Wait until the dApp exposes the signature and log it
  const signature = await page.waitForFunction(() => (window as any).__lastSignature, {
    timeout: 15000
  });
  const value = await signature.jsonValue();
  console.log(`✅ Message signed: ${value}`);

  // --- Transaction signing via dApp button (may fail due to insufficient balance) ---
  await page.waitForSelector("#sign-tx:not([disabled])", { timeout: 15000 });
  await page.click("#sign-tx");

  // Reject the transaction in MetaMask (simplified flow)
  await metamask.rejectTransaction();
  console.log("ℹ️ Transaction rejected (expected without funds)");

  // --- File fingerprint computation ---
  await page.waitForSelector("#fingerprint-btn", { timeout: 60000 });
  await page.click("#fingerprint-btn");
  // Wait for the fingerprint result
  const fpHandle = await page.waitForFunction(() => (window as any).__lastFingerprint, {
    timeout: 15000
  });
  const fp = await fpHandle.jsonValue();
  console.log(`✅ Fingerprint computed: ${fp}`);
  expect(fp).toBe(EXPECTED_FINGERPRINT_HEX);
});
