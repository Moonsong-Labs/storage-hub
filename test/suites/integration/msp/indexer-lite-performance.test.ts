import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Enhanced Indexer Lite Mode - Performance Tests
 * 
 * This test verifies that the enhanced lite mode maintains acceptable performance
 * when indexing all buckets, files, and BSP events.
 */
describeMspNet(
  "Indexer Lite Mode - Performance Verification",
  { initialised: true, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient, createBspApi }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      await userApi.docker.waitForLog({
        containerName: "docker-sh-msp-1",
        searchString: "IndexerService starting up in",
        timeout: 10000
      });

      // Give indexer time to sync
      await sleep(5000);
    });

    it("handles high volume of bucket and file creation efficiently", async () => {
      const startTime = Date.now();
      const numBuckets = 10;
      const filesPerBucket = 10;
      const bucketIds: string[] = [];

      // Create multiple buckets across different MSPs
      for (let i = 0; i < numBuckets; i++) {
        const mspId = i % 2 === 0 
          ? userApi.shConsts.NODE_INFOS.msp1.AddressId 
          : userApi.shConsts.NODE_INFOS.msp2.AddressId;
        
        const bucketEvent = await userApi.block.seal({
          calls: [
            userApi.tx.fileSystem.createBucket(
              mspId,
              `perf-bucket-${i}`,
              true
            )
          ],
          signer: shUser
        });

        const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
        if (bucketId) {
          bucketIds.push(bucketId.toString());
        }
      }

      // Add files to each bucket
      for (let i = 0; i < bucketIds.length; i++) {
        const bucketId = bucketIds[i];
        const calls = [];

        for (let j = 0; j < filesPerBucket; j++) {
          const fileIndex = i * filesPerBucket + j;
          calls.push(
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              `file-${fileIndex}.dat`,
              `0x${fileIndex.toString(16).padStart(64, '0')}`,
              1024 + j * 100,
              i % 2 === 0 
                ? userApi.shConsts.NODE_INFOS.msp1.AddressId 
                : userApi.shConsts.NODE_INFOS.msp2.AddressId,
              [userApi.alice.publicKey],
              null
            )
          );
        }

        // Batch file creation
        await userApi.block.seal({
          calls,
          signer: shUser
        });
      }

      // Wait for indexing to complete
      const indexingWaitTime = 5000;
      await sleep(indexingWaitTime);

      const totalTime = Date.now() - startTime;

      // Verify all data was indexed
      const counts = await sql`
        SELECT 
          (SELECT COUNT(*) FROM bucket WHERE name LIKE 'perf-bucket-%') as bucket_count,
          (SELECT COUNT(*) FROM file WHERE location LIKE 'file-%.dat') as file_count,
          (SELECT COUNT(*) FROM block_event WHERE section = 'fileSystem') as event_count
      `;

      const result = counts[0];
      console.log("\nPerformance Test Results:");
      console.log(`Total time: ${totalTime}ms`);
      console.log(`Buckets indexed: ${result.bucket_count}/${numBuckets}`);
      console.log(`Files indexed: ${result.file_count}/${numBuckets * filesPerBucket}`);
      console.log(`FileSystem events: ${result.event_count}`);
      console.log(`Average time per bucket: ${Math.round(totalTime / numBuckets)}ms`);

      // Verify counts
      assert(
        Number(result.bucket_count) === numBuckets,
        `Should have indexed all ${numBuckets} buckets`
      );
      assert(
        Number(result.file_count) === numBuckets * filesPerBucket,
        `Should have indexed all ${numBuckets * filesPerBucket} files`
      );

      // Performance threshold - should complete within reasonable time
      // Adjust based on your requirements
      const maxAcceptableTime = 30000; // 30 seconds for 100 files
      assert(
        totalTime < maxAcceptableTime,
        `Indexing should complete within ${maxAcceptableTime}ms, took ${totalTime}ms`
      );
    });

    it("maintains query performance with large datasets", async () => {
      // Test various query patterns
      const queries = [
        {
          name: "List all buckets",
          query: sql`SELECT COUNT(*) as count FROM bucket`
        },
        {
          name: "Find files by bucket",
          query: sql`
            SELECT COUNT(*) as count 
            FROM file f 
            JOIN bucket b ON f.bucket_id = b.id 
            WHERE b.name LIKE 'perf-bucket-%'
          `
        },
        {
          name: "BSP file associations",
          query: sql`
            SELECT COUNT(*) as count 
            FROM bsp_file bf 
            JOIN file f ON bf.file_id = f.id
          `
        },
        {
          name: "Recent events",
          query: sql`
            SELECT COUNT(*) as count 
            FROM block_event 
            WHERE section = 'fileSystem' 
            ORDER BY block_number DESC 
            LIMIT 100
          `
        }
      ];

      console.log("\nQuery Performance Test:");
      for (const test of queries) {
        const startTime = Date.now();
        const result = await test.query;
        const queryTime = Date.now() - startTime;
        
        console.log(`${test.name}: ${queryTime}ms (${result[0].count} rows)`);
        
        // Query should complete quickly
        assert(
          queryTime < 1000,
          `Query "${test.name}" should complete within 1000ms, took ${queryTime}ms`
        );
      }
    });

    it("handles concurrent BSP volunteering efficiently", async () => {
      // Create a bucket with files for BSP testing
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            "bsp-perf-bucket",
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Create multiple files
      const numFiles = 20;
      const fileKeys = [];

      for (let i = 0; i < numFiles; i += 5) {
        const calls = [];
        for (let j = 0; j < 5 && i + j < numFiles; j++) {
          const fileIndex = i + j;
          calls.push(
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              `bsp-file-${fileIndex}.dat`,
              `0x${(9000 + fileIndex).toString(16).padStart(64, '0')}`,
              2048,
              userApi.shConsts.NODE_INFOS.msp1.AddressId,
              [userApi.alice.publicKey],
              null
            )
          );
        }

        const event = await userApi.block.seal({
          calls,
          signer: shUser
        });

        // Extract file keys from events
        userApi.events.fileSystem.NewStorageRequest.buildV3Tuples(event).forEach(([eventData]) => {
          fileKeys.push(eventData.fileKey);
        });
      }

      await sleep(2000);

      // BSP volunteers for multiple files
      const bspStartTime = Date.now();
      
      // Volunteer for files in batches
      for (let i = 0; i < fileKeys.length; i += 5) {
        const calls = [];
        for (let j = 0; j < 5 && i + j < fileKeys.length; j++) {
          calls.push(bspApi.tx.fileSystem.bspVolunteer(fileKeys[i + j]));
        }
        
        await bspApi.block.seal({
          calls,
          signer: bspApi.signer
        });
      }

      await sleep(3000);
      const bspTime = Date.now() - bspStartTime;

      // Verify BSP events were indexed
      const bspEvents = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'AcceptedBspVolunteer'
      `;

      console.log("\nBSP Performance Test:");
      console.log(`BSP volunteering time: ${bspTime}ms`);
      console.log(`AcceptedBspVolunteer events: ${bspEvents[0].count}`);
      console.log(`Average time per BSP volunteer: ${Math.round(bspTime / numFiles)}ms`);

      // Should handle BSP volunteering efficiently
      assert(
        bspTime < 20000,
        `BSP volunteering should complete within 20s, took ${bspTime}ms`
      );
    });

    it("monitors indexer resource usage", async () => {
      // Check database connection count
      const connections = await sql`
        SELECT COUNT(*) as count
        FROM pg_stat_activity
        WHERE datname = current_database()
      `;

      console.log("\nResource Usage:");
      console.log(`Active DB connections: ${connections[0].count}`);

      // Check table sizes
      const tableSizes = await sql`
        SELECT 
          schemaname,
          tablename,
          pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename IN ('bucket', 'file', 'bsp_file', 'block_event')
        ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
      `;

      console.log("\nTable Sizes:");
      tableSizes.forEach(t => {
        console.log(`  ${t.tablename}: ${t.size}`);
      });

      // Check indexer lag
      const serviceState = await sql`
        SELECT 
          last_processed_block,
          created_at,
          updated_at,
          EXTRACT(EPOCH FROM (NOW() - updated_at)) as seconds_since_update
        FROM service_state
        ORDER BY updated_at DESC
        LIMIT 1
      `;

      if (serviceState.length > 0) {
        const state = serviceState[0];
        console.log(`\nIndexer State:`);
        console.log(`  Last processed block: ${state.last_processed_block}`);
        console.log(`  Last update: ${Math.round(state.seconds_since_update)}s ago`);
        
        // Indexer should be keeping up (updated within last 30 seconds)
        assert(
          state.seconds_since_update < 30,
          `Indexer should be actively processing (last update ${state.seconds_since_update}s ago)`
        );
      }
    });

    it("verifies no data loss with enhanced mode", async () => {
      // Comprehensive data integrity check
      const integrityChecks = await sql`
        SELECT 
          'Buckets without MSP' as check_name,
          COUNT(*) as count
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE m.id IS NULL
        
        UNION ALL
        
        SELECT 
          'Files without bucket' as check_name,
          COUNT(*) as count
        FROM file f
        LEFT JOIN bucket b ON f.bucket_id = b.id
        WHERE b.id IS NULL
        
        UNION ALL
        
        SELECT 
          'BSP files without file' as check_name,
          COUNT(*) as count
        FROM bsp_file bf
        LEFT JOIN file f ON bf.file_id = f.id
        WHERE f.id IS NULL
        
        UNION ALL
        
        SELECT 
          'BSP files without BSP' as check_name,
          COUNT(*) as count
        FROM bsp_file bf
        LEFT JOIN bsp b ON bf.bsp_id = b.id
        WHERE b.id IS NULL
      `;

      console.log("\nData Integrity Checks:");
      let hasIssues = false;
      integrityChecks.forEach(check => {
        console.log(`  ${check.check_name}: ${check.count}`);
        if (Number(check.count) > 0) {
          hasIssues = true;
        }
      });

      assert(!hasIssues, "No data integrity issues should be found");

      // Verify enhanced mode captures more data than original lite mode would
      const enhancedDataStats = await sql`
        SELECT 
          (SELECT COUNT(*) FROM bucket WHERE msp_id NOT IN (
            SELECT id FROM msp WHERE onchain_msp_id = ${userApi.shConsts.NODE_INFOS.msp1.AddressId}
          )) as non_msp1_buckets,
          (SELECT COUNT(*) FROM bsp_file) as bsp_associations,
          (SELECT COUNT(*) FROM block_event WHERE method IN ('AcceptedBspVolunteer', 'BspConfirmedStoring')) as bsp_events
      `;

      const stats = enhancedDataStats[0];
      console.log("\nEnhanced Mode Additional Data:");
      console.log(`  Non-MSP1 buckets indexed: ${stats.non_msp1_buckets}`);
      console.log(`  BSP file associations: ${stats.bsp_associations}`);
      console.log(`  BSP events indexed: ${stats.bsp_events}`);

      // Should have captured additional data beyond original lite mode
      assert(
        Number(stats.non_msp1_buckets) > 0 || Number(stats.bsp_associations) > 0,
        "Enhanced mode should index additional data beyond original lite mode"
      );
    });
  }
);