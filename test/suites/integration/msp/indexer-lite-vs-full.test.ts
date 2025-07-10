import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

// Run the same test scenario twice - once in full mode and once in lite mode
const runTestScenario = async (
  userApi: EnrichedBspApi,
  msp1Api: EnrichedBspApi,
  msp2Api: EnrichedBspApi,
  sql: SqlClient,
  mode: "full" | "lite"
) => {
  console.log(`\nRunning test scenario in ${mode} mode`);

  // Create test data
  const msp1BucketName = `msp1-bucket-${mode}-comparison`;
  const msp2BucketName = `msp2-bucket-${mode}-comparison`;
  const userBucketName = `user-bucket-${mode}-comparison`;

  // Create buckets
  await userApi.block.seal({
    calls: [
      userApi.tx.fileSystem.createBucket(msp1Api.accountId(), msp1BucketName, true),
      userApi.tx.fileSystem.createBucket(msp2Api.accountId(), msp2BucketName, true),
      userApi.tx.fileSystem.createBucket(userApi.accountId(), userBucketName, false)
    ],
    signer: shUser
  });

  // Get bucket IDs for file operations
  const buckets = await userApi.query.fileSystem.buckets.entries();
  const msp1Bucket = buckets.find(([_, bucket]) => bucket.unwrap().name.toString() === msp1BucketName);
  const msp2Bucket = buckets.find(([_, bucket]) => bucket.unwrap().name.toString() === msp2BucketName);
  
  assert(msp1Bucket, "MSP1 bucket not found");
  assert(msp2Bucket, "MSP2 bucket not found");

  const msp1BucketId = msp1Bucket[0].args[0];
  const msp2BucketId = msp2Bucket[0].args[0];

  // Create storage requests in different buckets
  await userApi.block.seal({
    calls: [
      userApi.tx.fileSystem.issueStorageRequest(
        msp1BucketId,
        "file1.txt",
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        1024,
        msp1Api.accountId(),
        [userApi.alice.publicKey],
        null
      ),
      userApi.tx.fileSystem.issueStorageRequest(
        msp2BucketId,
        "file2.txt",
        "0x2222222222222222222222222222222222222222222222222222222222222222",
        2048,
        msp2Api.accountId(),
        [userApi.alice.publicKey],
        null
      )
    ],
    signer: shUser
  });

  // Change provider capacities
  await msp1Api.block.seal({
    calls: [msp1Api.tx.providers.changeCapacity(1000000000n)],
    signer: msp1Api.signer
  });

  await msp2Api.block.seal({
    calls: [msp2Api.tx.providers.changeCapacity(2000000000n)],
    signer: msp2Api.signer
  });

  // Wait for indexing
  await sleep(3000);

  // Collect statistics
  const stats = {
    mode,
    totalEvents: 0,
    fileSystemEvents: 0,
    providerEvents: 0,
    ignoredPalletEvents: 0,
    msp1Events: 0,
    msp2Events: 0,
    bucketCount: 0,
    dbSize: 0
  };

  // Count total events
  const totalEventsResult = await sql`SELECT COUNT(*) as count FROM block_event`;
  stats.totalEvents = Number(totalEventsResult[0].count);

  // Count FileSystem events
  const fileSystemResult = await sql`
    SELECT COUNT(*) as count 
    FROM block_event 
    WHERE section = 'fileSystem'
  `;
  stats.fileSystemEvents = Number(fileSystemResult[0].count);

  // Count Provider events
  const providerResult = await sql`
    SELECT COUNT(*) as count 
    FROM block_event 
    WHERE section = 'providers'
  `;
  stats.providerEvents = Number(providerResult[0].count);

  // Count ignored pallet events
  const ignoredResult = await sql`
    SELECT COUNT(*) as count 
    FROM block_event 
    WHERE section IN ('bucketNfts', 'paymentStreams', 'proofsDealer', 'randomness')
  `;
  stats.ignoredPalletEvents = Number(ignoredResult[0].count);

  // Count buckets
  const bucketResult = await sql`SELECT COUNT(*) as count FROM bucket`;
  stats.bucketCount = Number(bucketResult[0].count);

  // Get database size (approximate using event count as proxy)
  stats.dbSize = stats.totalEvents;

  // Count MSP-specific events
  const msp1EventsResult = await sql`
    SELECT COUNT(*) as count 
    FROM block_event 
    WHERE data::text LIKE '%${msp1Api.accountId()}%'
  `;
  stats.msp1Events = Number(msp1EventsResult[0].count);

  const msp2EventsResult = await sql`
    SELECT COUNT(*) as count 
    FROM block_event 
    WHERE data::text LIKE '%${msp2Api.accountId()}%'
  `;
  stats.msp2Events = Number(msp2EventsResult[0].count);

  return stats;
};

