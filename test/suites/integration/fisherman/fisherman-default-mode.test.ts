import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  ShConsts,
  sleep
} from "../../../util";

describeMspNet(
  "Fisherman Default Indexer Mode",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    fishermanIndexerMode: "fishing"
  },
  ({ before, it, createFishermanApi, createSqlClient, createUserApi }) => {
    let fishermanApi: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();

      assert(createFishermanApi, "Fisherman API should be available when fisherman is enabled");

      // Wait for fisherman node to be ready before trying to connect
      await userApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "💤 Idle",
        timeout: 30000
      });

      fishermanApi = await createFishermanApi() as EnrichedBspApi;
      assert(fishermanApi, "Fisherman API should be created successfully");
      sql = createSqlClient();

      // Initialize blockchain state using direct RPC call for first block
      await userApi.rpc.engine.createBlock(true, true);

      // Small delay to ensure nodes are synced
      await sleep(1000);

      // Seal additional blocks to ensure stable state
      await userApi.block.seal();
      await userApi.block.seal();
    });

    it("fisherman node automatically enables indexer in fishing mode", async () => {
      // Check logs to verify fishing mode is active on the fisherman node
      await fishermanApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "IndexerService starting up in Fishing mode!",
        timeout: 10000
      });
    });

    it("fisherman node connects to postgres database", async () => {
      // Wait for fisherman indexer to start
      await fishermanApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "IndexerService starting up",
        timeout: 10000
      });

      // Verify database connection by checking service_state table
      const serviceState = await sql`SELECT * FROM service_state WHERE id = 1`;
      assert(serviceState.length > 0, "Service state should be initialized in database");
    });

    it("fisherman node syncs with network", async () => {
      // Verify fisherman node is syncing blocks
      await fishermanApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "💤 Idle",
        timeout: 30000
      });

      // Verify fisherman node is at a reasonable block height
      const header = await fishermanApi.rpc.chain.getHeader();
      assert(header.number.toNumber() > 0, "Fisherman node should be syncing blocks");

      // Create some activity for the indexer to process
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify indexer is processing blocks
      const serviceState = await sql`SELECT last_processed_block FROM service_state WHERE id = 1`;
      assert(serviceState[0]?.last_processed_block >= 0, "Indexer should be processing blocks");
    });
  }
);
