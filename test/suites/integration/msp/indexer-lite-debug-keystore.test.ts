import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Debug test to check keystore and MSP ID detection in lite mode
 */
describeMspNet(
  "Indexer Lite Mode - Keystore Debug",
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

      console.log("MSP1 address:", msp1Api.address);
      console.log("MSP2 address:", msp2Api.address);
    });

    it("checks MSP registration before indexer test", async () => {
      // Check if MSPs are registered on-chain
      const msp1Info = await msp1Api.query.providers.mainStorageProviders(msp1Api.address);
      const msp2Info = await msp2Api.query.providers.mainStorageProviders(msp2Api.address);

      console.log("MSP1 on-chain info:", msp1Info.toHuman());
      console.log("MSP2 on-chain info:", msp2Info.toHuman());

      // Check if MSPs have provider IDs
      const msp1ProviderId = await msp1Api.query.providers.accountIdToMainStorageProviderId(msp1Api.address);
      const msp2ProviderId = await msp2Api.query.providers.accountIdToMainStorageProviderId(msp2Api.address);

      console.log("MSP1 provider ID:", msp1ProviderId.toHuman());
      console.log("MSP2 provider ID:", msp2ProviderId.toHuman());

      // Wait a bit for indexer to process
      await sleep(5000);

      // Check what's in the database
      const msps = await sql`
        SELECT onchain_msp_id, capacity, created_at
        FROM msp
        ORDER BY onchain_msp_id
      `;

      console.log(`\nMSPs in database: ${msps.length}`);
      msps.forEach(msp => {
        console.log(`  - ${msp.onchain_msp_id} (capacity: ${msp.capacity}, created: ${msp.created_at})`);
      });

      // Check service state
      const serviceState = await sql`
        SELECT last_processed_block
        FROM service_state
        LIMIT 1
      `;

      if (serviceState.length > 0) {
        console.log(`\nIndexer last processed block: ${serviceState[0].last_processed_block}`);
      } else {
        console.log("\nNo service state found - indexer may not be running");
      }

      // Get current block number
      const currentBlock = await msp1Api.rpc.chain.getBlock();
      console.log(`Current block number: ${currentBlock.block.header.number.toNumber()}`);
    });

    it("creates a bucket and checks immediate indexing", async () => {
      // Create a bucket for MSP1
      const bucketName = `debug-bucket-${Date.now()}`;
      console.log(`\nCreating bucket: ${bucketName} for MSP1`);
      
      await userApi.file.newBucket(bucketName, { msp: msp1Api.address });
      
      // Check immediately
      let bucket = await sql`
        SELECT name, msp_id
        FROM bucket
        WHERE name = ${bucketName}
      `;
      
      console.log("Bucket found immediately:", bucket.length > 0);
      
      // Wait and check again
      await sleep(5000);
      
      bucket = await sql`
        SELECT b.name, b.msp_id, m.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.name = ${bucketName}
      `;
      
      if (bucket.length > 0) {
        console.log("Bucket indexed successfully:");
        console.log(`  - Name: ${bucket[0].name}`);
        console.log(`  - MSP ID in DB: ${bucket[0].msp_id}`);
        console.log(`  - MSP onchain ID: ${bucket[0].onchain_msp_id}`);
      } else {
        console.log("Bucket not found in database after waiting");
        
        // Check all buckets
        const allBuckets = await sql`
          SELECT name, msp_id
          FROM bucket
          ORDER BY created_at DESC
          LIMIT 5
        `;
        
        console.log("\nRecent buckets in database:");
        allBuckets.forEach(b => {
          console.log(`  - ${b.name} (msp_id: ${b.msp_id})`);
        });
      }
    });

    it("monitors indexer progress", async () => {
      // Monitor for 30 seconds
      const startTime = Date.now();
      let lastBlock = -1;
      
      while (Date.now() - startTime < 30000) {
        const serviceState = await sql`
          SELECT last_processed_block
          FROM service_state
          LIMIT 1
        `;
        
        if (serviceState.length > 0) {
          const currentProcessedBlock = serviceState[0].last_processed_block;
          if (currentProcessedBlock !== lastBlock) {
            console.log(`Indexer processed block: ${currentProcessedBlock}`);
            lastBlock = currentProcessedBlock;
          }
        }
        
        await sleep(2000);
      }
    });
  }
);