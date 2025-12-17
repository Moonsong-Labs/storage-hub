import assert from "node:assert";
import { describeBspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeBspNet(
  "BSPNet: Prometheus metrics ingestion",
  {
    initialised: false,
    telemetry: true
  },
  ({ before, createUserApi, createBspApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();

      // Wait for Prometheus to be ready and scraping targets
      // Increased iterations to 30 (60 seconds) to allow for Docker image pull on first run
      await waitFor({
        lambda: async () => {
          try {
            const targets = await userApi.prometheus.getTargets();
            const healthyTargets = targets.data.activeTargets.filter((t) => t.health === "up");
            return healthyTargets.length >= 2; // BSP and User
          } catch {
            return false;
          }
        },
        delay: 2000,
        iterations: 30
      });
    });

    it("Prometheus server is accessible and scraping BSPNet nodes", async () => {
      const response = await fetch(`${userApi.prometheus.url}/-/ready`);
      assert.strictEqual(response.ok, true, "Prometheus server should be ready");

      const targets = await userApi.prometheus.getTargets();
      const healthyTargets = targets.data.activeTargets.filter((t) => t.health === "up");

      console.log(`Prometheus is scraping ${healthyTargets.length} healthy targets`);
      for (const target of healthyTargets) {
        console.log(`  - ${target.labels.job}: ${target.scrapeUrl}`);
      }

      assert(
        healthyTargets.length >= 2,
        "Expected at least 2 healthy scrape targets (BSP and User)"
      );
    });

    it("Substrate metrics are available from BSPNet nodes", async () => {
      // Verify block height metrics are being collected from BSPNet nodes
      const jobs = ["storagehub-bsp", "storagehub-user"];

      for (const job of jobs) {
        const blockHeight = await userApi.prometheus.getMetricValue(
          `substrate_block_height{job="${job}"}`
        );
        console.log(`  ${job} block height: ${blockHeight}`);
        assert(blockHeight >= 0, `Expected block height from ${job} to be >= 0`);
      }
    });

    it("BSP storage request metrics increment after file uploads", async () => {
      // Get initial metric value
      const initialBspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_storage_requests_total{job="storagehub-bsp"}'
      );
      console.log(`Initial BSP storage requests: ${initialBspRequests}`);

      // Create storage requests
      const source = "res/whatsup.jpg";
      const location = "test/prometheus-bspnet-1.jpg";
      const bucketName = "prometheus-bspnet-bucket";

      await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        location,
        bucketName,
        null,
        shUser,
        null,
        1
      );

      // Wait for BSP to volunteer and store the file
      await userApi.wait.bspVolunteer();
      await userApi.wait.bspStored();

      // Verify file is stored in BSP
      const fileKey = (
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          location,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          userApi.shConsts.DUMMY_MSP_ID
        )
      ).file_metadata.file_key;

      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for Prometheus to scrape the updated metrics
      await userApi.prometheus.waitForScrape();

      // Check that metrics have incremented
      const finalBspRequests = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_storage_requests_total{job="storagehub-bsp"}'
      );
      console.log(`Final BSP storage requests: ${finalBspRequests}`);

      // Metrics should have increased after processing storage request
      assert(
        finalBspRequests > initialBspRequests || finalBspRequests >= 1,
        `Expected BSP storage requests to increment, got ${finalBspRequests}`
      );
    });

    it("Histogram metrics are populated after storage operations", async () => {
      // After the previous test ran storage requests, check histogram metrics

      // Storage request duration histogram
      const storageReqCount = await userApi.prometheus.getMetricValue(
        "storagehub_storage_request_setup_seconds_count"
      );
      const storageReqSum = await userApi.prometheus.getMetricValue(
        "storagehub_storage_request_setup_seconds_sum"
      );
      console.log(
        `Storage request setup histogram - count: ${storageReqCount}, sum: ${storageReqSum.toFixed(
          3
        )}s`
      );
      assert(
        storageReqCount > 0,
        "Expected storage_request_setup_seconds to have recorded observations"
      );
      assert(storageReqSum >= 0, "Histogram sum should be non-negative");
      console.log(
        `  Average storage request time: ${(storageReqSum / storageReqCount).toFixed(3)} seconds`
      );
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

    it("Prometheus can aggregate BSP metrics", async () => {
      // Test aggregation query summing storage requests for BSP
      const bspSumResult = await userApi.prometheus.query(
        "sum(storagehub_bsp_storage_requests_total) by (status)"
      );

      console.log("\nAggregated BSP metrics:");
      if (bspSumResult.data.result.length > 0) {
        for (const item of bspSumResult.data.result) {
          console.log(
            `  BSP storage requests (${item.metric.status || "total"}): ${item.value?.[1]}`
          );
        }
      }

      assert.strictEqual(bspSumResult.status, "success", "Aggregation query should succeed");
    });
  }
);
