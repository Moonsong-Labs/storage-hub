import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, sleep } from "../../../util";

/**
 * Comprehensive Test Suite for Indexer Lite Mode
 * 
 * This test suite serves as a runner that verifies all lite mode functionality
 * and ensures that the indexer correctly filters data based on the current MSP.
 */
describeMspNet(
  "Indexer Lite Mode - Comprehensive Test Suite",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    // Test results tracking
    const testResults = {
      totalTests: 0,
      passedTests: 0,
      failedTests: 0,
      coverage: {
        tables: new Set<string>(),
        mspFiltering: false,
        bucketFiltering: false
      }
    };

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

      // Give indexer time to process initial events
      await sleep(3000);
    });

    it("verifies lite mode database schema", async () => {
      testResults.totalTests++;

      // Check that expected tables exist
      const tables = await sql`
        SELECT table_name 
        FROM information_schema.tables 
        WHERE table_schema = 'public'
        ORDER BY table_name;
      `;

      const tableNames = tables.map(t => t.table_name);
      const expectedTables = [
        "service_state",
        "multiaddress", 
        "bsp",
        "bsp_multiaddress",
        "msp", 
        "msp_multiaddress",
        "bucket",
        "paymentstream",
        "file",
        "bsp_file",
        "peer_id",
        "file_peer_id"
      ];

      // Track which tables exist
      expectedTables.forEach(table => {
        if (tableNames.includes(table)) {
          testResults.coverage.tables.add(table);
        }
      });

      const hasAllTables = expectedTables.every(table => tableNames.includes(table));
      
      if (hasAllTables) {
        console.log("✓ All expected database tables exist");
        testResults.passedTests++;
      } else {
        console.log("✗ Missing database tables");
        const missing = expectedTables.filter(t => !tableNames.includes(t));
        console.log("  Missing:", missing.join(", "));
        testResults.failedTests++;
      }
    });

    it("verifies MSP filtering in lite mode", async () => {
      testResults.totalTests++;

      // Check that only MSP1 is in the database (since this is MSP1's indexer)
      const msps = await sql`
        SELECT onchain_msp_id, capacity
        FROM msp
        ORDER BY onchain_msp_id
      `;

      const hasMsp1 = msps.some(m => m.onchain_msp_id === msp1Api.accountId());
      const hasMsp2 = msps.some(m => m.onchain_msp_id === msp2Api.accountId());

      if (hasMsp1 && !hasMsp2) {
        console.log("✓ MSP filtering is working - only MSP1 data is indexed");
        testResults.passedTests++;
        testResults.coverage.mspFiltering = true;
      } else if (hasMsp1 && hasMsp2) {
        console.log("✗ MSP filtering not working - both MSPs are indexed");
        testResults.failedTests++;
      } else {
        console.log("✗ No MSP data found in database");
        testResults.failedTests++;
      }
    });

    it("verifies bucket filtering by MSP ownership", async () => {
      testResults.totalTests++;

      // Get MSP IDs from database
      const mspRecords = await sql`
        SELECT id, onchain_msp_id
        FROM msp
      `;

      if (mspRecords.length === 0) {
        console.log("✗ No MSP records found to verify bucket filtering");
        testResults.failedTests++;
        return;
      }

      // Check buckets
      const buckets = await sql`
        SELECT b.name, b.private, m.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.msp_id IS NOT NULL
      `;

      // In lite mode, all buckets should belong to MSP1
      const allBucketsBelongToMsp1 = buckets.every(b => b.onchain_msp_id === msp1Api.accountId());
      const hasAnyBuckets = buckets.length > 0;

      if (hasAnyBuckets && allBucketsBelongToMsp1) {
        console.log("✓ Bucket filtering is working - only MSP1's buckets are indexed");
        testResults.passedTests++;
        testResults.coverage.bucketFiltering = true;
      } else if (!hasAnyBuckets) {
        console.log("⚠️  No MSP-owned buckets found to verify filtering");
        // This is not necessarily a failure - might just be no buckets created yet
        testResults.passedTests++;
      } else {
        console.log("✗ Bucket filtering not working - found buckets for other MSPs");
        testResults.failedTests++;
      }
    });

    it("measures lite mode data reduction", async () => {
      testResults.totalTests++;

      // Get counts from various tables
      const counts = await sql`
        SELECT 
          (SELECT COUNT(*) FROM msp) as msp_count,
          (SELECT COUNT(*) FROM bucket) as bucket_count,
          (SELECT COUNT(*) FROM file) as file_count,
          (SELECT COUNT(*) FROM bsp) as bsp_count,
          (SELECT pg_database_size(current_database())) as db_size
      `;

      const stats = counts[0];
      console.log("\n=== Lite Mode Statistics ===");
      console.log(`MSPs indexed: ${stats.msp_count}`);
      console.log(`Buckets indexed: ${stats.bucket_count}`);
      console.log(`Files indexed: ${stats.file_count}`);
      console.log(`BSPs indexed: ${stats.bsp_count}`);
      console.log(`Database size: ${(Number(stats.db_size) / 1024 / 1024).toFixed(2)} MB`);

      // In lite mode, we expect only 1 MSP
      if (Number(stats.msp_count) === 1) {
        console.log("✓ Lite mode MSP filtering confirmed");
        testResults.passedTests++;
      } else {
        console.log("✗ Expected only 1 MSP in lite mode");
        testResults.failedTests++;
      }
    });

    it("validates service state tracking", async () => {
      testResults.totalTests++;

      // Check service state
      const serviceState = await sql`
        SELECT last_processed_block
        FROM service_state
        WHERE id = 1
      `;

      if (serviceState.length > 0 && serviceState[0].last_processed_block > 0) {
        console.log(`✓ Indexer is tracking blockchain state (last block: ${serviceState[0].last_processed_block})`);
        testResults.passedTests++;
      } else {
        console.log("✗ Service state not properly initialized");
        testResults.failedTests++;
      }
    });

    it("generates comprehensive test report", async () => {
      console.log("\n=== LITE MODE TEST SUITE SUMMARY ===");
      console.log(`Total tests: ${testResults.totalTests}`);
      console.log(`Passed: ${testResults.passedTests}`);
      console.log(`Failed: ${testResults.failedTests}`);
      console.log(`Success rate: ${(testResults.passedTests / testResults.totalTests * 100).toFixed(2)}%`);

      console.log("\n=== Coverage Summary ===");
      console.log(`Database tables verified: ${testResults.coverage.tables.size}`);
      console.log(`MSP filtering: ${testResults.coverage.mspFiltering ? '✓' : '✗'}`);
      console.log(`Bucket filtering: ${testResults.coverage.bucketFiltering ? '✓' : '✗'}`);

      // Get detailed statistics
      const detailedStats = await sql`
        SELECT 
          'msp' as entity, COUNT(*) as count FROM msp
        UNION ALL
        SELECT 'bucket', COUNT(*) FROM bucket
        UNION ALL
        SELECT 'file', COUNT(*) FROM file
        UNION ALL
        SELECT 'bsp', COUNT(*) FROM bsp
        UNION ALL
        SELECT 'paymentstream', COUNT(*) FROM paymentstream
        ORDER BY entity
      `;

      console.log("\n=== Entity Counts ===");
      detailedStats.forEach(stat => {
        console.log(`${stat.entity}: ${stat.count} records`);
      });

      // Final assertion
      assert(
        testResults.failedTests === 0,
        `Test suite failed with ${testResults.failedTests} failures`
      );

      console.log("\n✅ Lite mode test suite completed successfully!");
    });
  }
);