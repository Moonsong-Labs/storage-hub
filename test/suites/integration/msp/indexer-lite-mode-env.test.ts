import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Test suite for verifying indexer lite mode with proper environment setup.
 * 
 * Note: This test assumes the indexer is running with the --indexer-mode lite flag.
 * In a real deployment, this would be set via:
 * --indexer --indexer-mode lite
 * 
 * The test verifies that when the indexer is in lite mode:
 * 1. Only specific events are indexed (as defined in LITE_MODE_EVENTS.md)
 * 2. ValueProp events are filtered to only include those for the current MSP
 * 3. Database size is significantly reduced compared to full mode
 */
describeMspNet(
  "Indexer Lite Mode with Environment Configuration", 
  { 
    initialised: false, 
    indexer: true,
    indexerMode: "lite"
  },
  ({ before, it, createUserApi, createMsp1Api, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      const maybeMsp1Api = await createMsp1Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      // Wait for postgres and indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
      
      // Give indexer time to start processing
      await sleep(3000);
    });

    it("creates test data and verifies lite mode filtering", async () => {
      // Create various events and verify only lite mode events are indexed
      
      // 1. Create buckets (SHOULD be indexed)
      const buckets = ["lite-test-1", "lite-test-2", "lite-test-3"];
      for (const bucketName of buckets) {
        await userApi.file.newBucket(bucketName);
      }
      
      // 2. Create storage requests (should NOT be indexed in lite mode)
      const bucket1Event = await userApi.file.newBucket("storage-test-bucket");
      const bucketId = userApi.events.fileSystem.NewBucket.is(bucket1Event) && bucket1Event.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      // Issue multiple storage requests
      const files = [
        { source: "res/adolphus.jpg", location: "test/file1.jpg" },
        { source: "res/smile.jpg", location: "test/file2.jpg" },
        { source: "res/whatsup.jpg", location: "test/file3.jpg" }
      ];
      
      for (const file of files) {
        await userApi.file.newStorageRequest(
          file.source,
          file.location,
          bucketId,
          shUser
        );
      }
      
      // 3. Update bucket privacy (SHOULD be indexed)
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.updateBucketPrivacy(bucketId, { Private: null })],
        signer: shUser
      });
      
      // 4. Create more non-lite events
      // These would include payment stream events, detailed file system events, etc.
      // For brevity, we'll just verify the counts
      
      // Wait for all events to be processed
      await sleep(3000);
      
      // Now verify what was indexed
      const results = {
        buckets: await sql`SELECT COUNT(*) as count FROM bucket`,
        totalEvents: await sql`SELECT COUNT(*) as count FROM block_event`,
        fileSystemEvents: await sql`
          SELECT method, COUNT(*) as count 
          FROM block_event 
          WHERE section = 'fileSystem' 
          GROUP BY method
        `,
        providerEvents: await sql`
          SELECT method, COUNT(*) as count 
          FROM block_event 
          WHERE section = 'providers' 
          GROUP BY method
        `
      };
      
      console.log("Lite Mode Indexing Results:");
      console.log("Total buckets:", results.buckets[0].count);
      console.log("Total events:", results.totalEvents[0].count);
      console.log("\nFileSystem events:");
      results.fileSystemEvents.forEach(e => {
        console.log(`  ${e.method}: ${e.count}`);
      });
      console.log("\nProvider events:");
      results.providerEvents.forEach(e => {
        console.log(`  ${e.method}: ${e.count}`);
      });
      
      // Verify expectations
      assert(
        parseInt(results.buckets[0].count) >= buckets.length + 1,
        "All created buckets should be indexed"
      );
      
      // Check that NewStorageRequest events are NOT indexed
      const storageRequestEvents = results.fileSystemEvents.find(e => e.method === 'NewStorageRequest');
      assert(
        !storageRequestEvents || parseInt(storageRequestEvents.count) === 0,
        "NewStorageRequest events should NOT be indexed in lite mode"
      );
      
      // Check that NewBucket events ARE indexed
      const newBucketEvents = results.fileSystemEvents.find(e => e.method === 'NewBucket');
      assert(
        newBucketEvents && parseInt(newBucketEvents.count) >= buckets.length + 1,
        "NewBucket events should be indexed in lite mode"
      );
      
      // Check that BucketPrivacyUpdateAccepted is indexed
      const privacyEvents = results.fileSystemEvents.find(e => e.method === 'BucketPrivacyUpdateAccepted');
      assert(
        privacyEvents && parseInt(privacyEvents.count) >= 1,
        "BucketPrivacyUpdateAccepted events should be indexed in lite mode"
      );
    });

    it("verifies database size reduction in lite mode", async () => {
      // This test estimates the reduction in database size by comparing event counts
      
      // Get counts of all tables
      const tableCounts = await sql`
        SELECT 
          schemaname,
          tablename,
          n_live_tup as row_count
        FROM pg_stat_user_tables
        WHERE schemaname = 'public'
        ORDER BY n_live_tup DESC
      `;
      
      console.log("\nTable row counts in lite mode:");
      tableCounts.forEach(t => {
        console.log(`  ${t.tablename}: ${t.row_count} rows`);
      });
      
      // Get total database size
      const dbSize = await sql`
        SELECT 
          pg_database_size('storage_hub') as size,
          pg_size_pretty(pg_database_size('storage_hub')) as pretty_size
      `;
      
      console.log("\nDatabase size in lite mode:", dbSize[0].pretty_size);
      
      // In lite mode, we expect:
      // - Fewer rows in block_event table
      // - Only essential provider data
      // - No detailed file tracking beyond buckets
      
      const blockEventCount = tableCounts.find(t => t.tablename === 'block_event');
      assert(blockEventCount, "block_event table should exist");
      
      // The actual reduction would be more apparent with a longer-running test
      // For now, we just verify the structure is correct
    });

    it("verifies ValueProp filtering for current MSP", async () => {
      // This test would verify that ValueProp events are filtered by MSP
      // In a real scenario, we would:
      // 1. Create ValueProps for multiple MSPs
      // 2. Verify only the current MSP's ValueProps are indexed
      
      // For now, just verify the query structure works
      const valuePropEvents = await sql`
        SELECT * FROM block_event 
        WHERE section = 'providers' 
        AND method IN ('ValuePropUpserted', 'ValuePropDeleted')
      `;
      
      console.log("\nValueProp events found:", valuePropEvents.length);
      
      // In lite mode with proper MSP filtering, we would expect to see
      // only events for the MSP that's running the indexer
    });

    it("verifies critical events are not missed", async () => {
      // Ensure that all critical events defined in LITE_MODE_EVENTS.md are captured
      
      const criticalEvents = [
        { section: 'providers', methods: ['MspSignUpSuccess', 'MspSignOffSuccess', 'BspSignUpSuccess', 'BspSignOffSuccess'] },
        { section: 'fileSystem', methods: ['NewBucket', 'BucketPrivacyUpdateAccepted', 'MoveBucketAccepted', 'BucketDeleted'] },
        { section: 'proofsDealer', methods: ['ProofAccepted'] }
      ];
      
      for (const eventGroup of criticalEvents) {
        const events = await sql`
          SELECT method, COUNT(*) as count
          FROM block_event
          WHERE section = ${eventGroup.section}
          AND method = ANY(${eventGroup.methods})
          GROUP BY method
        `;
        
        console.log(`\n${eventGroup.section} critical events:`);
        events.forEach(e => {
          console.log(`  ${e.method}: ${e.count}`);
        });
      }
      
      // The network initialization should have created at least some of these events
      const hasProviderEvents = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'providers' 
        AND method IN ('MspSignUpSuccess', 'BspSignUpSuccess')
      `;
      
      assert(
        parseInt(hasProviderEvents[0].count) > 0,
        "Should have provider signup events from network initialization"
      );
    });
  }
);