import assert from "node:assert";
import Docker from "dockerode";
import { describeMspNet, type EnrichedBspApi, type SqlClient, waitFor } from "../../../util";
import { getLastIndexedBlock, waitForBucketIndexed } from "../../../util/indexerHelpers";

await describeMspNet(
  "Indexer Service - Database Connection Resilience During Resync",
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
    let docker: Docker;
    let postgresContainer: Docker.Container;

    before(async () => {
      userApi = await createUserApi();
      await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      sql = createSqlClient();

      // Create API connection to standalone indexer service (port 9800)
      indexerApi = await createApi("ws://127.0.0.1:9800");

      // Initialize Docker client for database container control
      docker = new Docker();
      postgresContainer = docker.getContainer("storage-hub-sh-postgres-1");

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);
    });

    after(async () => {
      await indexerApi?.disconnect();
      await sql?.end();
    });

    it("retries processing blocks when database connection is temporarily unavailable during resync", async () => {
      // Phase 1: Establish baseline - allow indexer to process initial blocks normally
      const initialBuckets: string[] = [];
      let initialBlockNumber = 0;
      for (let i = 0; i < 3; i++) {
        const bucketName = `test-bucket-initial-${i}`;
        await userApi.createBucket(bucketName);
        await userApi.block.seal({ finaliseBlock: true });
        const header = await userApi.rpc.chain.getHeader();
        initialBlockNumber = header.number.toNumber();
        initialBuckets.push(bucketName);
      }

      // Finalize blocks on indexer node so it can process the initial buckets
      const initialFinalizedHash = await userApi.rpc.chain.getFinalizedHead();

      await indexerApi.wait.blockImported(initialFinalizedHash.toString());

      await indexerApi.block.finaliseBlock(initialFinalizedHash.toString());

      // Wait for indexer to catch up to initial buckets
      await waitFor({
        lambda: async () => {
          const lastIndexed = await getLastIndexedBlock(sql);
          return lastIndexed >= initialBlockNumber;
        },
        iterations: 100,
        delay: 500
      });

      // Verify indexer processed initial buckets - establishes healthy baseline
      for (const bucketName of initialBuckets) {
        await waitForBucketIndexed(sql, bucketName);
      }

      const blockBeforePause = await getLastIndexedBlock(sql);

      // Phase 2: Create catchup scenario - pause indexer while blockchain advances
      await userApi.docker.pauseContainer("storage-hub-sh-indexer-1");

      // Produce blocks exceeding sync threshold (7 blocks > 5 threshold)
      const catchupBuckets: string[] = [];
      let finalBlockNumber = 0;
      for (let i = 0; i < 7; i++) {
        const bucketName = `test-bucket-catchup-${i}`;
        await userApi.createBucket(bucketName);
        await userApi.block.seal({ finaliseBlock: true });
        const header = await userApi.rpc.chain.getHeader();
        finalBlockNumber = header.number.toNumber();
        catchupBuckets.push(bucketName);
      }

      // Confirm indexer remained frozen at pre-pause block
      const lastIndexedBeforeCatchup = await getLastIndexedBlock(sql);

      assert.equal(
        lastIndexedBeforeCatchup,
        blockBeforePause,
        `Indexer should still be at block ${blockBeforePause} but is at ${lastIndexedBeforeCatchup}`
      );

      // Phase 3: Simulate database connection failure during resync
      // Immediately pause the database to simulate connection issues during resync
      await postgresContainer.pause();

      // Resume indexer first to start the resync process
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-indexer-1" });

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-indexer-1",
        timeout: 10000
      });

      // Non-producer nodes must explicitly finalize imported blocks to trigger indexing
      // Get the final block hash that we need the indexer to process
      const finalBlockHash = (await userApi.rpc.chain.getBlockHash(finalBlockNumber)).toString();

      // Wait for indexer to import the target block via RPC
      await indexerApi.wait.blockImported(finalBlockHash);

      // Explicitly finalize the block on the indexer node to trigger indexing
      await indexerApi.block.finaliseBlock(finalBlockHash);

      // Seal a block on user node to trigger finality notification on indexer
      // The indexer will receive the notification but fail to process due to DB connection timeout
      await userApi.block.seal({ finaliseBlock: true });

      // Wait for DB connection timeout (15s) + buffer time for retry attempts
      // The indexer will be stuck trying to acquire DB connections during this period
      await new Promise((resolve) => setTimeout(resolve, 20000));

      // Phase 4: Verify recovery - resume database and confirm indexer catches up
      await postgresContainer.unpause();

      // Wait for database to be ready to accept connections again
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });

      // Verify indexer is still stuck at the same block (no progress during DB outage)
      const lastIndexedDuringOutage = await getLastIndexedBlock(sql);

      assert.equal(
        lastIndexedDuringOutage,
        blockBeforePause,
        `Indexer should still be stuck at block ${blockBeforePause} during DB outage, but is at ${lastIndexedDuringOutage}`
      );

      // Create and finalize a trigger block to generate a new finality notification
      // This demonstrates that the indexer needs a new finality notification to resume processing
      // The indexer will catch up on ALL missed blocks (not just this new one) because
      // handle_finality_notification processes from last_indexed_block to current finalized block
      const triggerBlockNumber = finalBlockNumber + 1;
      await userApi.block.seal({ finaliseBlock: true });
      const triggerHash = (await userApi.rpc.chain.getBlockHash(triggerBlockNumber)).toString();

      // Wait for indexer to import and finalize the trigger block
      await indexerApi.wait.blockImported(triggerHash);
      await indexerApi.block.finaliseBlock(triggerHash);

      // The indexer should now process all missed blocks (from last_indexed_block to trigger block)
      // Wait for it to catch up to the trigger block number
      await waitFor({
        lambda: async () => {
          const lastIndexed = await getLastIndexedBlock(sql);
          return lastIndexed >= triggerBlockNumber;
        },
        iterations: 100,
        delay: 500
      });

      // Verify all buckets from both phases are present - tests full consistency
      for (const bucketName of [...initialBuckets, ...catchupBuckets]) {
        await waitForBucketIndexed(sql, bucketName);
      }

      // Final validation - indexer should have successfully processed all blocks including trigger
      const lastIndexedBlock = await getLastIndexedBlock(sql);

      assert(
        lastIndexedBlock >= triggerBlockNumber,
        `Indexer should have indexed up to block ${triggerBlockNumber}, but only indexed ${lastIndexedBlock}`
      );

      // Verify database consistency - all buckets should be present
      const allBuckets = await sql`
        SELECT name FROM bucket
        WHERE name LIKE 'test-bucket-%'
        ORDER BY name
      `;

      const expectedBucketCount = initialBuckets.length + catchupBuckets.length;

      assert.equal(
        allBuckets.length,
        expectedBucketCount,
        `Expected ${expectedBucketCount} buckets in database, found ${allBuckets.length}`
      );
    });
  }
);
