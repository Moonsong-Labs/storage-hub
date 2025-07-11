import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * This test suite verifies MSP-specific filtering in lite mode.
 * In lite mode, only MSP1 runs the indexer and should only index its own data.
 * 
 * The test focuses on verifying that:
 * - Only MSP1's value propositions are indexed in the msp table
 * - MSP2's data is NOT indexed when running in lite mode
 * - Domain tables are used for verification (not event tables)
 */
describeMspNet(
  "Indexer Lite Mode - MSP Filtering",
  { initialised: true, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      msp1Api = await createMsp1Api();
      msp2Api = await createMsp2Api();
      userApi = await createUserApi();
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

    it("verifies only MSP1 data is indexed in lite mode", async () => {
      console.log("\n=== Testing MSP-specific filtering ===");
      
      // Get MSP addresses
      const msp1Address = msp1Api.ss58.storageHub(msp1Api.keyringPair.address);
      const msp2Address = msp2Api.ss58.storageHub(msp2Api.keyringPair.address);
      
      console.log(`MSP1 address: ${msp1Address}`);
      console.log(`MSP2 address: ${msp2Address}`);
      
      // Create value propositions for both MSPs
      console.log("\nCreating value propositions for both MSPs...");
      
      // MSP1 adds value prop
      await msp1Api.block.seal({
        calls: [msp1Api.tx.providers.addValueProp(100n, "msp1-service")],
        signer: msp1Api.keyringPair
      });
      
      // MSP2 adds value prop
      await msp2Api.block.seal({
        calls: [msp2Api.tx.providers.addValueProp(200n, "msp2-service")],
        signer: msp2Api.keyringPair
      });
      
      // Wait for indexing
      await sleep(5000);
      
      // Check MSP table - in lite mode, only MSP1 should be indexed
      const msps = await sql`
        SELECT onchain_msp_id, value_prop 
        FROM msp 
        WHERE value_prop IS NOT NULL
        ORDER BY value_prop
      `;
      
      console.log(`\nMSPs with value props in database: ${msps.length}`);
      msps.forEach(m => {
        console.log(`  - MSP ${m.onchain_msp_id}: ${m.value_prop}`);
      });
      
      // In lite mode, only MSP1's data should be indexed
      assert(msps.length === 1, "Should only have one MSP with value prop in lite mode");
      assert(msps[0].onchain_msp_id === msp1Address, "Should be MSP1's address");
      assert(msps[0].value_prop === "msp1-service", "Should be MSP1's value prop");
      
      // Verify MSP2's value prop is NOT indexed
      const msp2Check = await sql`
        SELECT * FROM msp 
        WHERE onchain_msp_id = ${msp2Address} 
        AND value_prop IS NOT NULL
      `;
      assert(msp2Check.length === 0, "MSP2's value prop should NOT be indexed in lite mode");
      
      console.log("\n✅ MSP filtering verified: Only MSP1 data is indexed");
    });
    
    it("verifies MSP1 bucket operations are indexed", async () => {
      console.log("\n=== Testing MSP1 bucket indexing ===");
      
      // Create a bucket as MSP1's user
      const bucketName = "msp1-bucket-test";
      
      // Count buckets before
      const beforeCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      console.log(`Buckets before: ${beforeCount[0].count}`);
      
      // Create bucket
      const newBucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to create bucket");
      
      await sleep(3000);
      
      // Count buckets after
      const afterCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      console.log(`Buckets after: ${afterCount[0].count}`);
      
      // Verify bucket was indexed
      const bucket = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
      assert(bucket.length === 1, "Bucket should be indexed");
      assert(bucket[0].name === bucketName, "Bucket name should match");
      
      // Verify bucket belongs to MSP1 user
      const bucketWithUser = await sql`
        SELECT b.*, u.msp_id, m.onchain_msp_id
        FROM bucket b
        JOIN "user" u ON b.owner_id = u.id
        JOIN msp m ON u.msp_id = m.id
        WHERE b.name = ${bucketName}
      `;
      
      assert(bucketWithUser.length === 1, "Bucket should have associated user and MSP");
      const msp1Address = msp1Api.ss58.storageHub(msp1Api.keyringPair.address);
      assert(bucketWithUser[0].onchain_msp_id === msp1Address, "Bucket should belong to MSP1 user");
      
      console.log("\n✅ MSP1 bucket operations are properly indexed");
    });
    
    it("verifies all MSPs are present but only MSP1 has indexed operations", async () => {
      console.log("\n=== Verifying MSP presence and operation filtering ===");
      
      // All MSPs should be present in the database (for provider relationships)
      const allMsps = await sql`
        SELECT id, onchain_msp_id, multiaddress 
        FROM msp 
        ORDER BY id
      `;
      
      console.log(`\nTotal MSPs in database: ${allMsps.length}`);
      assert(allMsps.length >= 2, "Should have at least 2 MSPs (MSP1 and MSP2)");
      
      // Check MSP1 exists
      const msp1 = allMsps.find(m => m.multiaddress && m.multiaddress.includes('5001'));
      assert(msp1, "MSP1 (port 5001) should exist");
      
      // Check MSP2 exists
      const msp2 = allMsps.find(m => m.multiaddress && m.multiaddress.includes('5002'));
      assert(msp2, "MSP2 (port 5002) should exist");
      
      // But only MSP1 should have indexed operations (like value props)
      const mspsWithValueProps = await sql`
        SELECT id, onchain_msp_id, value_prop 
        FROM msp 
        WHERE value_prop IS NOT NULL
      `;
      
      console.log(`\nMSPs with value props: ${mspsWithValueProps.length}`);
      assert(mspsWithValueProps.length <= 1, "At most one MSP should have value props in lite mode");
      
      if (mspsWithValueProps.length === 1) {
        const msp1Address = msp1Api.ss58.storageHub(msp1Api.keyringPair.address);
        assert(
          mspsWithValueProps[0].onchain_msp_id === msp1Address,
          "Only MSP1 should have indexed operations"
        );
      }
      
      console.log("\n✅ All MSPs present, but only MSP1 has indexed operations");
    });
    
    it("verifies BSP data is indexed for all BSPs", async () => {
      console.log("\n=== Verifying BSP indexing ===");
      
      // All BSPs should be indexed (needed for provider relationships)
      const bsps = await sql`
        SELECT id, onchain_bsp_id, multiaddress 
        FROM bsp 
        ORDER BY id
      `;
      
      console.log(`\nTotal BSPs in database: ${bsps.length}`);
      assert(bsps.length >= 1, "Should have at least 1 BSP");
      
      // Verify BSP has required fields
      bsps.forEach((bsp, index) => {
        assert(bsp.id, `BSP ${index} should have ID`);
        assert(bsp.onchain_bsp_id, `BSP ${index} should have onchain ID`);
        assert(bsp.multiaddress, `BSP ${index} should have multiaddress`);
      });
      
      console.log("\n✅ All BSPs are properly indexed");
    });
    
    it("provides lite mode filtering summary", async () => {
      console.log("\n=== LITE MODE FILTERING SUMMARY ===");
      
      // Get counts from all relevant tables
      const mspCount = await sql`SELECT COUNT(*) as count FROM msp`;
      const bspCount = await sql`SELECT COUNT(*) as count FROM bsp`;
      const userCount = await sql`SELECT COUNT(*) as count FROM "user"`;
      const bucketCount = await sql`SELECT COUNT(*) as count FROM bucket`;
      const fileCount = await sql`SELECT COUNT(*) as count FROM file`;
      
      // Get MSP1-specific counts
      const msp1Address = msp1Api.ss58.storageHub(msp1Api.keyringPair.address);
      const msp1Data = await sql`
        SELECT 
          (SELECT COUNT(*) FROM msp WHERE onchain_msp_id = ${msp1Address} AND value_prop IS NOT NULL) as msp_with_props,
          (SELECT COUNT(*) FROM "user" u JOIN msp m ON u.msp_id = m.id WHERE m.onchain_msp_id = ${msp1Address}) as users,
          (SELECT COUNT(*) FROM bucket b JOIN "user" u ON b.owner_id = u.id JOIN msp m ON u.msp_id = m.id WHERE m.onchain_msp_id = ${msp1Address}) as buckets
      `;
      
      console.log("\nProvider tables (all providers indexed):");
      console.log(`  - MSPs: ${mspCount[0].count}`);
      console.log(`  - BSPs: ${bspCount[0].count}`);
      
      console.log("\nMSP1-specific data (filtered in lite mode):");
      console.log(`  - MSP1 with value props: ${msp1Data[0].msp_with_props}`);
      console.log(`  - Users on MSP1: ${msp1Data[0].users}`);
      console.log(`  - Buckets owned by MSP1 users: ${msp1Data[0].buckets}`);
      
      console.log("\nOther tables:");
      console.log(`  - Total users: ${userCount[0].count}`);
      console.log(`  - Total buckets: ${bucketCount[0].count}`);
      console.log(`  - Files: ${fileCount[0].count} (should be 0 in lite mode)`);
      
      // Verify lite mode constraints
      assert(
        parseInt(fileCount[0].count) === 0,
        "File table should be empty in lite mode"
      );
      
      console.log("\n✅ Lite mode filtering is working correctly");
    });
  }
);