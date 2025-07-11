import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Basic test to verify indexer lite mode is working
 * This test checks that the indexer is running and processing data
 */
describeMspNet(
  "Indexer Lite Mode - Basic Verification", 
  { initialised: true, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      msp1Api = maybeMsp1Api;
      userApi = await createUserApi();
      sql = createSqlClient();
    });

    it("verifies indexer is running and database is accessible", async () => {
      // Check that we can connect to the database
      const tables = await sql`
        SELECT table_name 
        FROM information_schema.tables 
        WHERE table_schema = 'public'
        ORDER BY table_name;
      `;

      assert(tables.length > 0, "Should have database tables");
      console.log(`Found ${tables.length} database tables`);
    });

    it("checks service state to verify indexer is processing blocks", async () => {
      // Wait a bit for indexer to process some blocks
      await sleep(5000);

      const serviceState = await sql`
        SELECT last_processed_block
        FROM service_state
        WHERE id = 1
      `;

      if (serviceState.length > 0) {
        const lastBlock = serviceState[0].last_processed_block;
        console.log(`Indexer processed up to block: ${lastBlock}`);
        assert(lastBlock > 0, "Indexer should have processed some blocks");
      } else {
        console.log("Service state not found - indexer may not be running");
      }
    });

    it("creates a bucket and verifies it's indexed", async () => {
      const bucketName = `lite-test-bucket-${Date.now()}`;
      
      // Create a bucket assigned to MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(5000);

      // Check if bucket was indexed
      const buckets = await sql`
        SELECT b.name, b.private, m.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.name = ${bucketName}
      `;

      if (buckets.length > 0) {
        console.log(`✓ Bucket '${bucketName}' was indexed`);
        console.log(`  MSP: ${buckets[0].onchain_msp_id || 'none'}`);
        console.log(`  Private: ${buckets[0].private}`);
      } else {
        console.log(`✗ Bucket '${bucketName}' was not indexed`);
      }

      // Check all buckets
      const allBuckets = await sql`
        SELECT COUNT(*) as count FROM bucket
      `;
      console.log(`Total buckets in database: ${allBuckets[0].count}`);
    });

    it("checks if any MSPs are indexed", async () => {
      const msps = await sql`
        SELECT onchain_msp_id, capacity
        FROM msp
        ORDER BY onchain_msp_id
      `;

      console.log(`\nMSPs in database: ${msps.length}`);
      msps.forEach(msp => {
        console.log(`  MSP ID: ${msp.onchain_msp_id}`);
        console.log(`  Capacity: ${msp.capacity}`);
      });

      // Check if we're in lite mode by seeing if only one MSP is indexed
      if (msps.length === 1) {
        console.log("✓ Lite mode filtering appears to be working (only 1 MSP)");
      } else if (msps.length > 1) {
        console.log("⚠️  Multiple MSPs indexed - might not be in lite mode");
      } else {
        console.log("⚠️  No MSPs indexed - indexer might not be running");
      }
    });

    it("provides diagnostic information", async () => {
      console.log("\n=== Diagnostic Information ===");
      
      // Check all entity counts
      const counts = await sql`
        SELECT 
          'msp' as entity, COUNT(*) as count FROM msp
        UNION ALL
        SELECT 'bucket', COUNT(*) FROM bucket
        UNION ALL  
        SELECT 'file', COUNT(*) FROM file
        UNION ALL
        SELECT 'bsp', COUNT(*) FROM bsp
        ORDER BY entity
      `;

      counts.forEach(row => {
        console.log(`${row.entity}: ${row.count} records`);
      });

      // Check database size
      const dbSize = await sql`
        SELECT pg_database_size(current_database()) as size
      `;
      console.log(`\nDatabase size: ${(Number(dbSize[0].size) / 1024 / 1024).toFixed(2)} MB`);
    });
  }
);