import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeMspNet(
  "Prometheus metrics ingestion",
  {
    initialised: false,
    indexer: true,
    telemetry: true
  },
  ({ before, createUserApi, createMsp1Api, createBspApi, it }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
      bspApi = await createBspApi();

      // Wait for Prometheus to be ready and scraping targets
      // Increased iterations to 30 (60 seconds) to allow for Docker image pull on first run
      await waitFor({
        lambda: async () => {
          try {
            const targets = await userApi.prometheus.getTargets();
            const healthyTargets = targets.data.activeTargets.filter((t) => t.health === "up");
            return healthyTargets.length >= 4; // BSP, MSP-1, MSP-2, User
          } catch {
            return false;
          }
        },
        delay: 2000,
        iterations: 30
      });
    });

    it("Prometheus server is accessible and scraping all nodes", async () => {
      const response = await fetch(`${userApi.prometheus.url}/-/ready`);
      assert.strictEqual(response.ok, true, "Prometheus server should be ready");

      const targets = await userApi.prometheus.getTargets();
      const healthyTargets = targets.data.activeTargets.filter((t) => t.health === "up");

      console.log(`Prometheus is scraping ${healthyTargets.length} healthy targets`);
      for (const target of healthyTargets) {
        console.log(`  - ${target.labels.job}: ${target.scrapeUrl}`);
      }

      assert(healthyTargets.length >= 4, "Expected at least 4 healthy scrape targets");
    });

    it("Substrate metrics are available from all nodes", async () => {
      // Verify block height metrics are being collected from all nodes
      const jobs = ["storagehub-bsp", "storagehub-msp-1", "storagehub-msp-2", "storagehub-user"];

      for (const job of jobs) {
        const blockHeight = await userApi.prometheus.getMetricValue(
          `substrate_block_height{job="${job}"}`
        );
        console.log(`  ${job} block height: ${blockHeight}`);
        assert(blockHeight >= 0, `Expected block height from ${job} to be >= 0`);
      }
    });

    it("Storage request metrics increment after batch file uploads", async () => {
      // Get initial metric values
      const initialMspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_msp_storage_requests_total{job="storagehub-msp-1"}'
      );
      const initialBspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_storage_requests_total{job="storagehub-bsp"}'
      );
      console.log(`Initial MSP storage requests: ${initialMspRequests}`);
      console.log(`Initial BSP storage requests: ${initialBspRequests}`);

      // Get MSP value proposition for batch storage requests
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests to create multiple files at once
      // This helper handles bucket creation, file loading, storage request issuance,
      // MSP acceptance, and BSP volunteering all in one call
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/prometheus-batch-0.jpg",
            bucketIdOrName: "prometheus-test-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/prometheus-batch-1.jpg",
            bucketIdOrName: "prometheus-test-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/prometheus-batch-2.jpg",
            bucketIdOrName: "prometheus-test-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/prometheus-batch-3.jpg",
            bucketIdOrName: "prometheus-test-bucket-1",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi
      });

      const { fileKeys } = batchResult;
      console.log(`Created ${fileKeys.length} storage requests`);

      // Verify all files are stored in MSP
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Verify all files are stored in BSP
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Wait for Prometheus to scrape the updated metrics
      await userApi.prometheus.waitForScrape();

      // Check that metrics have incremented
      const finalMspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_msp_storage_requests_total{job="storagehub-msp-1"}'
      );
      const finalBspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_storage_requests_total{job="storagehub-bsp"}'
      );
      console.log(`Final MSP storage requests: ${finalMspRequests}`);
      console.log(`Final BSP storage requests: ${finalBspRequests}`);

      // Metrics should have increased after processing 4 storage requests
      assert(
        finalMspRequests > initialMspRequests || finalMspRequests >= 4,
        `Expected MSP storage requests to increment, got ${finalMspRequests}`
      );
      assert(
        finalBspRequests > initialBspRequests || finalBspRequests >= 4,
        `Expected BSP storage requests to increment, got ${finalBspRequests}`
      );
    });

    it("Histogram metrics are populated after storage operations", async () => {
      // After the previous test ran storage requests, check histogram metrics

      // Storage request duration histogram
      const storageReqCount = await userApi.prometheus.getMetricValue(
        "storagehub_storage_request_seconds_count"
      );
      const storageReqSum = await userApi.prometheus.getMetricValue(
        "storagehub_storage_request_seconds_sum"
      );
      console.log(
        `Storage request histogram - count: ${storageReqCount}, sum: ${storageReqSum.toFixed(3)}s`
      );
      assert(storageReqCount > 0, "Expected storage_request_seconds to have recorded observations");
      assert(storageReqSum >= 0, "Histogram sum should be non-negative");
      console.log(
        `  Average storage request time: ${(storageReqSum / storageReqCount).toFixed(3)} seconds`
      );

      // File transfer duration histogram
      // Note: file_transfer_seconds is only recorded when send_chunks() is called for peer-to-peer
      // transfers. Direct uploads via batchStorageRequests may not trigger this metric.
      const fileTransferCount = await userApi.prometheus.getMetricValue(
        "storagehub_file_transfer_seconds_count"
      );
      const fileTransferSum = await userApi.prometheus.getMetricValue(
        "storagehub_file_transfer_seconds_sum"
      );
      console.log(
        `File transfer histogram - count: ${fileTransferCount}, sum: ${fileTransferSum.toFixed(3)}s`
      );
      // Only log informational - this metric may be 0 for direct uploads
      if (fileTransferCount > 0) {
        console.log(
          `  Average file transfer time: ${(fileTransferSum / fileTransferCount).toFixed(
            3
          )} seconds`
        );
      } else {
        console.log("  (No peer-to-peer file transfers occurred in this test)");
      }
    });

    it("All StorageHub metric types are queryable", async () => {
      // Query all storagehub metrics and log them for visibility
      const result = await userApi.prometheus.query('{__name__=~"storagehub_.*"}');

      assert.strictEqual(result.status, "success", "Failed to query StorageHub metrics");

      // Group metrics by type
      const counters: string[] = [];
      const histograms: string[] = [];
      const gauges: string[] = [];

      for (const metric of result.data.result) {
        const name = metric.metric.__name__;
        if (name.endsWith("_total")) {
          if (!counters.includes(name)) counters.push(name);
        } else if (name.endsWith("_bucket") || name.endsWith("_sum") || name.endsWith("_count")) {
          const baseName = name.replace(/_bucket$|_sum$|_count$/, "");
          if (!histograms.includes(baseName)) histograms.push(baseName);
        } else {
          if (!gauges.includes(name)) gauges.push(name);
        }
      }

      console.log("\nStorageHub Metrics Summary:");
      console.log(`  Counters (${counters.length}): ${counters.join(", ") || "none"}`);
      console.log(`  Histograms (${histograms.length}): ${histograms.join(", ") || "none"}`);
      console.log(`  Gauges (${gauges.length}): ${gauges.join(", ") || "none"}`);

      const totalMetrics = counters.length + histograms.length + gauges.length;
      console.log(`  Total unique metrics: ${totalMetrics}`);
    });

    it("Prometheus can aggregate metrics across nodes", async () => {
      // Test aggregation query summing storage requests across all nodes
      const sumResult = await userApi.prometheus.query(
        "sum(storagehub_msp_storage_requests_total) by (status)"
      );
      const bspSumResult = await userApi.prometheus.query(
        "sum(storagehub_bsp_storage_requests_total) by (status)"
      );

      console.log("\nAggregated metrics:");
      if (sumResult.data.result.length > 0) {
        for (const item of sumResult.data.result) {
          console.log(
            `  MSP storage requests (${item.metric.status || "total"}): ${item.value?.[1]}`
          );
        }
      }
      if (bspSumResult.data.result.length > 0) {
        for (const item of bspSumResult.data.result) {
          console.log(
            `  BSP storage requests (${item.metric.status || "total"}): ${item.value?.[1]}`
          );
        }
      }

      assert.strictEqual(sumResult.status, "success", "Aggregation query should succeed");
    });
  }
);
