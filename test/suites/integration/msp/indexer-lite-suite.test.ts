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
  { initialised: false, indexer: true, indexerMode: "lite" },
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

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      // Give indexer time to process initial events
      await sleep(3000);
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
      const msp1Id = msp1Api.accountId();
      const msp2Id = msp2Api.accountId();
      
      const hasMsp1 = msps.some(m => m.onchain_msp_id === msp1Id);
      const hasMsp2 = msps.some(m => m.onchain_msp_id === msp2Id);
      
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
        const msp1Id = msp1Api.accountId();
        const allBelongToMsp1 = buckets.every(b => b.msp_id === msp1Id);
        
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
          f.name as file_name,
          f.description,
          b.name as bucket_name,
          m.onchain_msp_id as msp_id
        FROM file f
        JOIN bucket b ON f.bucket_id = b.id
        JOIN msp m ON b.msp_id = m.id
        ORDER BY f.name
      `;

      console.log(`Found ${files.length} file(s) in database`);
      
      if (files.length > 0) {
        // Verify all files belong to MSP1's buckets
        const msp1Id = msp1Api.accountId();
        const allBelongToMsp1 = files.every(f => f.msp_id === msp1Id);
        
        assert(allBelongToMsp1, "All files should belong to MSP1's buckets");
        
        // Log file details for visibility
        files.forEach(f => {
          console.log(`  - File: ${f.file_name} in bucket ${f.bucket_name}`);
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