import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";

await describeMspNet(
  "Prometheus fisherman metrics",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    telemetry: true
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
      assert(
        createFishermanApi,
        "Fisherman API not available. Ensure `fisherman` is set to `true` in the network configuration."
      );
      // Initialize fisherman API (needed for network setup)
      await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process blocks
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for Prometheus to be ready
      await userApi.prometheus.waitForReady();
    });

    it("Fisherman batch deletions metric increments after batch processing", async () => {
      // Get initial metric value
      const initialBatchDeletions = await userApi.prometheus.getMetricValue(
        'storagehub_fisherman_batch_deletions_total{job="storagehub-fisherman"}'
      );

      console.log(`Initial fisherman batch deletions: ${initialBatchDeletions}`);

      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create files using batchStorageRequests helper
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/fisherman-metrics-1.txt",
            bucketIdOrName: "fisherman-metrics-bucket",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/fisherman-metrics-2.txt",
            bucketIdOrName: "fisherman-metrics-bucket",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api
      });

      const { fileKeys, bucketIds, locations, fingerprints, fileSizes } = batchResult;
      console.log(`Created ${fileKeys.length} files in bucket ${bucketIds[0]}`);

      // Wait for indexer to catch up
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
        await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
        await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
      }

      // Build deletion request calls with proper FileOperationIntention and signature
      const deletionCalls = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const fileOperationIntention = {
          fileKey: fileKeys[i],
          operation: { Delete: null }
        };

        const intentionCodec = userApi.createType(
          "PalletFileSystemFileOperationIntention",
          fileOperationIntention
        );
        const intentionPayload = intentionCodec.toU8a();
        const rawSignature = shUser.sign(intentionPayload);
        const userSignature = userApi.createType("MultiSignature", {
          Sr25519: rawSignature
        });

        deletionCalls.push(
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketIds[i],
            locations[i],
            fileSizes[i],
            fingerprints[i]
          )
        );
      }

      await userApi.block.seal({
        calls: deletionCalls,
        signer: shUser,
        finaliseBlock: true
      });

      // Wait for the deletion events
      const deletionEvents = await userApi.assert.eventMany("fileSystem", "FileDeletionRequested");
      console.log(`${deletionEvents.length} file deletion(s) requested`);

      // Wait for indexer to process the deletion requests
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process batch deletions
      // The fisherman batches deletions and submits them periodically (5 seconds in test config)
      await waitFor({
        lambda: async () => {
          try {
            // Check if fisherman has submitted any deletion extrinsics
            await userApi.assert.extrinsicPresent({
              module: "fileSystem",
              method: "deleteFiles",
              checkTxPool: true
            });
            return true;
          } catch {
            // Seal a block to allow fisherman to submit
            await userApi.block.seal();
            return false;
          }
        },
        iterations: 20,
        delay: 1000
      });

      // Seal the block with fisherman's batch deletion
      await userApi.block.seal();

      // Verify deletion completed events
      const bspDeletionCompletedEvents = await userApi.assert.eventMany(
        "fileSystem",
        "BspFileDeletionsCompleted"
      );
      console.log(`${bspDeletionCompletedEvents.length} BSP deletion batch(es) completed`);

      // Wait for Prometheus to scrape the updated metrics
      await userApi.prometheus.waitForScrape();

      // Check that fisherman batch deletions metric has incremented
      const finalBatchDeletions = await userApi.prometheus.getMetricValue(
        'storagehub_fisherman_batch_deletions_total{job="storagehub-fisherman"}'
      );

      console.log(`Final fisherman batch deletions: ${finalBatchDeletions}`);

      // Verify counter incremented
      assert(
        finalBatchDeletions > initialBatchDeletions,
        `Expected fisherman_batch_deletions_total to increment, got initial=${initialBatchDeletions}, final=${finalBatchDeletions}`
      );
    });

    it("All fisherman-related metrics are queryable", async () => {
      // Query all fisherman-related metrics
      const fishermanMetrics = [
        'storagehub_fisherman_batch_deletions_total{job="storagehub-fisherman"}'
      ];

      console.log("\nFisherman Metrics Summary:");
      for (const metric of fishermanMetrics) {
        const result = await userApi.prometheus.query(metric);
        assert.strictEqual(result.status, "success", `Query for ${metric} should succeed`);

        const value = await userApi.prometheus.getMetricValue(metric);
        console.log(`  ${metric}: ${value}`);
      }
    });
  }
);
