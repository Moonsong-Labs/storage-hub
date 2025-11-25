/**
 * Big File Download Benchmark Test
 *
 * Measures MSP download performance with large files.
 * Uses `initialised_big` to generate/upload a test file automatically.
 */

import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, waitFor } from "../../../util";
import { getFileSize, deleteFileIfExists } from "../../../util/fileGeneration";

await describeMspNet(
  "MSP Big File Download Benchmark",
  {
    initialised_big: 0.02, // 1GB file (use smaller for faster tests, increase for real benchmarks)
    networkConfig: [{ noisy: false, rocksdb: true }],
    only: true,
  },
  ({ before, after, it, createMsp1Api, getLaunchResponse }) => {
    let mspApi: EnrichedBspApi;
    let fileKey: string;
    let tempFilePath: string;
    let originalFileSize: number;

    before(async () => {
      const api = await createMsp1Api();
      assert(api, "MSP1 API should be available");
      mspApi = api;

      const launchData = await getLaunchResponse() as {
        fileMetadata: { fileKey: string };
        tempFilePath: string;
      } | undefined;

      assert(launchData, "Launch data should be available for initialised_big");

      fileKey = launchData.fileMetadata.fileKey;
      tempFilePath = launchData.tempFilePath;
      originalFileSize = await getFileSize(tempFilePath);

      console.log(`📊 File: ${(originalFileSize / 1024 / 1024 / 1024).toFixed(1)}GB | Key: ${fileKey.slice(0, 16)}...`);

      // Wait for the MSP to actually have the file in storage (can take time for large files)
      console.log(`⏳ Waiting for MSP to store the file...`);
      await waitFor({
        lambda: async () => {
          const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
          return result.isFileFound;
        },
        delay: 5000,
        iterations: 120 // Wait up to 10 minutes for large files
      });
      console.log(`✅ File is in MSP storage`);
    });

    after(async () => {
      await deleteFileIfExists(tempFilePath);
      console.log(`🧹 Cleaned up: ${tempFilePath}`);
    });

    it("should download file and measure throughput", async () => {
      // Download to container path
      const downloadPath = `/storage/test/benchmark-${Date.now()}.bin`;

      console.log(`📥 Starting download...`);
      const startTime = Date.now();

      const result = await mspApi.rpc.storagehubclient.saveFileToDisk(fileKey, downloadPath);

      const downloadTime = Date.now() - startTime;
      assert(result.isSuccess, `Download failed: ${JSON.stringify(result)}`);

      // Report metrics
      const sizeMB = originalFileSize / (1024 * 1024);
      const throughput = sizeMB / (downloadTime / 1000);

      console.log(`\n📊 Results: ${(downloadTime / 1000).toFixed(1)}s | ${throughput.toFixed(1)} MB/s\n`);
    });
  }
);
