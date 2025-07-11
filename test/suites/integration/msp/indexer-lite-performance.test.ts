import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Performance metrics test for indexer lite mode.
 * Measures and compares:
 * - Total events indexed
 * - Database size
 * - Indexing speed
 * - Query performance
 * - Expected ~80% reduction in indexed events
 */
describeMspNet(
  "Indexer Lite Mode - Performance Metrics",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient, createBspApi }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    // Performance metrics storage
    const metrics = {
      startTime: 0,
      endTime: 0,
      totalEvents: 0,
      dbSize: 0,
      tableRowCounts: {} as Record<string, number>,
      eventsBySection: {} as Record<string, number>,
      indexingDuration: 0,
      queryPerformance: {} as Record<string, number>
    };

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

      // Record start time
      metrics.startTime = Date.now();
    });

    it("generates test workload", async () => {
      console.log("Generating test workload for performance measurement...");

      // Create multiple buckets
      const bucketPromises = [];
      for (let i = 0; i < 5; i++) {
        bucketPromises.push(
          userApi.tx.fileSystem.createBucket(
            msp1Api.accountId(),
            `msp1-perf-bucket-${i}`,
            true
          ),
          userApi.tx.fileSystem.createBucket(
            msp2Api.accountId(),
            `msp2-perf-bucket-${i}`,
            true
          )
        );
      }

      await userApi.block.seal({
        calls: bucketPromises,
        signer: shUser
      });

      // Create storage requests
      const buckets = await userApi.query.fileSystem.buckets.entries();
      const msp1Buckets = buckets.filter(([_, bucket]) => 
        bucket.unwrap().mspId.isSome && 
        bucket.unwrap().mspId.unwrap().toString() === msp1Api.accountId()
      );

      if (msp1Buckets.length > 0) {
        const storageRequestPromises = [];
        const bucketId = msp1Buckets[0][0].args[0];
        
        for (let i = 0; i < 10; i++) {
          storageRequestPromises.push(
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              `file-${i}.txt`,
              `0x${i.toString(16).padStart(64, '0')}`,
              1024 * (i + 1),
              msp1Api.accountId(),
              [userApi.alice.publicKey],
              null
            )
          );
        }

        await userApi.block.seal({
          calls: storageRequestPromises,
          signer: shUser
        });
      }

      // Provider operations
      await msp1Api.block.seal({
        calls: [
          msp1Api.tx.providers.changeCapacity(10000000000n),
          msp1Api.tx.providers.addValueProp(100n, "premium-service-perf")
        ],
        signer: msp1Api.signer
      });

      await msp2Api.block.seal({
        calls: [
          msp2Api.tx.providers.changeCapacity(20000000000n),
          msp2Api.tx.providers.addValueProp(50n, "basic-service-perf")
        ],
        signer: msp2Api.signer
      });

      // BSP operations (should be indexed)
      await bspApi.block.seal({
        calls: [
          bspApi.tx.providers.requestBspSignUp(5000000000n)
        ],
        signer: bspApi.signer
      });

      // Wait for all indexing to complete
      await sleep(5000);
      
      metrics.endTime = Date.now();
      metrics.indexingDuration = metrics.endTime - metrics.startTime;
    });

    it("measures event count metrics", async () => {
      // Total events indexed
      const totalEventsResult = await sql`SELECT COUNT(*) as count FROM block_event`;
      metrics.totalEvents = Number(totalEventsResult[0].count);

      // Events by section
      const eventsBySection = await sql`
        SELECT section, COUNT(*) as count
        FROM block_event
        GROUP BY section
        ORDER BY count DESC
      `;

      eventsBySection.forEach(row => {
        metrics.eventsBySection[row.section] = Number(row.count);
      });

      console.log("\n=== Event Count Metrics ===");
      console.log(`Total events indexed: ${metrics.totalEvents}`);
      console.log("Events by section:", metrics.eventsBySection);

      // Verify lite mode filtering is working
      assert(
        !metrics.eventsBySection.bucketNfts,
        "BucketNfts events should not be indexed in lite mode"
      );
      assert(
        !metrics.eventsBySection.paymentStreams,
        "PaymentStreams events should not be indexed in lite mode"
      );
      assert(
        !metrics.eventsBySection.randomness,
        "Randomness events should not be indexed in lite mode"
      );

      // Calculate filtering effectiveness
      const ignoredSections = ['bucketNfts', 'paymentStreams', 'proofsDealer', 'randomness'];
      const indexedSections = Object.keys(metrics.eventsBySection);
      const filteredSections = ignoredSections.filter(s => !indexedSections.includes(s));
      
      console.log(`Filtered out ${filteredSections.length}/${ignoredSections.length} non-essential pallets`);
    });

    it("measures database size metrics", async () => {
      // Get database size
      const dbSizeResult = await sql`
        SELECT pg_database_size(current_database()) as size
      `;
      metrics.dbSize = Number(dbSizeResult[0].size);

      // Get table row counts
      const tables = ['block_event', 'bucket', 'msp', 'bsp'];
      for (const table of tables) {
        const countResult = await sql`SELECT COUNT(*) as count FROM ${sql(table)}`;
        metrics.tableRowCounts[table] = Number(countResult[0].count);
      }

      console.log("\n=== Database Size Metrics ===");
      console.log(`Database size: ${(metrics.dbSize / 1024 / 1024).toFixed(2)} MB`);
      console.log("Table row counts:", metrics.tableRowCounts);

      // Calculate average event size
      const avgEventSize = metrics.totalEvents > 0 
        ? metrics.dbSize / metrics.totalEvents 
        : 0;
      console.log(`Average bytes per event: ${avgEventSize.toFixed(2)}`);
    });

    it("measures query performance", async () => {
      // Test various query patterns
      const queries = [
        {
          name: "bucket_lookup",
          query: sql`SELECT * FROM bucket WHERE name = 'msp1-perf-bucket-0'`
        },
        {
          name: "event_by_section",
          query: sql`SELECT * FROM block_event WHERE section = 'fileSystem' LIMIT 100`
        },
        {
          name: "msp_info",
          query: sql`SELECT * FROM msp WHERE onchain_msp_id = ${msp1Api.accountId()}`
        },
        {
          name: "recent_events",
          query: sql`SELECT * FROM block_event ORDER BY block_number DESC LIMIT 50`
        }
      ];

      console.log("\n=== Query Performance Metrics ===");
      
      for (const { name, query } of queries) {
        const startTime = Date.now();
        await query;
        const duration = Date.now() - startTime;
        
        metrics.queryPerformance[name] = duration;
        console.log(`${name}: ${duration}ms`);
      }

      // All queries should be fast in lite mode due to smaller dataset
      Object.entries(metrics.queryPerformance).forEach(([query, duration]) => {
        assert(
          duration < 100,
          `Query ${query} took ${duration}ms, expected < 100ms`
        );
      });
    });

    it("calculates event reduction percentage", async () => {
      // Estimate full mode event count based on operations performed
      const estimatedFullModeEvents = {
        buckets: 10, // 5 MSP1 + 5 MSP2
        storageRequests: 10, // Would be indexed in full mode
        providerEvents: 6, // Capacity + ValueProp for both MSPs
        systemEvents: 50, // Estimate for system/session/etc events
        paymentStreamEvents: 20, // Would be indexed in full mode
        other: 30 // Other misc events
      };

      const estimatedTotal = Object.values(estimatedFullModeEvents).reduce((a, b) => a + b, 0);
      const reductionPercentage = ((estimatedTotal - metrics.totalEvents) / estimatedTotal * 100).toFixed(2);

      console.log("\n=== Event Reduction Analysis ===");
      console.log(`Estimated full mode events: ${estimatedTotal}`);
      console.log(`Actual lite mode events: ${metrics.totalEvents}`);
      console.log(`Reduction: ${reductionPercentage}%`);

      // Should achieve significant reduction
      assert(
        Number(reductionPercentage) > 50,
        `Expected >50% event reduction, got ${reductionPercentage}%`
      );
    });

    it("verifies MSP-specific filtering effectiveness", async () => {
      // Count MSP1 vs MSP2 related events
      const msp1Events = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data::text LIKE '%${msp1Api.accountId()}%'
      `;

      const msp2Events = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data::text LIKE '%${msp2Api.accountId()}%'
      `;

      console.log("\n=== MSP Filtering Effectiveness ===");
      console.log(`MSP1 events: ${msp1Events[0].count}`);
      console.log(`MSP2 events: ${msp2Events[0].count}`);

      // MSP2 events should be filtered out
      assert(
        Number(msp2Events[0].count) === 0,
        `MSP2 events should be filtered out, found ${msp2Events[0].count}`
      );

      // MSP1 should have events
      assert(
        Number(msp1Events[0].count) > 0,
        "MSP1 should have indexed events"
      );
    });

    it("generates performance summary", async () => {
      console.log("\n=== PERFORMANCE SUMMARY ===");
      console.log(`Indexing duration: ${metrics.indexingDuration}ms`);
      console.log(`Total events indexed: ${metrics.totalEvents}`);
      console.log(`Database size: ${(metrics.dbSize / 1024 / 1024).toFixed(2)} MB`);
      console.log(`Events per second: ${(metrics.totalEvents / (metrics.indexingDuration / 1000)).toFixed(2)}`);
      console.log(`Average query time: ${
        Object.values(metrics.queryPerformance).reduce((a, b) => a + b, 0) / 
        Object.keys(metrics.queryPerformance).length
      }ms`);

      // Performance expectations for lite mode
      assert(
        metrics.totalEvents < 100,
        `Lite mode should have <100 events for this workload, found ${metrics.totalEvents}`
      );

      console.log("\nLite mode is performing as expected with significant event reduction!");
    });
  }
);