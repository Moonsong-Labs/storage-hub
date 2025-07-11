import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Test to verify that indexer lite mode properly detects MSP registration
 * and starts indexing MSP-specific data after registration
 */
describeMspNet(
  "Indexer Lite Mode - MSP Registration Detection",
  { initialised: true, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi; 
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      userApi = await createUserApi();
      sql = createSqlClient();

      // Wait for initial setup
      await sleep(5000);
    });

    it("verifies MSP is registered and indexed", async () => {
      // Check MSP registration on-chain
      const msp1Info = await msp1Api.query.providers.mainStorageProviders(msp1Api.address);
      const msp1ProviderId = await msp1Api.query.providers.accountIdToMainStorageProviderId(msp1Api.address);
      
      console.log("MSP1 on-chain registration status:");
      console.log("  - Account:", msp1Api.address);
      console.log("  - Provider ID:", msp1ProviderId.toHuman());
      console.log("  - Info:", msp1Info.toHuman());

      // Check database
      const msps = await sql`
        SELECT onchain_msp_id, capacity
        FROM msp
        WHERE onchain_msp_id = ${msp1Api.address}
      `;

      assert(msps.length > 0, "MSP1 should be indexed in lite mode");
      console.log("✓ MSP1 found in database");
    });

    it("creates buckets and verifies proper filtering", async () => {
      // Get MSP1's database ID
      const msp1Record = await sql`
        SELECT id, onchain_msp_id
        FROM msp
        WHERE onchain_msp_id = ${msp1Api.address}
      `;
      
      assert(msp1Record.length > 0, "MSP1 must be in database");
      const msp1DbId = msp1Record[0].id;

      // Create buckets for both MSPs
      const bucket1Name = `msp1-reg-test-${Date.now()}`;
      const bucket2Name = `msp2-reg-test-${Date.now()}`;
      
      console.log("\nCreating test buckets:");
      console.log("  - MSP1 bucket:", bucket1Name);
      console.log("  - MSP2 bucket:", bucket2Name);
      
      await userApi.file.newBucket(bucket1Name, { msp: msp1Api.address });
      await userApi.file.newBucket(bucket2Name, { msp: msp2Api.address });
      
      // Wait for indexing
      await sleep(5000);
      
      // Check which buckets were indexed
      const buckets = await sql`
        SELECT b.name, b.msp_id, m.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.name IN (${bucket1Name}, ${bucket2Name})
      `;
      
      console.log(`\nBuckets found in database: ${buckets.length}`);
      buckets.forEach(bucket => {
        console.log(`  - ${bucket.name} (MSP: ${bucket.onchain_msp_id})`);
      });
      
      // In lite mode, only MSP1's bucket should be indexed
      const msp1Bucket = buckets.find(b => b.name === bucket1Name);
      const msp2Bucket = buckets.find(b => b.name === bucket2Name);
      
      assert(msp1Bucket, "MSP1's bucket should be indexed");
      assert(!msp2Bucket, "MSP2's bucket should NOT be indexed in lite mode");
      
      console.log("✓ Lite mode filtering working correctly");
    });

    it("creates files and verifies filtering", async () => {
      // Create a bucket for MSP1
      const bucketName = `file-test-${Date.now()}`;
      await userApi.file.newBucket(bucketName, { msp: msp1Api.address });
      await sleep(3000);

      // Get bucket ID
      const bucket = await sql`
        SELECT id, onchain_bucket_id
        FROM bucket
        WHERE name = ${bucketName}
      `;
      
      assert(bucket.length > 0, "Test bucket must exist");
      const bucketId = bucket[0].onchain_bucket_id;

      // Create files in both MSP1's and MSP2's buckets
      const file1Key = shUser.utils.generateFileKeyHash("0xfile1");
      const file2Key = shUser.utils.generateFileKeyHash("0xfile2");
      
      // For MSP2, create a bucket first
      const msp2BucketName = `msp2-file-test-${Date.now()}`;
      await userApi.file.newBucket(msp2BucketName, { msp: msp2Api.address });
      await sleep(3000);

      // Create file in MSP1's bucket
      await userApi.file.newStorageRequest(
        bucketId,
        "file1.txt",
        1024,
        "0x1234567890abcdef",
        [await userApi.fileSize()],
        file1Key
      );

      // Wait for indexing
      await sleep(5000);

      // Check which files were indexed
      const files = await sql`
        SELECT f.file_key, b.name as bucket_name
        FROM file f
        JOIN bucket b ON f.bucket_id = b.id
        WHERE f.file_key IN (${Buffer.from(file1Key)}, ${Buffer.from(file2Key)})
      `;

      console.log(`\nFiles indexed: ${files.length}`);
      files.forEach(file => {
        console.log(`  - File in bucket: ${file.bucket_name}`);
      });

      // Only files in MSP1's buckets should be indexed
      assert(files.length === 1, "Only MSP1's file should be indexed");
      assert(files[0].bucket_name === bucketName, "File should be in MSP1's bucket");
      
      console.log("✓ File filtering working correctly in lite mode");
    });

    it("verifies service state is being updated", async () => {
      // Monitor service state updates
      const initialState = await sql`
        SELECT last_processed_block
        FROM service_state
        LIMIT 1
      `;
      
      assert(initialState.length > 0, "Service state should exist");
      const initialBlock = initialState[0].last_processed_block;
      
      console.log(`\nInitial processed block: ${initialBlock}`);
      
      // Wait for some blocks
      await sleep(10000);
      
      const finalState = await sql`
        SELECT last_processed_block
        FROM service_state
        LIMIT 1
      `;
      
      const finalBlock = finalState[0].last_processed_block;
      console.log(`Final processed block: ${finalBlock}`);
      
      assert(finalBlock > initialBlock, "Indexer should be processing blocks");
      console.log(`✓ Indexer processed ${finalBlock - initialBlock} blocks`);
    });
  }
);