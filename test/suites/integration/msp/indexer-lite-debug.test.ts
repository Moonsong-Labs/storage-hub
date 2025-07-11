import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, sleep } from "../../../util";

/**
 * Debug test for indexer lite mode
 */
describeMspNet(
  "Indexer Lite Mode - Debug",
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

      console.log("Postgres is ready");

      // Check if MSP1 container has indexer in its command
      const msp1Container = await userApi.docker.docker.listContainers({ 
        all: true, 
        filters: { name: ["docker-sh-msp-1"] } 
      });
      
      if (msp1Container.length > 0) {
        console.log("MSP1 container command:", msp1Container[0].Command);
      } else {
        console.log("MSP1 container not found!");
      }

      // Try to wait for indexer startup with a shorter timeout
      try {
        await userApi.docker.waitForLog({
          containerName: "docker-sh-msp-1",
          searchString: "IndexerService starting up",
          timeout: 5000
        });
        console.log("Indexer started successfully");
      } catch (e) {
        console.log("Failed to find indexer startup log:", e);
        
        // Get last few logs from MSP1
        try {
          const container = userApi.docker.docker.getContainer("docker-sh-msp-1");
          const logs = await container.logs({ 
            stdout: true, 
            stderr: true, 
            tail: 50 
          });
          console.log("Last 50 lines from MSP1:");
          console.log(logs.toString());
        } catch (logError) {
          console.log("Failed to get MSP1 logs:", logError);
        }
      }
    });

    it("checks basic database connectivity", async () => {
      // Check if we can query the database
      try {
        const tables = await sql`
          SELECT table_name 
          FROM information_schema.tables 
          WHERE table_schema = 'public' 
          ORDER BY table_name
        `;
        console.log("Database tables:", tables.map(t => t.table_name));
      } catch (e) {
        console.error("Failed to query database:", e);
        throw e;
      }

      // Check service state
      try {
        const serviceState = await sql`SELECT * FROM service_state`;
        console.log("Service state:", serviceState);
      } catch (e) {
        console.log("No service state found (indexer might not be running)");
      }

      // Check MSPs
      const msps = await sql`SELECT * FROM msp`;
      console.log(`Found ${msps.length} MSPs`);
      
      // Check what block we're at
      const currentBlock = await userApi.query.system.number();
      console.log(`Current blockchain block: ${currentBlock}`);
    });
  }
);