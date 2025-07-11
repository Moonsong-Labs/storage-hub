import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, sleep } from "../../../util";

/**
 * Indexer Lite Mode Test Suite
 * 
 * This test verifies the core promise of lite mode: only MSP1's data is indexed.
 * It focuses on three key domain tables: MSP, Bucket, and File.
 */
describeMspNet(
  "Indexer Lite Mode - Domain Table Verification",
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

      // Wait for postgres to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      // Wait for MSP1's indexer to start (it runs the indexer in lite mode)
      await userApi.docker.waitForLog({
        containerName: "docker-sh-msp-1",
        searchString: "IndexerService starting up in",
        timeout: 10000
      });

      // Give indexer additional time to sync and process initial events
      // In lite mode, the indexer needs to sync from genesis to catch MSP registrations
      console.log("Waiting for indexer to sync initial blocks...");
      await sleep(10000);
      
      // Debug: Check service state to see what block the indexer is at
      const serviceState = await sql`SELECT * FROM service_state`;
      console.log("Service state:", serviceState);
      
      // Debug: Check MSP table directly
      const mspCheck = await sql`SELECT COUNT(*) as count FROM msp`;
      console.log(`Initial MSP count: ${mspCheck[0].count}`);
      
      // If no MSPs found, wait longer for indexer to catch up
      if (mspCheck[0].count === 0) {
        console.log("No MSPs found yet, waiting longer for indexer to catch up...");
        await sleep(5000);
        
        const retryMspCheck = await sql`SELECT COUNT(*) as count FROM msp`;
        console.log(`MSP count after retry: ${retryMspCheck[0].count}`);
      }
      
      // Also check the raw MSP table to see what's there
      const allMsps = await sql`SELECT * FROM msp`;
      console.log(`All MSPs in database:`, allMsps);
    });

    it("MSP table should contain only MSP1", async () => {
      // Query all MSPs in the database
      const msps = await sql`
        SELECT onchain_msp_id, capacity
        FROM msp
        ORDER BY onchain_msp_id
      `;

      console.log(`Found ${msps.length} MSP(s) in database`);
      
      // Verify only MSP1 exists
      const msp1Id = userApi.shConsts.NODE_INFOS.msp1.AddressId;
      const msp2Id = userApi.shConsts.NODE_INFOS.msp2.AddressId;
      const msp1OnchainId = userApi.shConsts.DUMMY_MSP_ID;
      const msp2OnchainId = userApi.shConsts.DUMMY_MSP_ID_2;
      
      console.log(`Looking for MSP1 ID: ${msp1Id} or onchain ID: ${msp1OnchainId}`);
      console.log(`Looking for MSP2 ID: ${msp2Id} or onchain ID: ${msp2OnchainId}`);
      
      if (msps.length > 0) {
        console.log(`MSPs in database:`);
        msps.forEach(m => {
          console.log(`  - ID: ${m.onchain_msp_id}, Capacity: ${m.capacity}`);
        });
      }
      
      const hasMsp1 = msps.some(m => 
        m.onchain_msp_id === msp1Id || m.onchain_msp_id === msp1OnchainId
      );
      const hasMsp2 = msps.some(m => 
        m.onchain_msp_id === msp2Id || m.onchain_msp_id === msp2OnchainId
      );
      
      assert(hasMsp1, "MSP1 should be indexed");
      assert(!hasMsp2, "MSP2 should NOT be indexed in lite mode");
      assert.strictEqual(msps.length, 1, "Exactly one MSP should be indexed");
      
      console.log("✓ MSP table contains only MSP1");
    });

    it("Bucket table should contain only MSP1's buckets", async () => {
      // Query all buckets with their MSP ownership
      const buckets = await sql`
        SELECT 
          b.name,
          b.private,
          m.onchain_msp_id as msp_id
        FROM bucket b
        JOIN msp m ON b.msp_id = m.id
        ORDER BY b.name
      `;

      console.log(`Found ${buckets.length} bucket(s) in database`);
      
      if (buckets.length > 0) {
        // Verify all buckets belong to MSP1
        const msp1Id = userApi.shConsts.NODE_INFOS.msp1.AddressId;
        const msp1OnchainId = userApi.shConsts.DUMMY_MSP_ID;
        const allBelongToMsp1 = buckets.every(b => 
          b.msp_id === msp1Id || b.msp_id === msp1OnchainId
        );
        
        assert(allBelongToMsp1, "All buckets should belong to MSP1");
        
        // Log bucket names for visibility
        buckets.forEach(b => {
          console.log(`  - Bucket: ${b.name} (private: ${b.private})`);
        });
        
        console.log("✓ Bucket table contains only MSP1's buckets");
      } else {
        console.log("✓ No buckets found (valid for initial state)");
      }
    });

    it("File table should contain only files in MSP1's buckets", async () => {
      // Query all files with their bucket and MSP ownership
      const files = await sql`
        SELECT 
          f.location as file_location,
          f.fingerprint,
          b.name as bucket_name,
          m.onchain_msp_id as msp_id
        FROM file f
        JOIN bucket b ON f.bucket_id = b.id
        JOIN msp m ON b.msp_id = m.id
        ORDER BY f.location
      `;

      console.log(`Found ${files.length} file(s) in database`);
      
      if (files.length > 0) {
        // Verify all files belong to MSP1's buckets
        const msp1Id = userApi.shConsts.NODE_INFOS.msp1.AddressId;
        const msp1OnchainId = userApi.shConsts.DUMMY_MSP_ID;
        const allBelongToMsp1 = files.every(f => 
          f.msp_id === msp1Id || f.msp_id === msp1OnchainId
        );
        
        assert(allBelongToMsp1, "All files should belong to MSP1's buckets");
        
        // Log file details for visibility
        files.forEach(f => {
          console.log(`  - File: ${f.file_location} in bucket ${f.bucket_name}`);
        });
        
        console.log("✓ File table contains only files in MSP1's buckets");
      } else {
        console.log("✓ No files found (valid for lite mode or initial state)");
      }
    });

    it("Summary: Lite mode filtering verification", async () => {
      // Get final counts for summary
      const summary = await sql`
        SELECT 
          (SELECT COUNT(*) FROM msp) as msp_count,
          (SELECT COUNT(*) FROM bucket) as bucket_count,
          (SELECT COUNT(*) FROM file) as file_count
      `;

      const counts = summary[0];
      
      console.log("\n=== LITE MODE VERIFICATION SUMMARY ===");
      console.log(`MSPs indexed: ${counts.msp_count} (expected: 1)`);
      console.log(`Buckets indexed: ${counts.bucket_count} (all should belong to MSP1)`);
      console.log(`Files indexed: ${counts.file_count} (all should be in MSP1's buckets)`);
      
      // Final verification
      assert.strictEqual(Number(counts.msp_count), 1, "Lite mode should index exactly one MSP");
      
      console.log("\n✅ Lite mode filtering is working correctly - only MSP1's data is indexed");
    });
  }
);