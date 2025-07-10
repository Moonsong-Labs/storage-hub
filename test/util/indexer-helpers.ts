import { type BspNetConfig } from "./bspNet/types";
import { describeMspNet, type EnrichedBspApi, type SqlClient } from "./index";

/**
 * Helper utilities for testing indexer in different modes
 */

export interface IndexerTestContext {
  userApi: EnrichedBspApi;
  msp1Api: EnrichedBspApi;
  msp2Api: EnrichedBspApi;
  bspApi: EnrichedBspApi;
  sql: SqlClient;
}

/**
 * Run a test function with a specific indexer mode
 */
export async function runWithIndexerMode(
  mode: "full" | "lite",
  testName: string,
  testFn: (context: IndexerTestContext) => Promise<void>
): Promise<void> {
  return new Promise((resolve, reject) => {
    describeMspNet(
      `${testName} (${mode} mode)`,
      { initialised: false, indexer: true, indexerMode: mode },
      ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createBspApi, createSqlClient }) => {
        let context: IndexerTestContext;

        before(async () => {
          const maybeMsp1Api = await createMsp1Api();
          const maybeMsp2Api = await createMsp2Api();
          if (!maybeMsp1Api || !maybeMsp2Api) {
            throw new Error("MSP APIs not available");
          }

          context = {
            userApi: await createUserApi(),
            msp1Api: maybeMsp1Api,
            msp2Api: maybeMsp2Api,
            bspApi: await createBspApi(),
            sql: createSqlClient()
          };

          // Wait for indexer to be ready
          await context.userApi.docker.waitForLog({
            containerName: "docker-sh-postgres-1",
            searchString: "database system is ready to accept connections",
            timeout: 5000
          });
        });

        it("runs test scenario", async () => {
          try {
            await testFn(context);
            resolve();
          } catch (error) {
            reject(error);
          }
        });
      }
    );
  });
}

/**
 * Compare results between full and lite indexer modes
 */
export async function compareIndexerModes<T>(
  testName: string,
  operation: (context: IndexerTestContext) => Promise<T>
): Promise<{ full: T; lite: T }> {
  const results: { full?: T; lite?: T } = {};

  // Run in full mode
  await runWithIndexerMode("full", `${testName} - Full Mode`, async (context) => {
    results.full = await operation(context);
  });

  // Run in lite mode
  await runWithIndexerMode("lite", `${testName} - Lite Mode`, async (context) => {
    results.lite = await operation(context);
  });

  if (!results.full || !results.lite) {
    throw new Error("Failed to get results from both modes");
  }

  return { full: results.full, lite: results.lite };
}

/**
 * Get event statistics for comparison
 */
export async function getEventStatistics(sql: SqlClient): Promise<{
  totalEvents: number;
  eventsBySection: Record<string, number>;
  eventsByMethod: Record<string, number>;
  uniqueSections: string[];
  databaseSize: number;
}> {
  // Total events
  const totalResult = await sql`SELECT COUNT(*) as count FROM block_event`;
  const totalEvents = Number(totalResult[0].count);

  // Events by section
  const sectionResult = await sql`
    SELECT section, COUNT(*) as count
    FROM block_event
    GROUP BY section
    ORDER BY count DESC
  `;
  const eventsBySection: Record<string, number> = {};
  sectionResult.forEach(row => {
    eventsBySection[row.section] = Number(row.count);
  });

  // Events by method
  const methodResult = await sql`
    SELECT method, COUNT(*) as count
    FROM block_event
    GROUP BY method
    ORDER BY count DESC
  `;
  const eventsByMethod: Record<string, number> = {};
  methodResult.forEach(row => {
    eventsByMethod[row.method] = Number(row.count);
  });

  // Unique sections
  const uniqueSections = Object.keys(eventsBySection);

  // Database size (approximation)
  const dbSizeResult = await sql`
    SELECT pg_database_size(current_database()) as size
  `;
  const databaseSize = Number(dbSizeResult[0].size);

  return {
    totalEvents,
    eventsBySection,
    eventsByMethod,
    uniqueSections,
    databaseSize
  };
}

/**
 * Filter events by MSP
 */
