import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173/file-manager.html");
});

test("should compute merkle root hash for adolphus.jpg", async ({ page }) => {
    const isHeadless = process.env.HEADLESS === 'true';

    console.log(`🧪 FileManager test - ${isHeadless ? 'HEADLESS' : 'HEADED'} mode`);

    // Wait for WASM to be ready
    await page.waitForFunction(() => (window as any).wasmReady === true, { timeout: 10000 });

    // Compute hash
    const result = await page.evaluate(async () => {
        const response = await fetch('/adolphus.jpg');
        const blob = await response.blob();

        const fileObject = {
            size: blob.size,
            stream: () => blob.stream()
        };

        const { FileManager } = window as any;
        const fileManager = new FileManager(fileObject);
        const fingerprint = await fileManager.getFingerprint();

        return {
            hash: fingerprint.toHex(),
            size: blob.size
        };
    });

    // Update UI
    await page.evaluate((data) => {
        document.getElementById('hash-result').value = data.hash;
        document.getElementById('file-size').value = `${data.size} bytes`;
        document.getElementById('status').textContent = 'Completed ✅';
    }, result);

    // Verify
    expect(result.hash).toMatch(/^0x[a-fA-F0-9]{64}$/);
    expect(result.size).toBe(416400);

    console.log(`✅ Hash: ${result.hash}`);

    // In headed mode, wait a bit so user can see the result
    if (!isHeadless) {
        await page.waitForTimeout(2000);
    }
});