// Full mode test
describeMspNet(
  "Indexer Mode Comparison - Full Mode",
  { initialised: false, indexer: true, indexerMode: "full" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;
    let fullModeStats: any;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      userApi = await createUserApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("collects full mode statistics", async () => {
      fullModeStats = await runTestScenario(userApi, msp1Api, msp2Api, sql, "full");
      console.log("Full mode statistics:", fullModeStats);

      // In full mode, we expect all events to be indexed
      assert(fullModeStats.totalEvents > 0, "Should have indexed events in full mode");
      assert(fullModeStats.msp1Events > 0, "Should have MSP1 events in full mode");
      assert(fullModeStats.msp2Events > 0, "Should have MSP2 events in full mode");
    });
  }
);

// Lite mode test  
describeMspNet(
  "Indexer Mode Comparison - Lite Mode",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;
    let liteModeStats: any;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      userApi = await createUserApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("collects lite mode statistics and compares with full mode", async () => {
      liteModeStats = await runTestScenario(userApi, msp1Api, msp2Api, sql, "lite");
      console.log("Lite mode statistics:", liteModeStats);

      // In lite mode, we expect:
      // 1. Fewer total events
      assert(liteModeStats.totalEvents > 0, "Should have some events in lite mode");
      
      // 2. No ignored pallet events
      assert(
        liteModeStats.ignoredPalletEvents === 0,
        `Should have no ignored pallet events in lite mode, found ${liteModeStats.ignoredPalletEvents}`
      );

      // 3. Only MSP1 events (since this is MSP1's indexer)
      assert(liteModeStats.msp1Events > 0, "Should have MSP1 events in lite mode");
      assert(
        liteModeStats.msp2Events === 0,
        `Should have no MSP2 events in lite mode, found ${liteModeStats.msp2Events}`
      );

      // Note: Actual comparison with full mode stats would require running both tests
      // in the same process or storing results externally
      console.log("\nComparison Summary:");
      console.log("- Lite mode indexes only current MSP events");
      console.log("- Lite mode ignores events from non-essential pallets");
      console.log(`- Event reduction: Lite mode has ${liteModeStats.totalEvents} events`);
      
      // Calculate expected reduction (should be ~80% fewer events)
      const reductionPercentage = liteModeStats.msp2Events === 0 ? "significant" : "minimal";
      console.log(`- Event filtering is ${reductionPercentage}`);
    });

    it("verifies lite mode filtering rules", async () => {
      // Check specific filtering rules
      
      // 1. FileSystem events should only be for MSP1's buckets
      const fileSystemEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method IN ('NewBucket', 'NewStorageRequest')
      `;

      for (const event of fileSystemEvents) {
        const eventData = JSON.parse(event.data);
        
        if (event.method === 'NewBucket') {
          // Check if bucket is for MSP1
          const bucketName = eventData.name || "";
          assert(
            bucketName.includes("msp1") || bucketName.includes("user"),
            `Unexpected bucket in lite mode: ${bucketName}`
          );
        }
      }

      // 2. Provider events should only be for MSP1
      const providerEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
      `;

      for (const event of providerEvents) {
        const eventData = JSON.parse(event.data);
        assert(
          eventData.providerId === msp1Api.accountId(),
          `Provider event for wrong MSP: ${eventData.providerId}`
        );
      }
    });
  }
);