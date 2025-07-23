import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, bspKey } from "../../../util";

describeMspNet("Indexer Modes Comparison", { initialised: false, skip: true }, () => {
  // Test Full Mode
  describeMspNet(
    "Full Indexer Mode",
    { initialised: false, indexer: true, userIndexerMode: "full" },
    ({ before, it, createUserApi, createBspApi, createSqlClient }) => {
      let userApi: EnrichedBspApi;
      let bspApi: EnrichedBspApi;
      let sql: SqlClient;

      before(async () => {
        userApi = await createUserApi();
        bspApi = await createBspApi();
        sql = createSqlClient();

        // Initialize blockchain state to prevent Aura consensus errors
        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();
      });

      it("indexes all event types in full mode", async () => {
        const bucketName = "test-full-mode";

        // Create various events
        await userApi.file.newBucket(bucketName);

        // Trigger capacity change (should be indexed in full mode)
        await bspApi.block.seal({
          calls: [bspApi.tx.providers.changeCapacity(2000000)],
          signer: bspKey
        });

        // Create payment stream (should be indexed in full mode)
        await userApi.block.seal({
          calls: [
            userApi.tx.paymentStreams.createFixedRatePaymentStream(
              bspApi.shConsts.DUMMY_BSP_ID,
              bspKey.address,
              100
            )
          ]
        });

        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();

        // Verify all events are indexed
        const buckets = await sql`SELECT * FROM bucket WHERE name = ${bucketName}`;
        assert.equal(buckets.length, 1, "Bucket should be indexed");

        const bsp = await sql`SELECT capacity FROM bsp WHERE id = ${bspApi.shConsts.DUMMY_BSP_ID}`;
        assert.equal(bsp[0].capacity, 2000000, "Capacity change should be indexed in full mode");

        const streams = await sql`SELECT COUNT(*) FROM paymentstream`;
        assert(streams[0].count > 0, "Payment streams should be indexed in full mode");
      });
    }
  );

  // Test Lite Mode
  describeMspNet(
    "Lite Indexer Mode",
    { initialised: false, indexer: true, userIndexerMode: "lite" },
    ({ before, it, createUserApi, createBspApi, createSqlClient }) => {
      let userApi: EnrichedBspApi;
      let bspApi: EnrichedBspApi;
      let sql: SqlClient;

      before(async () => {
        userApi = await createUserApi();
        bspApi = await createBspApi();
        sql = createSqlClient();

        // Initialize blockchain state to prevent Aura consensus errors
        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();
      });

      it("indexes MSP-specific events in lite mode", async () => {
        const bucketName = "test-lite-mode";
        const mspId = userApi.shConsts.DUMMY_MSP_ID;

        // Get value proposition for MSP
        const valueProps =
          await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
        const valuePropId = valueProps[0].id;

        // Create MSP bucket
        await userApi.block.seal({
          calls: [userApi.tx.fileSystem.createBucket(mspId, bucketName, true, valuePropId)]
        });

        // Create storage request (MSP should accept it)
        const bucketEvent = await userApi.assert.eventPresent("fileSystem", "NewBucket");
        const bucketId = (bucketEvent.data as any).bucketId;
        const fileMetadata = await userApi.file.newStorageRequest(
          "/res/smile.jpg",
          "test/lite-file.txt",
          bucketId
        );

        // Trigger non-MSP event (should NOT be indexed in lite mode)
        await bspApi.block.seal({
          calls: [bspApi.tx.providers.changeCapacity(3000000)],
          signer: bspKey
        });

        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();

        // Verify MSP events are indexed
        const mspFiles = await sql`
            SELECT * FROM msp_file 
            WHERE file_id = (
              SELECT id FROM file WHERE file_key = ${fileMetadata.fileKey}
            )
          `;
        assert(mspFiles.length > 0, "MSP file associations should be indexed in lite mode");

        // Verify non-MSP events are NOT indexed
        const bsp = await sql`SELECT capacity FROM bsp WHERE id = ${bspApi.shConsts.DUMMY_BSP_ID}`;
        assert.notEqual(
          bsp[0].capacity,
          3000000,
          "BSP capacity changes should NOT be indexed in lite mode"
        );
      });
    }
  );

  // Test Fishing Mode
  describeMspNet(
    "Fishing Indexer Mode",
    { initialised: false, indexer: true, userIndexerMode: "fishing" },
    ({ before, it, createUserApi, createBspApi, createSqlClient }) => {
      let userApi: EnrichedBspApi;
      let bspApi: EnrichedBspApi;
      let sql: SqlClient;

      before(async () => {
        userApi = await createUserApi();
        bspApi = await createBspApi();
        sql = createSqlClient();

        // Initialize blockchain state to prevent Aura consensus errors
        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();
      });

      it("indexes only file-related events in fishing mode", async () => {
        const bucketName = "test-fishing-mode";

        // Create file events
        const newBucketEvent = await userApi.file.newBucket(bucketName);
        const newBucketEventData =
          userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

        if (!newBucketEventData) {
          throw new Error("NewBucket event data not found");
        }

        const bucketId = newBucketEventData.bucketId;
        const fileMetadata = await userApi.file.newStorageRequest(
          "/res/smile.jpg",
          "test/fishing-file.txt",
          bucketId
        );

        // BSP volunteers (should be indexed)
        await userApi.block.seal({
          calls: [bspApi.tx.fileSystem.bspVolunteer(fileMetadata.fileKey)],
          signer: bspKey
        });

        // Skip blocks until the BSP can change its capacity
        await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

        // Wait for BSP to be available to send transactions
        await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());

        // Trigger non-file events (should NOT be indexed)
        await userApi.block.seal({
          calls: [bspApi.tx.providers.changeCapacity(4000000)],
          signer: bspKey
        });

        await userApi.block.seal({
          calls: [
            userApi.tx.paymentStreams.createFixedRatePaymentStream(
              bspApi.shConsts.DUMMY_BSP_ID,
              bspKey.address,
              200
            )
          ]
        });

        await userApi.block.seal();
        await userApi.block.seal();
        await userApi.block.seal();

        // Verify file events are indexed
        const files = await sql`SELECT * FROM file WHERE file_key = ${fileMetadata.fileKey}`;
        assert.equal(files.length, 1, "Files should be indexed in fishing mode");

        const bspFiles = await sql`
            SELECT * FROM bsp_file 
            WHERE file_id = ${files[0].id}
          `;
        assert(bspFiles.length > 0, "BSP file associations should be indexed in fishing mode");

        // Verify non-file events are NOT indexed
        const bsp = await sql`SELECT capacity FROM bsp WHERE id = ${bspApi.shConsts.DUMMY_BSP_ID}`;
        assert.notEqual(
          bsp[0].capacity,
          4000000,
          "Capacity changes should NOT be indexed in fishing mode"
        );

        const streams = await sql`SELECT COUNT(*) FROM paymentstream`;
        assert.equal(streams[0].count, 0, "Payment streams should NOT be indexed in fishing mode");
      });
    }
  );
});
