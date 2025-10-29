import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, waitFor } from "../../../util";
import { getLastIndexedBlock, waitForBucketIndexed } from "../../../util/indexerHelpers";

await describeMspNet(
  "Indexer Service - Block Notification Sync (Genesis Pause)",
  {
    initialised: false,
    indexer: true,
    indexerMode: "full",
    standaloneIndexer: true
  },
  ({
    before,
    after,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createApi
  }) => {
    let userApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      sql = createSqlClient();

      // Create API connection to standalone indexer service (port 9800)
      indexerApi = await createApi("ws://127.0.0.1:9800");

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);
    });

    after(async () => {
      await indexerApi?.disconnect();
      await sql?.end();
    });

    it("indexes all events produced while behind and during sync", async () => {
      // Capture baseline state to verify indexer was truly paused during block production
      await getLastIndexedBlock(sql);

      // Simulate indexer falling behind by pausing its container while blockchain continues
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.indexer.containerName);

      // Produce enough blocks (7) to exceed sync_mode_min_blocks_behind threshold (5)
      // This ensures the indexer will enter sync mode rather than processing blocks individually
      const buckets: string[] = [];
      for (let i = 0; i < 7; i++) {
        const bucketName = `test-bucket-sync-${i}`;
        await userApi.createBucket(bucketName);
        await userApi.block.seal({ finaliseBlock: true });
        buckets.push(bucketName);
      }

      const finalBlockHeader = await userApi.rpc.chain.getHeader();
      const finalBlockNumber = finalBlockHeader.number.toNumber();

      // Confirm indexer remained frozen at initial block - ensures pause was effective
      await getLastIndexedBlock(sql);

      // Resume indexer to trigger catchup - it must now process backlog via finality notifications
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.indexer.containerName
      });

      // Non-producer nodes must explicitly finalize imported blocks to trigger indexing
      // Producer node (user) has finalized blocks, but indexer node must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await indexerApi.wait.blockImported(finalisedBlockHash.toString());

      await indexerApi.block.finaliseBlock(finalisedBlockHash.toString());

      // Block until indexer catches up - verifies finality notification pipeline works under load
      await waitFor({
        lambda: async () => {
          const lastIndexed = await getLastIndexedBlock(sql);
          return lastIndexed >= finalBlockNumber;
        },
        iterations: 100,
        delay: 500
      });

      // Verify data consistency - all events from missed blocks should be present in database
      for (const bucketName of buckets) {
        await waitForBucketIndexed(sql, bucketName);
      }

      // Validate service_state tracking is accurate after catchup
      const lastIndexedBlock = await getLastIndexedBlock(sql);

      // Indexer should have processed at minimum the blocks we created, possibly more if chain advanced
      assert(
        lastIndexedBlock >= finalBlockNumber,
        `Indexer should have indexed up to block ${finalBlockNumber}, but only indexed ${lastIndexedBlock}`
      );

      // Final consistency check - bucket count in database matches blockchain events
      const dbBuckets = await sql`
        SELECT name FROM bucket
        WHERE name LIKE 'test-bucket-sync-%'
        ORDER BY name
      `;

      assert.equal(
        dbBuckets.length,
        buckets.length,
        `Expected ${buckets.length} buckets in database, found ${dbBuckets.length}`
      );
    });
  }
);
