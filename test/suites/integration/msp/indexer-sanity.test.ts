import assert, { equal, strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient } from "../../../util";

describeMspNet(
  "Indexer Sanity Checks",
  { initialised: false, indexer: true },
  ({ before, it, createUserApi, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      sql = createSqlClient();
    });

    it("postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("db migrated with standard tables", async () => {
      const sqlResp = await sql`
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public'
            ORDER BY table_name;
        `;

      const tables = sqlResp.map((t) => t.table_name);

      // This is not exhaustive because we don't want this test to be brittle
      const expectedTables = ["bsp", "msp", "bucket"];

      assert(
        expectedTables.every((table) => tables.includes(table)),
        `Expected tables not found. \nExpected: ${expectedTables.join(", ")} \nFound: ${tables.join(", ")}`
      );
    });

    it("standard network setup populates table", async () => {
      const bsps = await sql`
            SELECT COUNT(*)
            FROM bsp;
        `;

      const msps = await sql`
            SELECT COUNT(*)
            FROM msp;
        `;

      const buckets = await sql`
            SELECT COUNT(*)
            FROM bucket;
        `;

      assert(bsps[0].count > 0, "BSP count should be 1");
      assert(msps[0].count > 0, "MSP count should be 1");
      equal(buckets[0].count, 0, "Buckets should be 0");
    });

    it("table updated on new block data", async () => {
      const bucketName = "kfc-family-feast";
      let sqlResp = await sql`
                SELECT *
                FROM bucket
                WHERE name = ${bucketName};
            `;
      assert(sqlResp.length === 0, "Bucket should not exist yet");

      await userApi.file.newBucket(bucketName);

      sqlResp = await sql`
                SELECT *
                FROM bucket
                WHERE name = ${bucketName};
            `;

      assert(sqlResp.length === 1, "Bucket should exist");
      strictEqual(
        sqlResp[0].name.toString(),
        bucketName,
        "Bucket name should match the one created"
      );
    });
  }
);