export async function getMspSpecificEvents(
  sql: SqlClient,
  mspId: string
): Promise<{
  count: number;
  events: any[];
}> {
  const result = await sql`
    SELECT *
    FROM block_event
    WHERE data::text LIKE ${'%' + mspId + '%'}
    ORDER BY block_number DESC
  `;

  return {
    count: result.length,
    events: result
  };
}

/**
 * Verify event filtering rules
 */
export async function verifyLiteModeFiltering(
  sql: SqlClient,
  expectedMspId: string
): Promise<{
  passed: boolean;
  issues: string[];
}> {
  const issues: string[] = [];

  // Check for ignored pallets
  const ignoredPallets = ["bucketNfts", "paymentStreams", "proofsDealer", "randomness"];
  for (const pallet of ignoredPallets) {
    const count = await sql`
      SELECT COUNT(*) as count
      FROM block_event
      WHERE section = ${pallet}
    `;
    if (Number(count[0].count) > 0) {
      issues.push(`Found ${count[0].count} events from ignored pallet: ${pallet}`);
    }
  }

  // Check for other MSPs' events
  const allEvents = await sql`SELECT data FROM block_event`;
  const otherMspEvents = allEvents.filter(event => {
    const data = JSON.parse(event.data);
    return data.providerId && 
           data.providerId !== expectedMspId && 
           data.providerId.startsWith("0x");
  });

  if (otherMspEvents.length > 0) {
    issues.push(`Found ${otherMspEvents.length} events from other MSPs`);
  }

  return {
    passed: issues.length === 0,
    issues
  };
}

/**
 * Measure indexing performance
 */
export async function measureIndexingPerformance(
  context: IndexerTestContext,
  operations: () => Promise<void>
): Promise<{
  duration: number;
  eventsIndexed: number;
  eventsPerSecond: number;
}> {
  const startTime = Date.now();
  const startEvents = await context.sql`SELECT COUNT(*) as count FROM block_event`;
  const startCount = Number(startEvents[0].count);

  await operations();

  const endTime = Date.now();
  const endEvents = await context.sql`SELECT COUNT(*) as count FROM block_event`;
  const endCount = Number(endEvents[0].count);

  const duration = endTime - startTime;
  const eventsIndexed = endCount - startCount;
  const eventsPerSecond = eventsIndexed / (duration / 1000);

  return {
    duration,
    eventsIndexed,
    eventsPerSecond
  };
}

/**
 * Utility to check if specific events exist
 */
export async function checkEventExists(
  sql: SqlClient,
  section: string,
  method: string,
  additionalFilter?: Record<string, any>
): Promise<boolean> {
  let query = sql`
    SELECT COUNT(*) as count
    FROM block_event
    WHERE section = ${section}
    AND method = ${method}
  `;

  if (additionalFilter) {
    const events = await sql`
      SELECT *
      FROM block_event
      WHERE section = ${section}
      AND method = ${method}
    `;

    const filtered = events.filter(event => {
      const data = JSON.parse(event.data);
      return Object.entries(additionalFilter).every(([key, value]) => 
        data[key] === value
      );
    });

    return filtered.length > 0;
  }

  const result = await query;
  return Number(result[0].count) > 0;
}

/**
 * Generate test report comparing modes
 */
export function generateComparisonReport(
  fullStats: any,
  liteStats: any
): string {
  const reduction = ((fullStats.totalEvents - liteStats.totalEvents) / fullStats.totalEvents * 100).toFixed(2);
  
  return `
=== Indexer Mode Comparison Report ===

Full Mode:
- Total Events: ${fullStats.totalEvents}
- Database Size: ${(fullStats.databaseSize / 1024 / 1024).toFixed(2)} MB
- Unique Sections: ${fullStats.uniqueSections.length}

Lite Mode:
- Total Events: ${liteStats.totalEvents}
- Database Size: ${(liteStats.databaseSize / 1024 / 1024).toFixed(2)} MB
- Unique Sections: ${liteStats.uniqueSections.length}

Reduction: ${reduction}%
Space Saved: ${((fullStats.databaseSize - liteStats.databaseSize) / 1024 / 1024).toFixed(2)} MB

Filtered Sections:
${fullStats.uniqueSections.filter(s => !liteStats.uniqueSections.includes(s)).join(", ") || "None"}
`;
}