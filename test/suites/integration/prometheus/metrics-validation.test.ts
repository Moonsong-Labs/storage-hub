import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser } from "../../../util";

await describeMspNet(
  "Prometheus metrics validation - all StorageHub metrics",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    prometheus: true
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createFishermanApi,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();
      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available");
      // Initialize fisherman API (needed for network setup)
      await createFishermanApi();

      // Connect to standalone indexer node
      assert(createIndexerApi, "Indexer API not available");
      indexerApi = await createIndexerApi();

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for Prometheus to be ready
      await userApi.prometheus.waitForReady();
    });

    it("Trigger storage activity to populate metrics", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create files to trigger storage metrics
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/validation-1.jpg",
            bucketIdOrName: "validation-test-bucket",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/validation-2.jpg",
            bucketIdOrName: "validation-test-bucket",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api
      });

      console.log(`Created ${batchResult.fileKeys.length} files for metrics validation`);

      // Wait for metrics to be scraped
      await userApi.prometheus.waitForScrape();
    });

    it("All StorageHub metrics are defined and queryable", async () => {
      console.log("\n=== StorageHub Metrics Validation Report ===\n");

      const metricsReport: {
        name: string;
        type: string;
        queryable: boolean;
        hasData: boolean;
        value?: string;
      }[] = [];

      for (const [_key, metric] of Object.entries(userApi.prometheus.metrics)) {
        let queryable = false;
        let hasData = false;
        let displayValue = "";

        try {
          // Query the base metric (without labels for counters/gauges, or _count for histograms)
          let queryMetric = metric.name;
          if (metric.type === "histogram") {
            queryMetric = `${metric.name}_count`;
          }

          const result = await userApi.prometheus.query(queryMetric);
          queryable = result.status === "success";

          if (result.data.result.length > 0) {
            hasData = true;
            // Sum all values if there are multiple series (different labels)
            let totalValue = 0;
            for (const r of result.data.result) {
              totalValue += Number.parseFloat(r.value?.[1] ?? "0");
            }
            displayValue = totalValue.toString();
          } else {
            displayValue = "0 (no data yet)";
          }
        } catch (e) {
          displayValue = `Error: ${e}`;
        }

        metricsReport.push({
          name: metric.name,
          type: metric.type,
          queryable,
          hasData,
          value: displayValue
        });
      }

      // Print report
      console.log("| Metric Name | Type | Queryable | Has Data | Value |");
      console.log("|-------------|------|-----------|----------|-------|");

      for (const m of metricsReport) {
        console.log(
          `| ${m.name} | ${m.type} | ${m.queryable ? "✓" : "✗"} | ${m.hasData ? "✓" : "✗"} | ${m.value} |`
        );
      }

      console.log("\n");

      // Summary
      const totalMetrics = metricsReport.length;
      const queryableCount = metricsReport.filter((m) => m.queryable).length;
      const withDataCount = metricsReport.filter((m) => m.hasData).length;

      console.log(`Total metrics: ${totalMetrics}`);
      console.log(`Queryable: ${queryableCount}/${totalMetrics}`);
      console.log(`With data: ${withDataCount}/${totalMetrics}`);

      // All metrics must be queryable
      assert.strictEqual(
        queryableCount,
        totalMetrics,
        `Expected all ${totalMetrics} metrics to be queryable, but only ${queryableCount} are`
      );
    });

    it("BSP metrics are exposed on BSP scrape target", async () => {
      const bspMetrics = [
        'storagehub_bsp_storage_requests_total{job="storagehub-bsp"}',
        'storagehub_bsp_proofs_submitted_total{job="storagehub-bsp"}',
        'storagehub_bsp_fees_charged_total{job="storagehub-bsp"}',
        'storagehub_bsp_files_deleted_total{job="storagehub-bsp"}',
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp"}',
        'storagehub_bsp_proof_generation_seconds_count{job="storagehub-bsp"}',
        'storagehub_storage_request_seconds_count{job="storagehub-bsp"}',
        'storagehub_file_transfer_seconds_count{job="storagehub-bsp"}'
      ];

      console.log("\nBSP Metrics from storagehub-bsp job:");
      for (const query of bspMetrics) {
        const result = await userApi.prometheus.query(query);
        assert.strictEqual(result.status, "success", `BSP metric query should succeed: ${query}`);
        const value = await userApi.prometheus.getMetricValue(query);
        console.log(`  ${query}: ${value}`);
      }
    });

    it("MSP metrics are exposed on MSP scrape targets", async () => {
      const mspMetrics = [
        'storagehub_msp_storage_requests_total{job="storagehub-msp-1"}',
        'storagehub_msp_files_distributed_total{job="storagehub-msp-1"}',
        'storagehub_msp_files_deleted_total{job="storagehub-msp-1"}',
        'storagehub_msp_buckets_deleted_total{job="storagehub-msp-1"}',
        'storagehub_msp_fees_charged_total{job="storagehub-msp-1"}',
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-1"}',
        'storagehub_storage_request_seconds_count{job="storagehub-msp-1"}',
        'storagehub_file_transfer_seconds_count{job="storagehub-msp-1"}'
      ];

      console.log("\nMSP Metrics from storagehub-msp-1 job:");
      for (const query of mspMetrics) {
        const result = await userApi.prometheus.query(query);
        assert.strictEqual(result.status, "success", `MSP metric query should succeed: ${query}`);
        const value = await userApi.prometheus.getMetricValue(query);
        console.log(`  ${query}: ${value}`);
      }
    });

    it("Fisherman metrics are exposed on fisherman scrape target", async () => {
      const fishermanMetrics = [
        'storagehub_fisherman_batch_deletions_total{job="storagehub-fisherman"}'
      ];

      console.log("\nFisherman Metrics from storagehub-fisherman job:");
      for (const query of fishermanMetrics) {
        const result = await userApi.prometheus.query(query);
        assert.strictEqual(
          result.status,
          "success",
          `Fisherman metric query should succeed: ${query}`
        );
        const value = await userApi.prometheus.getMetricValue(query);
        console.log(`  ${query}: ${value}`);
      }
    });

    it("Aggregated metrics across all nodes", async () => {
      console.log("\nAggregated Metrics Across All Nodes:");

      // Sum storage requests across all providers
      const totalStorageRequests = await userApi.prometheus.query(
        "sum(storagehub_bsp_storage_requests_total) + sum(storagehub_msp_storage_requests_total)"
      );
      console.log(
        `  Total storage requests (BSP + MSP): ${totalStorageRequests.data.result[0]?.value?.[1] ?? "N/A"}`
      );

      // Average storage request duration
      const avgStorageRequestDuration = await userApi.prometheus.query(
        "sum(storagehub_storage_request_seconds_sum) / sum(storagehub_storage_request_seconds_count)"
      );
      const avgDuration = Number.parseFloat(
        avgStorageRequestDuration.data.result[0]?.value?.[1] ?? "0"
      ).toFixed(3);
      console.log(`  Average storage request duration: ${avgDuration}s`);

      // Average file transfer duration
      const avgFileTransferDuration = await userApi.prometheus.query(
        "sum(storagehub_file_transfer_seconds_sum) / sum(storagehub_file_transfer_seconds_count)"
      );
      const avgTransfer = Number.parseFloat(
        avgFileTransferDuration.data.result[0]?.value?.[1] ?? "0"
      ).toFixed(3);
      console.log(`  Average file transfer duration: ${avgTransfer}s`);
    });
  }
);
