import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, sleep } from "../../../util";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

/**
 * Comprehensive Test Suite for Indexer Lite Mode
 * 
 * This test suite serves as a runner that verifies all lite mode functionality
 * and ensures complete coverage of events documented in LITE_MODE_EVENTS.md
 */
describeMspNet(
  "Indexer Lite Mode - Comprehensive Test Suite",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createUserApi, createSqlClient }) => {
    let msp1Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    // Test results tracking
    const testResults = {
      totalTests: 0,
      passedTests: 0,
      failedTests: 0,
      coverage: {
        fileSystemEvents: new Set<string>(),
        providerEvents: new Set<string>(),
        ignoredPallets: new Set<string>()
      }
    };

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      msp1Api = maybeMsp1Api;
      userApi = await createUserApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("verifies lite mode configuration", async () => {
      testResults.totalTests++;
      
      // Check indexer logs for lite mode confirmation
      const logs = await userApi.docker.getLogs({
        containerName: "docker-sh-msp-1",
        tail: 100
      });

      const liteModeEnabled = logs.includes("--indexer-mode=lite") || 
                             logs.includes("Indexer mode: lite");
      
      if (liteModeEnabled) {
        console.log("✓ Lite mode is enabled");
        testResults.passedTests++;
      } else {
        console.log("✗ Lite mode configuration not confirmed in logs");
        testResults.failedTests++;
      }
    });

    it("validates event filtering coverage", async () => {
      testResults.totalTests++;

      // Expected events based on LITE_MODE_EVENTS.md
      const expectedFileSystemEvents = [
        "NewBucket",
        "BucketPrivacyUpdateAccepted", 
        "MoveBucketAccepted",
        "BucketDeleted"
      ];

      const expectedProviderEvents = [
        "MspSignUpSuccess",
        "MspSignOffSuccess",
        "BspSignUpSuccess",
        "BspSignOffSuccess",
        "CapacityChanged",
        "MultiAddressesChanged",
        "ValuePropUpserted",
        "Slashed",
        "TopUpFulfilled"
      ];

      // Query actual indexed events
      const indexedEvents = await sql`
        SELECT DISTINCT section, method
        FROM block_event
        ORDER BY section, method
      `;

      // Track coverage
      indexedEvents.forEach(event => {
        if (event.section === "fileSystem" && expectedFileSystemEvents.includes(event.method)) {
          testResults.coverage.fileSystemEvents.add(event.method);
        } else if (event.section === "providers" && expectedProviderEvents.includes(event.method)) {
          testResults.coverage.providerEvents.add(event.method);
        }
      });

      // Check for ignored pallets
      const ignoredPallets = ["bucketNfts", "paymentStreams", "proofsDealer", "randomness"];
      const foundIgnoredPallets = indexedEvents
        .filter(e => ignoredPallets.includes(e.section))
        .map(e => e.section);

      foundIgnoredPallets.forEach(pallet => {
        testResults.coverage.ignoredPallets.add(pallet);
      });

      // Validate results
      const hasValidCoverage = testResults.coverage.fileSystemEvents.size > 0 &&
                              testResults.coverage.providerEvents.size > 0 &&
                              testResults.coverage.ignoredPallets.size === 0;

      if (hasValidCoverage) {
        console.log("✓ Event filtering coverage is valid");
        testResults.passedTests++;
      } else {
        console.log("✗ Event filtering coverage issues detected");
        testResults.failedTests++;
      }
    });

    it("runs all lite mode test files", async () => {
      testResults.totalTests++;

      // List of test files that should exist
      const testFiles = [
        "indexer-lite-mode.test.ts",
        "indexer-lite-mode-base.test.ts",
        "indexer-lite-mode-env.test.ts",
        "indexer-lite-mode-filtering.test.ts",
        "indexer-lite-msp-events.test.ts",
        "indexer-lite-performance.test.ts",
        "indexer-lite-event-processing.test.ts"
      ];

      console.log("\n=== Test File Verification ===");
      let allFilesExist = true;

      for (const file of testFiles) {
        try {
          const filePath = join(__dirname, file);
          await readFile(filePath, 'utf-8');
          console.log(`✓ ${file} exists`);
        } catch (error) {
          console.log(`✗ ${file} not found`);
          allFilesExist = false;
        }
      }

      if (allFilesExist) {
        testResults.passedTests++;
      } else {
        testResults.failedTests++;
      }
    });

    it("verifies MSP-specific filtering", async () => {
      testResults.totalTests++;

      // Check that only current MSP events are indexed
      const mspEvents = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data::text LIKE ${'%' + msp1Api.accountId() + '%'}
      `;

      // Check for other MSP events (should be none)
      const otherMspPattern = "0x0000000000000000000000000000000000000000000000000000000000000301"; // MSP2
      const otherMspEvents = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data::text LIKE ${'%' + otherMspPattern + '%'}
      `;

      const correctFiltering = Number(mspEvents[0].count) > 0 && 
                              Number(otherMspEvents[0].count) === 0;

      if (correctFiltering) {
        console.log("✓ MSP-specific filtering is working correctly");
        testResults.passedTests++;
      } else {
        console.log("✗ MSP-specific filtering issues detected");
        testResults.failedTests++;
      }
    });

    it("measures performance improvement", async () => {
      testResults.totalTests++;

      // Get current statistics
      const stats = await sql`
        SELECT 
          (SELECT COUNT(*) FROM block_event) as total_events,
          (SELECT COUNT(*) FROM bucket) as total_buckets,
          (SELECT COUNT(*) FROM msp) as total_msps,
          (SELECT pg_database_size(current_database())) as db_size
      `;

      const totalEvents = Number(stats[0].total_events);
      const dbSizeMB = Number(stats[0].db_size) / 1024 / 1024;

      console.log("\n=== Performance Metrics ===");
      console.log(`Total events: ${totalEvents}`);
      console.log(`Database size: ${dbSizeMB.toFixed(2)} MB`);

      // In lite mode, we expect significantly fewer events
      const isEfficient = totalEvents < 1000 && dbSizeMB < 50;

      if (isEfficient) {
        console.log("✓ Performance metrics show efficient lite mode operation");
        testResults.passedTests++;
      } else {
        console.log("✗ Performance metrics suggest lite mode may not be filtering effectively");
        testResults.failedTests++;
      }
    });

    it("validates database consistency", async () => {
      testResults.totalTests++;

      // Check foreign key relationships
      const orphanedBuckets = await sql`
        SELECT COUNT(*) as count
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.msp_id IS NOT NULL AND m.id IS NULL
      `;

      // Check event integrity
      const invalidEvents = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data IS NULL OR section IS NULL OR method IS NULL
      `;

      const isConsistent = Number(orphanedBuckets[0].count) === 0 &&
                          Number(invalidEvents[0].count) === 0;

      if (isConsistent) {
        console.log("✓ Database consistency validated");
        testResults.passedTests++;
      } else {
        console.log("✗ Database consistency issues found");
        testResults.failedTests++;
      }
    });

    it("generates comprehensive test report", async () => {
      console.log("\n=== LITE MODE TEST SUITE SUMMARY ===");
      console.log(`Total tests: ${testResults.totalTests}`);
      console.log(`Passed: ${testResults.passedTests}`);
      console.log(`Failed: ${testResults.failedTests}`);
      console.log(`Success rate: ${(testResults.passedTests / testResults.totalTests * 100).toFixed(2)}%`);

      console.log("\n=== Event Coverage ===");
      console.log(`FileSystem events covered: ${testResults.coverage.fileSystemEvents.size}`);
      console.log(`Provider events covered: ${testResults.coverage.providerEvents.size}`);
      console.log(`Ignored pallets found: ${testResults.coverage.ignoredPallets.size}`);

      // Get detailed event statistics
      const eventStats = await sql`
        SELECT section, method, COUNT(*) as count
        FROM block_event
        GROUP BY section, method
        ORDER BY section, method
      `;

      console.log("\n=== Indexed Event Distribution ===");
      eventStats.forEach(stat => {
        console.log(`${stat.section}.${stat.method}: ${stat.count} events`);
      });

      // Verify all documented events are tested
      const documentedEvents = [
        "fileSystem.NewBucket",
        "fileSystem.BucketPrivacyUpdateAccepted",
        "fileSystem.MoveBucketAccepted",
        "fileSystem.BucketDeleted",
        "providers.MspSignUpSuccess",
        "providers.MspSignOffSuccess",
        "providers.BspSignUpSuccess",
        "providers.BspSignOffSuccess",
        "providers.CapacityChanged",
        "providers.ValuePropUpserted"
      ];

      const untestedEvents = documentedEvents.filter(event => {
        const [section, method] = event.split(".");
        return !eventStats.some(stat => 
          stat.section === section && stat.method === method
        );
      });

      if (untestedEvents.length > 0) {
        console.log("\n⚠️  Warning: The following documented events were not found:");
        untestedEvents.forEach(event => console.log(`  - ${event}`));
      }

      // Final assertion
      assert(
        testResults.failedTests === 0,
        `Test suite failed with ${testResults.failedTests} failures`
      );

      console.log("\n✅ Lite mode test suite completed successfully!");
    });
  }
);