import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, waitFor } from "../../../util";
import { getLastIndexedBlock, waitForBucketIndexed } from "../../../util/indexerHelpers";

await describeMspNet(
  "Indexer Service - Block Notification Sync (Mid-Chain Pause)",
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
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);
    });

    after(async () => {
      await indexerApi?.disconnect();
      await sql?.end();
    });

    it("indexes all events when falling behind mid-chain", async () => {
      // Allow indexer to process initial blocks normally before simulating a failure
      // Capture block number after each seal to avoid race conditions
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

      // Verify indexer processed initial buckets - establishes non-genesis baseline
      for (const bucketName of initialBuckets) {
        await waitForBucketIndexed(sql, bucketName);
      }

      const blockBeforePause = await getLastIndexedBlock(sql);

      // Simulate indexer failure mid-chain - more realistic than genesis pause
      await userApi.docker.pauseContainer("storage-hub-sh-indexer-1");

      // Produce additional blocks exceeding sync threshold while indexer is down
      // Capture block number after each seal to avoid race conditions
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

      // Resume indexer to trigger mid-chain catchup scenario
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-indexer-1" });

      // Non-producer nodes must explicitly finalize imported blocks to trigger indexing
      // We need to finalize the exact target block, not just the current finalized head
      const finalBlockHash = (await userApi.rpc.chain.getBlockHash(finalBlockNumber)).toString();

      // Wait for indexer to import the target block via RPC (reliable check, doesn't depend on logs)
      await indexerApi.wait.blockImported(finalBlockHash);

      await indexerApi.block.finaliseBlock(finalBlockHash);

      // Block until indexer processes backlog from mid-chain position
      await waitFor({
        lambda: async () => {
          const lastIndexed = await getLastIndexedBlock(sql);
          return lastIndexed >= finalBlockNumber;
        },
        iterations: 100,
        delay: 500
      });

      // Verify both initial and catchup buckets are present - tests full consistency
      for (const bucketName of [...initialBuckets, ...catchupBuckets]) {
        await waitForBucketIndexed(sql, bucketName);
      }

      // Validate indexer reached target block after mid-chain resume
      const lastIndexedBlock = await getLastIndexedBlock(sql);

      assert(
        lastIndexedBlock >= finalBlockNumber,
        `Indexer should have indexed up to block ${finalBlockNumber}, but only indexed ${lastIndexedBlock}`
      );

      // Final consistency check - all buckets from both phases present in database
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
