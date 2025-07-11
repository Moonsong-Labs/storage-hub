import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * This test suite verifies that the indexer correctly filters data when running in lite mode.
 * In lite mode, only data relevant to the current MSP (MSP1) should be indexed to reduce database size
 * and improve performance.
 * 
 * Domain tables that SHOULD contain data in lite mode:
 * - msp: All MSPs (needed for provider relationships)
 * - bsp: All BSPs (needed for provider relationships)
 * - bucket: Only buckets owned by users of MSP1
 * - file: Only files in buckets owned by users of MSP1
 * - proof: Only proofs submitted by MSP1
 * 
 * The test verifies filtering by checking domain tables, not the block_event table.
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

    it("verifies buckets are indexed in lite mode", async () => {
      const bucketName = "lite-mode-included-bucket";
      
      // Get current MSP1 ID (the MSP running the indexer)
      const msp1 = await sql`SELECT id FROM msp WHERE multiaddress LIKE '%5001%' LIMIT 1`;
      assert(msp1.length > 0, "MSP1 should exist in database");
      const msp1Id = msp1[0].id;
      
      // Create a bucket - this SHOULD be indexed in lite mode since user is on MSP1
      const beforeBucketCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      
      await userApi.file.newBucket(bucketName);
      await sleep(2000); // Wait for indexing
      
      const afterBucketCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      const newBucket = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      
      assert(
        parseInt(afterBucketCount[0].count) > parseInt(beforeBucketCount[0].count),
        "Bucket should be indexed in lite mode for MSP1 users"
      );
      assert(newBucket.length === 1, "New bucket should exist in database");
      strictEqual(newBucket[0].name, bucketName, "Bucket name should match");
      
      // Verify bucket is associated with MSP1
      const bucketWithMsp = await sql`
        SELECT b.*, u.msp_id 
        FROM bucket b
        JOIN "user" u ON b.owner_id = u.id
        WHERE b.name = ${bucketName}
      `;
      
      assert(bucketWithMsp.length === 1, "Bucket should be associated with a user");
      strictEqual(bucketWithMsp[0].msp_id, msp1Id, "Bucket should belong to a user of MSP1");
    });

    it("verifies files are NOT indexed in lite mode", async () => {
      // In lite mode, file events (NewStorageRequest) should NOT be indexed
      // This means no entries should be created in the file table
      
      const source = "res/adolphus.jpg";
      const location = "test/not-indexed-file.jpg";
      const bucketName = "test-storage-request-bucket";
      
      // First create a bucket (will be indexed since user is on MSP1)
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      // Count files before
      const beforeFileCount = await sql`SELECT COUNT(*) as count FROM file`;
      
      // Issue a storage request - should NOT create a file entry in lite mode
      await userApi.file.newStorageRequest(source, location, bucketId, shUser);
      await sleep(2000); // Wait for potential indexing
      
      // Count files after
      const afterFileCount = await sql`SELECT COUNT(*) as count FROM file`;
      
      assert(
        parseInt(afterFileCount[0].count) === parseInt(beforeFileCount[0].count),
        "File entries should NOT be created in lite mode"
      );
      
      // Verify no file exists with this location
      const fileCheck = await sql`SELECT * FROM file WHERE location = ${location}`;
      assert(fileCheck.length === 0, "No file entry should exist for storage requests in lite mode");
      
      // Also verify that payment stream related data is not indexed
      const paymentStreams = await sql`SELECT COUNT(*) as count FROM fixed_rate_payment_stream`;
      console.log(`Payment streams count: ${paymentStreams[0].count}`);
      
      // In lite mode, payment streams should not be indexed at all
      assert(
        parseInt(paymentStreams[0].count) === 0,
        "Payment streams should not be indexed in lite mode"
      );
    });

    it("verifies provider data is indexed correctly", async () => {
      // Check that MSP and BSP data is present in domain tables
      const mspCount = await sql`SELECT COUNT(*) as count FROM msp`;
      const bspCount = await sql`SELECT COUNT(*) as count FROM bsp`;
      
      assert(
        parseInt(mspCount[0].count) >= 2,
        "MSP data should be indexed (at least 2 MSPs in network)"
      );
      assert(
        parseInt(bspCount[0].count) >= 1,
        "BSP data should be indexed (at least 1 BSP in network)"
      );
      
      // Verify MSP details are present
      const msps = await sql`SELECT id, multiaddress FROM msp ORDER BY id`;
      assert(msps.length >= 2, "Should have at least 2 MSPs");
      
      // Check that MSP1 (port 5001) exists
      const msp1 = msps.find(m => m.multiaddress && m.multiaddress.includes('5001'));
      assert(msp1, "MSP1 (port 5001) should exist in database");
      
      // Check that MSP2 (port 5002) exists
      const msp2 = msps.find(m => m.multiaddress && m.multiaddress.includes('5002'));
      assert(msp2, "MSP2 (port 5002) should exist in database");
      
      // Verify BSP details
      const bsps = await sql`SELECT id, multiaddress FROM bsp LIMIT 5`;
      assert(bsps.length >= 1, "Should have at least 1 BSP");
      assert(bsps[0].multiaddress, "BSP should have multiaddress");
      
      console.log(`Found ${mspCount[0].count} MSPs and ${bspCount[0].count} BSPs in database`);
    });

    it("verifies bucket privacy update is reflected in domain table", async () => {
      const bucketName = "privacy-test-bucket";
      
      // Create a public bucket
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      await sleep(2000); // Wait for indexing
      
      // Check initial bucket privacy (should be public by default)
      const bucketBefore = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      assert(bucketBefore.length === 1, "Bucket should exist");
      assert(!bucketBefore[0].private, "Bucket should be public initially");
      
      // Update bucket to private
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.updateBucketPrivacy(bucketId, { Private: null })],
        signer: shUser
      });
      
      await sleep(2000); // Wait for indexing
      
      // Check bucket privacy after update
      const bucketAfter = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      assert(bucketAfter.length === 1, "Bucket should still exist");
      assert(bucketAfter[0].private === true, "Bucket should be private after update");
      
      console.log(`Bucket ${bucketName} privacy updated from public to private`);
    });

    it("verifies bucket deletion is reflected in domain table", async () => {
      const bucketName = "delete-test-bucket";
      
      // Create a bucket
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");
      
      await sleep(2000); // Wait for indexing
      
      // Verify bucket exists before deletion
      const bucketBefore = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      assert(bucketBefore.length === 1, "Bucket should exist before deletion");
      
      // Delete the bucket
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });
      
      await sleep(2000); // Wait for indexing
      
      // Verify bucket is removed from database after deletion
      const bucketAfter = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      assert(bucketAfter.length === 0, "Bucket should be removed from database after deletion");
      
      console.log(`Bucket ${bucketName} successfully deleted from database`);
    });

    it("verifies domain table data is properly filtered for MSP1", async () => {
      // This test verifies that only MSP1-relevant data is indexed in lite mode
      
      // Get MSP1 ID
      const msp1 = await sql`SELECT id FROM msp WHERE multiaddress LIKE '%5001%' LIMIT 1`;
      assert(msp1.length > 0, "MSP1 should exist");
      const msp1Id = msp1[0].id;
      
      // 1. Check bucket table - all buckets should belong to MSP1 users
      const buckets = await sql`
        SELECT b.id, b.name, u.msp_id 
        FROM bucket b
        JOIN "user" u ON b.owner_id = u.id
      `;
      
      console.log(`Total buckets: ${buckets.length}`);
      const nonMsp1Buckets = buckets.filter(b => b.msp_id !== msp1Id);
      assert(
        nonMsp1Buckets.length === 0,
        `All buckets should belong to MSP1 users, found ${nonMsp1Buckets.length} buckets from other MSPs`
      );
      
      // 2. Check user table - all users should be associated with MSP1
      const users = await sql`SELECT id, msp_id FROM "user"`;
      console.log(`Total users: ${users.length}`);
      const nonMsp1Users = users.filter(u => u.msp_id !== msp1Id);
      assert(
        nonMsp1Users.length === 0,
        `All users should be associated with MSP1, found ${nonMsp1Users.length} users from other MSPs`
      );
      
      // 3. Verify file table is empty (files are not indexed in lite mode)
      const fileCount = await sql`SELECT COUNT(*) as count FROM file`;
      assert(
        parseInt(fileCount[0].count) === 0,
        "File table should be empty in lite mode"
      );
      
      // 4. Check that all providers are indexed (both MSPs and BSPs)
      const mspCount = await sql`SELECT COUNT(*) as count FROM msp`;
      const bspCount = await sql`SELECT COUNT(*) as count FROM bsp`;
      assert(parseInt(mspCount[0].count) >= 2, "All MSPs should be indexed");
      assert(parseInt(bspCount[0].count) >= 1, "All BSPs should be indexed");
      
      // 5. Verify no payment streams are indexed
      const paymentStreamCount = await sql`SELECT COUNT(*) as count FROM fixed_rate_payment_stream`;
      assert(
        parseInt(paymentStreamCount[0].count) === 0,
        "Payment streams should not be indexed in lite mode"
      );
      
      console.log("\nLite mode filtering summary:");
      console.log(`- MSPs indexed: ${mspCount[0].count}`);
      console.log(`- BSPs indexed: ${bspCount[0].count}`);
      console.log(`- Buckets (MSP1 only): ${buckets.length}`);
      console.log(`- Users (MSP1 only): ${users.length}`);
      console.log(`- Files: ${fileCount[0].count} (should be 0)`);
      console.log(`- Payment streams: ${paymentStreamCount[0].count} (should be 0)`);
    });
  }
);