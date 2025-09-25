import { expect, test } from "@playwright/test";

test.describe("MSP Web Page Flow", () => {
  test.use({ acceptDownloads: true });

  test("clicks through all buttons and validates via console + downloads", async ({ page }) => {
    const baseUrl = "http://localhost:3000/e2e/page/msp.html";

    const seen = new Set<string>();
    page.on("console", (msg) => {
      const text = msg.text();
      // Echo page console to terminal for debugging
      console.log("[PAGE]", text);
      if (text.startsWith("[HEALTH]")) seen.add("health");
      if (text.startsWith("[NONCE]")) seen.add("nonce");
      if (text.startsWith("[SIGN]")) seen.add("sign");
      if (text.startsWith("[VERIFY]")) seen.add("verify");
      if (text.startsWith("[UPLOAD][RECEIPT]")) seen.add("upload");
      if (text.startsWith("[DOWNLOAD][KEY][META]")) seen.add("dl-key");
      if (text.startsWith("[DOWNLOAD][PATH][META]")) seen.add("dl-path");
      if (text.startsWith("[BUCKETS][LIST]")) seen.add("buckets-list");
      if (text.startsWith("[BUCKETS][GET]")) seen.add("bucket-get");
      if (text.startsWith("[BUCKETS][FILES]")) seen.add("bucket-files");
      if (text.startsWith("[MSP][INFO]")) seen.add("msp-info");
      if (text.startsWith("[MSP][STATS]")) seen.add("msp-stats");
      if (text.startsWith("[MSP][VALUE-PROPS]")) seen.add("msp-value-props");
      if (text.startsWith("[MSP][FILE-INFO]")) seen.add("msp-file-info");
    });

    const waitForConsoleTag = async (tag: string, action: () => Promise<void>) => {
      // Clear previous occurrence to avoid tests passing due to stale state
      seen.delete(tag);
      await action();
      await expect.poll(() => seen.has(tag)).toBeTruthy();
    };

    console.log("[TEST] goto", baseUrl);
    await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
    console.log("✅ goto");

    // Connect MSP
    console.log("[TEST] click Connect MSP");
    await page.getByRole("button", { name: "Connect MSP" }).click();
    await page.getByText("MSP connected", { exact: false }).waitFor({ timeout: 30000 });
    console.log("✅ Connect MSP");

    // Get Health
    console.log("[TEST] click Get Health");
    await page.getByRole("button", { name: "Get Health" }).click();
    await expect.poll(() => seen.has("health")).toBeTruthy();
    console.log("✅ Get Health");

    // Get Nonce
    console.log("[TEST] click Get Nonce");
    await page.getByRole("button", { name: "Get Nonce" }).click();
    await expect.poll(() => seen.has("nonce")).toBeTruthy();
    console.log("✅ Get Nonce");

    // Sign Message
    console.log("[TEST] click Sign Message");
    await page.getByRole("button", { name: "Sign Message" }).click();
    await expect.poll(() => seen.has("sign")).toBeTruthy();
    console.log("✅ Sign Message");

    // Verify (set token)
    console.log("[TEST] click Verify (set token)");
    await page.getByRole("button", { name: "Verify (set token)" }).click();
    await expect.poll(() => seen.has("verify")).toBeTruthy();
    console.log("✅ Verify");

    // Upload adolphus.jpg
    console.log("[TEST] click Upload adolphus.jpg");
    await page.getByRole("button", { name: "Upload adolphus.jpg" }).click();
    await expect.poll(() => seen.has("upload")).toBeTruthy();
    console.log("✅ Upload");

    // Download by Key (validate download)
    console.log("[TEST] click Download by Key");
    const keyDownload = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download by Key" }).click();
    const keyFile = await keyDownload;
    const keyPath = await keyFile.path();
    console.log("[TEST] key download path:", keyPath);
    expect(keyPath).toBeTruthy();
    console.log("✅ Download by Key");

    // Ensure console tags for downloads captured
    await expect.poll(() => seen.has("dl-key")).toBeTruthy();
    console.log("✅ Console tags for downloads");

    // List buckets
    console.log("[TEST] click List Buckets");
    await page.getByRole("button", { name: "List Buckets" }).click();
    await expect.poll(() => seen.has("buckets-list")).toBeTruthy();
    const bucketsCountText = await page.locator("#bucketsCount").textContent();
    expect(Number(bucketsCountText)).toBeGreaterThan(0);
    console.log("✅ List Buckets");

    // Get bucket (since we are not setting a bucket ID it will return the first one from the previous step)
    console.log("[TEST] click Get Bucket");
    await page.getByRole("button", { name: "Get Bucket" }).click();
    await expect.poll(() => seen.has("bucket-get")).toBeTruthy();
    const lastBucketJson = await page.locator("#bucketJson").textContent();
    expect(lastBucketJson && lastBucketJson.length > 0).toBeTruthy();
    console.log("✅ Get Bucket");

    // Get files of a bucket at root
    console.log("[TEST] click Get Files (root)");
    await waitForConsoleTag("bucket-files", async () => {
      await page.getByRole("button", { name: "Get Files" }).click();
    });
    let filesJson = await page.locator("#filesJson").textContent();
    expect(filesJson && filesJson.includes("Thesis")).toBeTruthy();
    console.log("✅ Get Files (root)");

    // Get files of a bucket at path `/Thesis/`
    console.log("[TEST] click Get Files (Thesis)");
    await page.locator("#pathInput").fill("/Thesis/");
    await waitForConsoleTag("bucket-files", async () => {
      await page.getByRole("button", { name: "Get Files" }).click();
    });
    filesJson = await page.locator("#filesJson").textContent();
    expect(filesJson && filesJson.includes("chapter1.pdf")).toBeTruthy();
    console.log("✅ Get Files (Thesis)");

    // Get files of a bucket at path `/Reports/`
    console.log("[TEST] click Get Files (Reports)");
    await page.locator("#pathInput").fill("/Reports/");
    await waitForConsoleTag("bucket-files", async () => {
      await page.getByRole("button", { name: "Get Files" }).click();
    });
    filesJson = await page.locator("#filesJson").textContent();
    expect(filesJson && filesJson.includes("Q1-2024.pdf")).toBeTruthy();
    console.log("✅ Get Files (Reports)");

    // MSP Info: Get Info
    console.log("[TEST] click Get Info");
    await waitForConsoleTag("msp-info", async () => {
      await page.getByRole("button", { name: "Get Info" }).click();
    });
    const infoJson = await page.locator("#infoJson").textContent();
    expect(infoJson && infoJson.includes("mspId")).toBeTruthy();
    expect(infoJson && infoJson.includes("client")).toBeTruthy();
    console.log("✅ Get Info");

    // MSP Info: Get Stats
    console.log("[TEST] click Get Stats");
    await waitForConsoleTag("msp-stats", async () => {
      await page.getByRole("button", { name: "Get Stats" }).click();
    });
    const statsJson = await page.locator("#statsJson").textContent();
    expect(statsJson && statsJson.includes("capacity")).toBeTruthy();
    expect(statsJson && statsJson.includes("totalBytes")).toBeTruthy();
    console.log("✅ Get Stats");

    // MSP Info: Get Value Props
    console.log("[TEST] click Get Value Props");
    await waitForConsoleTag("msp-value-props", async () => {
      await page.getByRole("button", { name: "Get Value Props" }).click();
    });
    const valuePropsJson = await page.locator("#valuePropsJson").textContent();
    expect(valuePropsJson && valuePropsJson.includes("pricePerGbPerBlock")).toBeTruthy();
    console.log("✅ Get Value Props");

    // MSP File Info (uses defaults in page)
    console.log("[TEST] click Get File Info");
    await waitForConsoleTag("msp-file-info", async () => {
      await page.getByRole("button", { name: "Get File Info" }).click();
    });
    const fileInfoJson = await page.locator("#fileInfoJson").textContent();
    expect(fileInfoJson && fileInfoJson.includes("fileKey")).toBeTruthy();
    expect(fileInfoJson && fileInfoJson.includes("uploadedAt")).toBeTruthy();
    console.log("✅ Get File Info");
  });
});
