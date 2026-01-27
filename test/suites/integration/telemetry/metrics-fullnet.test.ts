import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeMspNet(
  "Prometheus metrics ingestion",
  {
    initialised: false,
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

      assert(healthyTargets.length >= 4, "Expected at least 4 healthy scrape targets");
    });

    it("Substrate metrics are available from all nodes", async () => {
      // Verify block height metrics are being collected from all nodes
      const jobs = ["storagehub-bsp", "storagehub-msp-1", "storagehub-msp-2", "storagehub-user"];

      for (const job of jobs) {
        const blockHeight = await userApi.prometheus.getMetricValue(
          `substrate_block_height{job="${job}"}`
        );
        assert(blockHeight >= 0, `Expected block height from ${job} to be >= 0`);
      }
    });

    it("Storage request metrics increment after batch file uploads", async () => {
      // Get initial metric values (using centralized event handler metrics)
      // The event label is derived from the event type name: NewStorageRequest -> new_storage_request
      // - event_handler_pending: gauge tracking currently in-flight handlers
      // - event_handler_total with status="success": counter for completed handlers
      const initialMspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_total{event="new_storage_request",status="success",job="storagehub-msp-1"}'
      );
      const initialBspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_total{event="new_storage_request",status="success",job="storagehub-bsp"}'
      );

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
        bspApis: [bspApi],
        mspApi
      });

      const { fileKeys } = batchResult;

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

      // Check that success counter has incremented
      const finalMspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_total{event="new_storage_request",status="success",job="storagehub-msp-1"}'
      );
      const finalBspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_total{event="new_storage_request",status="success",job="storagehub-bsp"}'
      );

      // Check pending gauge (should be 0 or low since handlers completed)
      const mspPendingGauge = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_pending{event="new_storage_request",job="storagehub-msp-1"}'
      );
      const bspPendingGauge = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_pending{event="new_storage_request",job="storagehub-bsp"}'
      );

      // Success counters should have increased after processing 4 storage requests
      assert(
        finalMspSuccess > initialMspSuccess,
        `Expected MSP success to increment, got ${finalMspSuccess}`
      );
      assert(
        finalBspSuccess > initialBspSuccess,
        `Expected BSP success to increment, got ${finalBspSuccess}`
      );

      // Pending gauge should be low (0 or close to 0) since all handlers completed
      // We don't assert exactly 0 because there could be new events arriving
      assert(
        mspPendingGauge <= 5,
        `MSP pending gauge (${mspPendingGauge}) should be low after handlers complete`
      );
      assert(
        bspPendingGauge <= 5,
        `BSP pending gauge (${bspPendingGauge}) should be low after handlers complete`
      );
    });

    it("Event handler metrics are tracked", async () => {
      // Event handler duration histogram - check metrics for events that occurred during the test
      // These events should have been triggered by the batch file uploads:
      // - new_storage_request: Initial storage request handling (BSP and MSP)
      // - remote_upload_request: File chunk uploads from user to MSP/BSP
      // - process_confirm_storing_request: BSP confirm storage processing
      // - process_msp_respond_storing_request: MSP respond to storage request

      const eventsToCheck = [
        "new_storage_request",
        "remote_upload_request",
        "process_confirm_storing_request",
        "process_msp_respond_storing_request"
      ];

      let totalEventCount = 0;

      for (const event of eventsToCheck) {
        const count = await userApi.prometheus.getMetricValue(
          `storagehub_event_handler_seconds_count{event="${event}"}`
        );
        if (count > 0) {
          totalEventCount += count;
        }
      }

      assert(
        totalEventCount > 0,
        "Expected event_handler_seconds to have recorded observations for at least one event type"
      );

      // Verify pending gauge exists (should be low since handlers completed)
      const pendingGauge = await userApi.prometheus.getMetricValue(
        'storagehub_event_handler_pending{event="new_storage_request"}'
      );
      assert(
        pendingGauge >= 0,
        `Event handler pending gauge should exist and be >= 0, got ${pendingGauge}`
      );
    });

    it("Command processing metrics are tracked", async () => {
      // Command metrics are recorded by BlockchainService and FishermanService
      // After storage operations, we should see command processing metrics

      // Check command processing histogram has observations
      // Commands like send_extrinsic, query_earliest_file_volunteer_tick, etc.
      const commandHistogramCount = await userApi.prometheus.getMetricValue(
        "storagehub_command_processing_seconds_count"
      );

      assert(
        commandHistogramCount > 0,
        `Expected command_processing_seconds to have observations, got ${commandHistogramCount}`
      );
    });

    it("Block processing metrics are tracked", async () => {
      // Block processing metrics track block_import and finalized_block operations
      // These should be populated as the chain produces and finalizes blocks

      // Check block_import histogram has observations
      const blockImportCount = await userApi.prometheus.getMetricValue(
        'storagehub_block_processing_seconds_count{operation="block_import"}'
      );

      // Check finalized_block histogram has observations
      const finalizedBlockCount = await userApi.prometheus.getMetricValue(
        'storagehub_block_processing_seconds_count{operation="finalized_block"}'
      );

      // At least one of the block processing operations should have been recorded
      const totalBlockProcessing = blockImportCount + finalizedBlockCount;
      assert(
        totalBlockProcessing > 0,
        `Expected block_processing_seconds to have observations (block_import: ${blockImportCount}, finalized_block: ${finalizedBlockCount})`
      );

      // Verify we can query block processing sum (for average calculation)
      const blockProcessingSum = await userApi.prometheus.getMetricValue(
        "storagehub_block_processing_seconds_sum"
      );
      assert(
        blockProcessingSum >= 0,
        `Block processing sum should be >= 0, got ${blockProcessingSum}`
      );
    });

    it("All StorageHub metric types are queryable", async () => {
      // Query all storagehub metrics and log them for visibility
      const result = await userApi.prometheus.query('{__name__=~"storagehub_.*"}');

      assert.strictEqual(result.status, "success", "Failed to query StorageHub metrics");
    });

    it("Prometheus can aggregate metrics across nodes", async () => {
      // Test aggregation query summing event handler metrics across all nodes
      // Filter by event type and aggregate by status
      const sumResult = await userApi.prometheus.query(
        'sum(storagehub_event_handler_total{event="new_storage_request",job=~"storagehub-msp.*"}) by (status)'
      );
      assert.strictEqual(sumResult.status, "success", "Aggregation query should succeed");
    });
  }
);
