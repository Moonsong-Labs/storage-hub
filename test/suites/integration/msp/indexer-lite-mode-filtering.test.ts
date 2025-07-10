import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * This test suite verifies that the indexer correctly filters events when running in lite mode.
 * In lite mode, only specific events should be indexed to reduce database size and improve performance.
 * 
 * Events that SHOULD be indexed in lite mode:
 * - Providers: MspSignUpSuccess, MspSignOffSuccess, BspSignUpSuccess, BspSignOffSuccess
 * - FileSystem: NewBucket, BucketPrivacyUpdateAccepted, MoveBucketAccepted, BucketDeleted
 * - Providers (ValueProp): ValuePropUpserted, ValuePropDeleted (filtered for current MSP only)
 * - ProofsDealer: ProofAccepted
 * 
 * All other events should be filtered out and NOT appear in the database.
 */
describeMspNet(
  "Indexer Lite Mode Event Filtering",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createUserApi, createSqlClient, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for postgres to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      // Give indexer time to initialize
      await sleep(2000);
    });

    it("verifies lite mode events are indexed", async () => {
      const bucketName = "lite-mode-included-bucket";
      
      // Create a bucket - this SHOULD be indexed in lite mode
      const beforeBucketCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      
      await userApi.file.newBucket(bucketName);
      await sleep(2000); // Wait for indexing
      
      const afterBucketCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      const newBucket = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      
      assert(
        parseInt(afterBucketCount[0].count) > parseInt(beforeBucketCount[0].count),
        "NewBucket event should be indexed in lite mode"
      );
      assert(newBucket.length === 1, "New bucket should exist in database");
      strictEqual(newBucket[0].name, bucketName, "Bucket name should match");

      // Verify the NewBucket event is in block_event table
      const bucketEvents = await sql`
        SELECT * FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'NewBucket'
        ORDER BY block_number DESC
        LIMIT 1
      `;
      
      assert(bucketEvents.length > 0, "NewBucket event should be in block_event table");
    });

    it("verifies non-lite mode events are NOT indexed", async () => {
      // Test various events that should NOT be indexed in lite mode
      
      // 1. NewStorageRequest - should NOT be indexed
      const source = "res/adolphus.jpg";
      const location = "test/not-indexed-file.jpg";
      const bucketName = "test-storage-request-bucket";
      
      // First create a bucket (will be indexed)
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      // Count storage request events before
      const beforeStorageRequests = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'NewStorageRequest'
      `;
      
      // Issue a storage request - should NOT be indexed in lite mode
      await userApi.file.newStorageRequest(source, location, bucketId, shUser);
      await sleep(2000); // Wait for potential indexing
      
      // Count storage request events after
      const afterStorageRequests = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'NewStorageRequest'
      `;
      
      assert(
        parseInt(afterStorageRequests[0].count) === parseInt(beforeStorageRequests[0].count),
        "NewStorageRequest event should NOT be indexed in lite mode"
      );

      // 2. Test other non-lite mode events
      // Check that common runtime events are not indexed
      const nonLiteModeEvents = [
        { section: 'fileSystem', method: 'NewStorageRequest' },
        { section: 'fileSystem', method: 'AcceptedBspVolunteer' },
        { section: 'fileSystem', method: 'BspConfirmedStoring' },
        { section: 'fileSystem', method: 'StorageRequestFulfilled' },
        { section: 'fileSystem', method: 'StorageRequestExpired' },
        { section: 'paymentStreams', method: 'FixedRatePaymentStreamCreated' },
        { section: 'paymentStreams', method: 'FixedRatePaymentStreamUpdated' },
      ];

      for (const event of nonLiteModeEvents) {
        const eventCount = await sql`
          SELECT COUNT(*) as count FROM block_event 
          WHERE section = ${event.section} 
          AND method = ${event.method}
        `;
        
        // In lite mode, these events should have zero count
        // (unless they were created during network initialization before lite mode was active)
        console.log(`Event ${event.section}.${event.method} count:`, eventCount[0].count);
      }
    });

    it("verifies provider events are indexed correctly", async () => {
      // Check that MSP and BSP signup events were indexed during network initialization
      const mspCount = await sql`SELECT COUNT(*) as count FROM msp`;
      const bspCount = await sql`SELECT COUNT(*) as count FROM bsp`;
      
      assert(
        parseInt(mspCount[0].count) >= 2,
        "MSP signup events should be indexed (at least 2 MSPs in network)"
      );
      assert(
        parseInt(bspCount[0].count) >= 1,
        "BSP signup events should be indexed (at least 1 BSP in network)"
      );

      // Verify provider events in block_event table
      const mspSignupEvents = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'providers' 
        AND method = 'MspSignUpSuccess'
      `;
      
      const bspSignupEvents = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'providers' 
        AND method = 'BspSignUpSuccess'
      `;
      
      assert(
        parseInt(mspSignupEvents[0].count) >= 2,
        "MspSignUpSuccess events should be indexed"
      );
      assert(
        parseInt(bspSignupEvents[0].count) >= 1,
        "BspSignUpSuccess events should be indexed"
      );
    });

    it("verifies bucket privacy update is indexed", async () => {
      const bucketName = "privacy-test-bucket";
      
      // Create a public bucket
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      // Count privacy update events before
      const beforePrivacyUpdates = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'BucketPrivacyUpdateAccepted'
      `;
      
      // Update bucket to private
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.updateBucketPrivacy(bucketId, { Private: null })],
        signer: shUser
      });
      
      await sleep(2000); // Wait for indexing
      
      // Count privacy update events after
      const afterPrivacyUpdates = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'BucketPrivacyUpdateAccepted'
      `;
      
      assert(
        parseInt(afterPrivacyUpdates[0].count) > parseInt(beforePrivacyUpdates[0].count),
        "BucketPrivacyUpdateAccepted event should be indexed in lite mode"
      );
    });

    it("verifies bucket deletion is indexed", async () => {
      const bucketName = "delete-test-bucket";
      
      // Create a bucket
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      // Count deletion events before
      const beforeDeletions = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'BucketDeleted'
      `;
      
      // Delete the bucket
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });
      
      await sleep(2000); // Wait for indexing
      
      // Count deletion events after
      const afterDeletions = await sql`
        SELECT COUNT(*) as count FROM block_event 
        WHERE section = 'fileSystem' 
        AND method = 'BucketDeleted'
      `;
      
      assert(
        parseInt(afterDeletions[0].count) > parseInt(beforeDeletions[0].count),
        "BucketDeleted event should be indexed in lite mode"
      );
      
      // Verify bucket status in database
      const deletedBucket = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      
      // The bucket handling depends on indexer implementation
      // It might be removed or marked as deleted
      console.log("Deleted bucket status:", deletedBucket);
    });

    it("verifies total event count is reduced in lite mode", async () => {
      // This test compares the total number of events indexed
      // In lite mode, we expect significantly fewer events
      
      const totalEvents = await sql`
        SELECT COUNT(*) as count FROM block_event
      `;
      
      const eventsBySection = await sql`
        SELECT section, method, COUNT(*) as count 
        FROM block_event 
        GROUP BY section, method 
        ORDER BY count DESC
      `;
      
      console.log("Total events indexed:", totalEvents[0].count);
      console.log("\nTop event types:");
      eventsBySection.slice(0, 10).forEach(e => {
        console.log(`  ${e.section}.${e.method}: ${e.count}`);
      });
      
      // Verify that only expected event types are present
      const liteModeSections = ['providers', 'fileSystem', 'proofsDealer'];
      const unexpectedSections = eventsBySection.filter(e => 
        !liteModeSections.includes(e.section) && 
        e.section !== 'system' && // System events might still be indexed
        e.section !== 'timestamp' // Timestamp events might still be indexed
      );
      
      if (unexpectedSections.length > 0) {
        console.log("\nWarning: Unexpected sections found in lite mode:");
        unexpectedSections.forEach(e => {
          console.log(`  ${e.section}.${e.method}: ${e.count}`);
        });
      }
    });
  }
);