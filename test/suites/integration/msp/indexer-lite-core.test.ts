import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Core test for indexer lite mode filtering functionality.
 * 
 * This test comprehensively verifies that in lite mode:
 * 1. Only MSP1's data is indexed (buckets, files, events)
 * 2. MSP2's data is completely filtered out
 * 3. Domain tables (msp, bucket, file) contain only MSP1 data
 * 4. Event filtering works correctly for MSP-specific operations
 * 
 * The test uses SQL queries against domain tables rather than block_event
 * to verify the actual business data filtering.
 */
describeMspNet(
  "Indexer Lite Mode Core Filtering",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient, createBspApi }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      // Initialize APIs
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
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
      await sleep(3000);
    });

    it("verifies only MSP1 exists in the msp table", async () => {
      // Query all MSPs in the database
      const allMsps = await sql`
        SELECT onchain_msp_id, capacity, used_capacity, multiaddresses
        FROM msp
        ORDER BY onchain_msp_id
      `;

      console.log("MSPs in database:", allMsps.length);
      allMsps.forEach(msp => {
        console.log(`  - MSP ID: ${msp.onchain_msp_id}`);
        console.log(`    Capacity: ${msp.capacity}, Used: ${msp.used_capacity}`);
      });

      // In lite mode, only MSP1 should exist
      assert(
        allMsps.length === 1,
        `Expected only 1 MSP (MSP1) in lite mode, found ${allMsps.length}`
      );
      
      assert(
        allMsps[0].onchain_msp_id === userApi.shConsts.NODE_INFOS.msp1.AddressId,
        `Expected MSP1 (${userApi.shConsts.NODE_INFOS.msp1.AddressId}) but found ${allMsps[0].onchain_msp_id}`
      );

      // Verify MSP2 is NOT in the database
      const msp2Check = await sql`
        SELECT COUNT(*) as count
        FROM msp
        WHERE onchain_msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}
      `;

      strictEqual(
        parseInt(msp2Check[0].count),
        0,
        "MSP2 should not exist in the database in lite mode"
      );
    });

    it("creates buckets for both MSPs and verifies filtering", async () => {
      // Create unique bucket names
      const msp1BucketName = `msp1-bucket-${Date.now()}`;
      const msp2BucketName = `msp2-bucket-${Date.now()}`;

      // Create bucket for MSP1 (should be indexed)
      console.log(`Creating bucket for MSP1: ${msp1BucketName}`);
      const msp1BucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            msp1BucketName,
            true // public
          )
        ],
        signer: shUser
      });

      const msp1BucketId = userApi.events.fileSystem.NewBucket.is(msp1BucketEvent) && 
                           msp1BucketEvent.data.bucketId;
      assert(msp1BucketId, "Failed to get MSP1 bucket ID");

      // Create bucket for MSP2 (should NOT be indexed)
      console.log(`Creating bucket for MSP2: ${msp2BucketName}`);
      const msp2BucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            msp2BucketName,
            true // public
          )
        ],
        signer: shUser
      });

      const msp2BucketId = userApi.events.fileSystem.NewBucket.is(msp2BucketEvent) && 
                           msp2BucketEvent.data.bucketId;
      assert(msp2BucketId, "Failed to get MSP2 bucket ID");

      // Wait for indexing
      await sleep(3000);

      // Query buckets in the database
      const allBuckets = await sql`
        SELECT bucket_id, name, msp_id, user_id, size, available_capacity
        FROM bucket
        WHERE name IN (${msp1BucketName}, ${msp2BucketName})
      `;

      console.log(`\nBuckets found in database: ${allBuckets.length}`);
      allBuckets.forEach(bucket => {
        console.log(`  - Bucket: ${bucket.name}`);
        console.log(`    ID: ${bucket.bucket_id}, MSP: ${bucket.msp_id}`);
      });

      // Verify only MSP1's bucket is indexed
      assert(
        allBuckets.length === 1,
        `Expected only MSP1's bucket, found ${allBuckets.length} buckets`
      );

      const indexedBucket = allBuckets[0];
      strictEqual(
        indexedBucket.name,
        msp1BucketName,
        "Only MSP1's bucket should be indexed"
      );
      strictEqual(
        indexedBucket.msp_id,
        userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "Bucket should belong to MSP1"
      );

      // Verify MSP2's bucket is NOT in the database
      const msp2BucketCheck = await sql`
        SELECT COUNT(*) as count
        FROM bucket
        WHERE name = ${msp2BucketName}
        OR msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}
      `;

      strictEqual(
        parseInt(msp2BucketCheck[0].count),
        0,
        "MSP2's bucket should not exist in the database"
      );

      // Also check by bucket ID
      const bucketByIdCheck = await sql`
        SELECT bucket_id, name, msp_id
        FROM bucket
        WHERE bucket_id IN (${msp1BucketId.toString()}, ${msp2BucketId.toString()})
      `;

      assert(
        bucketByIdCheck.length === 1,
        "Should only find MSP1's bucket by ID"
      );
      strictEqual(
        bucketByIdCheck[0].bucket_id,
        msp1BucketId.toString(),
        "Only MSP1's bucket should be found by ID"
      );
    });

    it("creates files in both MSPs' buckets and verifies filtering", async () => {
      // First create buckets for both MSPs
      const msp1FileBucketName = `msp1-file-bucket-${Date.now()}`;
      const msp2FileBucketName = `msp2-file-bucket-${Date.now()}`;

      // Create MSP1 bucket
      const msp1BucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            msp1FileBucketName,
            true
          )
        ],
        signer: shUser
      });

      const msp1BucketId = userApi.events.fileSystem.NewBucket.is(msp1BucketEvent) && 
                           msp1BucketEvent.data.bucketId;
      assert(msp1BucketId, "Failed to get MSP1 bucket ID");

      // Create MSP2 bucket
      const msp2BucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            msp2FileBucketName,
            true
          )
        ],
        signer: shUser
      });

      const msp2BucketId = userApi.events.fileSystem.NewBucket.is(msp2BucketEvent) && 
                           msp2BucketEvent.data.bucketId;
      assert(msp2BucketId, "Failed to get MSP2 bucket ID");

      // Create storage requests (files) in both buckets
      const msp1FileName = "msp1-test-file.txt";
      const msp2FileName = "msp2-test-file.txt";
      const fileSize = 1024;
      const fingerprint = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

      console.log(`\nCreating file in MSP1's bucket: ${msp1FileName}`);
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            msp1BucketId,
            msp1FileName,
            fingerprint,
            fileSize,
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      console.log(`Creating file in MSP2's bucket: ${msp2FileName}`);
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            msp2BucketId,
            msp2FileName,
            fingerprint,
            fileSize,
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(3000);

      // Query files in the database
      const allFiles = await sql`
        SELECT location, bucket_id, fingerprint, file_size
        FROM file
        WHERE location IN (${msp1FileName}, ${msp2FileName})
      `;

      console.log(`\nFiles found in database: ${allFiles.length}`);
      allFiles.forEach(file => {
        console.log(`  - File: ${file.location}`);
        console.log(`    Bucket: ${file.bucket_id}, Size: ${file.file_size}`);
      });

      // In lite mode, only files in MSP1's bucket should be indexed
      assert(
        allFiles.length === 1,
        `Expected only MSP1's file, found ${allFiles.length} files`
      );

      const indexedFile = allFiles[0];
      strictEqual(
        indexedFile.location,
        msp1FileName,
        "Only MSP1's file should be indexed"
      );
      strictEqual(
        indexedFile.bucket_id,
        msp1BucketId.toString(),
        "File should be in MSP1's bucket"
      );

      // Verify MSP2's file is NOT in the database
      const msp2FileCheck = await sql`
        SELECT COUNT(*) as count
        FROM file
        WHERE location = ${msp2FileName}
        OR bucket_id = ${msp2BucketId.toString()}
      `;

      strictEqual(
        parseInt(msp2FileCheck[0].count),
        0,
        "MSP2's file should not exist in the database"
      );

      // Check file count by bucket
      const filesByBucket = await sql`
        SELECT bucket_id, COUNT(*) as file_count
        FROM file
        WHERE bucket_id IN (${msp1BucketId.toString()}, ${msp2BucketId.toString()})
        GROUP BY bucket_id
      `;

      assert(
        filesByBucket.length === 1,
        "Should only have files in MSP1's bucket"
      );
      strictEqual(
        filesByBucket[0].bucket_id,
        msp1BucketId.toString(),
        "Files should only exist in MSP1's bucket"
      );
    });

    it("verifies ValueProp filtering for MSPs", async () => {
      // Add value propositions for both MSPs
      const msp1ValuePropId = `msp1-premium-${Date.now()}`;
      const msp2ValuePropId = `msp2-basic-${Date.now()}`;

      console.log("\nAdding ValueProp for MSP1...");
      await msp1Api.block.seal({
        calls: [
          msp1Api.tx.providers.addValueProp(
            100n, // price
            msp1ValuePropId
          )
        ],
        signer: msp1Api.signer
      });

      console.log("Adding ValueProp for MSP2...");
      await msp2Api.block.seal({
        calls: [
          msp2Api.tx.providers.addValueProp(
            50n, // price
            msp2ValuePropId
          )
        ],
        signer: msp2Api.signer
      });

      // Wait for indexing
      await sleep(3000);

      // Query MSPs with value props
      const mspsWithValueProps = await sql`
        SELECT onchain_msp_id, value_prop
        FROM msp
        WHERE value_prop IS NOT NULL
      `;

      console.log(`\nMSPs with value props: ${mspsWithValueProps.length}`);
      mspsWithValueProps.forEach(msp => {
        console.log(`  - MSP: ${msp.onchain_msp_id}`);
        console.log(`    ValueProps: ${JSON.stringify(msp.value_prop)}`);
      });

      // Only MSP1 should have value props in lite mode
      assert(
        mspsWithValueProps.length === 1,
        `Expected only MSP1 with value props, found ${mspsWithValueProps.length}`
      );

      const mspWithValueProp = mspsWithValueProps[0];
      strictEqual(
        mspWithValueProp.onchain_msp_id,
        userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "Only MSP1 should have value props"
      );

      // Verify the value prop contains MSP1's prop
      const valuePropData = mspWithValueProp.value_prop;
      assert(
        valuePropData && typeof valuePropData === 'object',
        "Value prop should be an object"
      );

      // Check that MSP1's value prop ID exists in the data
      const hasM1ValueProp = Object.values(valuePropData).some((prop: any) => 
        prop.id === msp1ValuePropId || prop.valuePropId === msp1ValuePropId
      );
      assert(hasM1ValueProp, "MSP1's value prop should be present");
    });

    it("verifies complete database state shows only MSP1 data", async () => {
      console.log("\n=== Final Database State Verification ===");

      // Check MSP table
      const mspCount = await sql`
        SELECT COUNT(*) as count FROM msp
      `;
      console.log(`Total MSPs in database: ${mspCount[0].count}`);

      const msps = await sql`
        SELECT onchain_msp_id FROM msp
      `;
      msps.forEach(msp => {
        console.log(`  - MSP: ${msp.onchain_msp_id}`);
      });

      // Check bucket table
      const bucketStats = await sql`
        SELECT 
          COUNT(*) as total_buckets,
          COUNT(DISTINCT msp_id) as distinct_msps
        FROM bucket
        WHERE msp_id IS NOT NULL
      `;
      console.log(`\nTotal buckets with MSPs: ${bucketStats[0].total_buckets}`);
      console.log(`Distinct MSPs owning buckets: ${bucketStats[0].distinct_msps}`);

      // List all MSPs that own buckets
      const bucketOwners = await sql`
        SELECT DISTINCT msp_id
        FROM bucket
        WHERE msp_id IS NOT NULL
      `;
      console.log("MSPs owning buckets:");
      bucketOwners.forEach(owner => {
        console.log(`  - ${owner.msp_id}`);
      });

      // Check file table
      const fileStats = await sql`
        SELECT 
          COUNT(*) as total_files,
          COUNT(DISTINCT b.msp_id) as distinct_msps
        FROM file f
        JOIN bucket b ON f.bucket_id = b.bucket_id
        WHERE b.msp_id IS NOT NULL
      `;
      console.log(`\nTotal files in MSP buckets: ${fileStats[0].total_files}`);
      console.log(`Distinct MSPs with files: ${fileStats[0].distinct_msps}`);

      // Final assertions
      strictEqual(
        parseInt(mspCount[0].count),
        1,
        "Should have exactly 1 MSP in lite mode"
      );

      strictEqual(
        msps[0].onchain_msp_id,
        userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "The only MSP should be MSP1"
      );

      if (parseInt(bucketStats[0].total_buckets) > 0) {
        strictEqual(
          parseInt(bucketStats[0].distinct_msps),
          1,
          "All buckets should belong to only one MSP"
        );

        strictEqual(
          bucketOwners[0].msp_id,
          userApi.shConsts.NODE_INFOS.msp1.AddressId,
          "All buckets should belong to MSP1"
        );
      }

      // Verify no MSP2 data exists anywhere
      const msp2DataCheck = await sql`
        SELECT 
          (SELECT COUNT(*) FROM msp WHERE onchain_msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}) as msp_count,
          (SELECT COUNT(*) FROM bucket WHERE msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}) as bucket_count,
          (SELECT COUNT(*) FROM file f JOIN bucket b ON f.bucket_id = b.bucket_id WHERE b.msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}) as file_count
      `;

      console.log("\nMSP2 data check:");
      console.log(`  MSP entries: ${msp2DataCheck[0].msp_count}`);
      console.log(`  Bucket entries: ${msp2DataCheck[0].bucket_count}`);
      console.log(`  File entries: ${msp2DataCheck[0].file_count}`);

      strictEqual(
        parseInt(msp2DataCheck[0].msp_count),
        0,
        "MSP2 should not exist in msp table"
      );
      strictEqual(
        parseInt(msp2DataCheck[0].bucket_count),
        0,
        "MSP2 should not have any buckets"
      );
      strictEqual(
        parseInt(msp2DataCheck[0].file_count),
        0,
        "MSP2 should not have any files"
      );

      console.log("\n✅ Lite mode filtering verified: Only MSP1 data is indexed");
    });
  }
);