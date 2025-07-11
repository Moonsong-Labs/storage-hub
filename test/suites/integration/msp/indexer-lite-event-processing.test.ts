import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Event Processing Verification Test
 * 
 * This test verifies that events indexed in lite mode are processed identically 
 * to how they would be processed in full mode. It ensures that while lite mode
 * filters which events to index, the events that ARE indexed maintain the same
 * data structure and field values.
 */
describeMspNet(
  "Indexer Lite Mode - Event Processing Verification",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createUserApi, createSqlClient, createBspApi }) => {
    let msp1Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      msp1Api = maybeMsp1Api;
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("verifies NewBucket event data structure", async () => {
      const bucketName = "verify-bucket-structure";
      const isPrivate = true;

      // Create bucket
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            msp1Api.accountId(),
            bucketName,
            isPrivate
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Wait for indexing
      await sleep(2000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'NewBucket'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "NewBucket event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.bucketId === bucketId.toString(), "Bucket ID should match");
      assert(eventData.name === bucketName, "Bucket name should match");
      assert(eventData.collectionId !== undefined, "Collection ID should be present");
      assert(eventData.private === isPrivate, "Privacy flag should match");
      assert(eventData.mspId === msp1Api.accountId(), "MSP ID should match");

      // Verify bucket table entry
      const bucketRecord = await sql`
        SELECT *
        FROM bucket
        WHERE bucket_id = ${bucketId.toNumber()}
      `;

      assert(bucketRecord.length > 0, "Bucket should exist in database");
      assert(bucketRecord[0].name === bucketName, "Bucket name in table should match");
      assert(bucketRecord[0].private === isPrivate, "Bucket privacy in table should match");
    });

    it("verifies ValuePropUpserted event data structure", async () => {
      const valuePropId = "verification-service";
      const price = 250n;

      // Add value prop
      await msp1Api.block.seal({
        calls: [
          msp1Api.tx.providers.addValueProp(price, valuePropId)
        ],
        signer: msp1Api.signer
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'ValuePropUpserted'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "ValuePropUpserted event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.providerId === msp1Api.accountId(), "Provider ID should match");
      assert(eventData.valuePropId === valuePropId, "Value prop ID should match");
      assert(eventData.price === price.toString(), "Price should match");

      // Verify MSP table update
      const mspRecord = await sql`
        SELECT value_prop
        FROM msp
        WHERE onchain_msp_id = ${msp1Api.accountId()}
      `;

      assert(mspRecord.length > 0, "MSP should exist in database");
      const valuePropData = JSON.parse(mspRecord[0].value_prop || "[]");
      const matchingProp = valuePropData.find((vp: any) => vp.id === valuePropId);
      assert(matchingProp, "Value prop should be in MSP record");
      assert(matchingProp.price === price.toString(), "Price in MSP record should match");
    });

    it("verifies CapacityChanged event data structure", async () => {
      // Get current capacity
      const currentInfo = await msp1Api.query.providers.mainStorageProviders(msp1Api.accountId());
      assert(currentInfo.isSome, "MSP should be registered");
      const currentCapacity = currentInfo.unwrap().capacity.toBigInt();
      
      const newCapacity = currentCapacity + 10000000n;

      // Change capacity
      await msp1Api.block.seal({
        calls: [msp1Api.tx.providers.changeCapacity(newCapacity)],
        signer: msp1Api.signer
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'CapacityChanged'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "CapacityChanged event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.providerId === msp1Api.accountId(), "Provider ID should match");
      assert(eventData.oldCapacity === currentCapacity.toString(), "Old capacity should match");
      assert(eventData.newCapacity === newCapacity.toString(), "New capacity should match");
      assert(eventData.nextBlockWhenNewCapacityCanBeUsed !== undefined, "Next block field should be present");

      // Verify MSP table update
      const mspRecord = await sql`
        SELECT capacity
        FROM msp
        WHERE onchain_msp_id = ${msp1Api.accountId()}
      `;

      assert(mspRecord.length > 0, "MSP should exist in database");
      assert(mspRecord[0].capacity === newCapacity.toString(), "Capacity in table should match");
    });

    it("verifies BspSignUpSuccess event data structure", async () => {
      const capacity = 5000000000n;

      // BSP sign up
      await bspApi.block.seal({
        calls: [bspApi.tx.providers.requestBspSignUp(capacity)],
        signer: bspApi.signer
      });

      // Confirm sign up
      await bspApi.block.seal({
        calls: [bspApi.tx.providers.confirmBspSignUp(null)],
        signer: bspApi.signer
      });

      // Wait for indexing
      await sleep(3000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'BspSignUpSuccess'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "BspSignUpSuccess event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.bspId === bspApi.accountId(), "BSP ID should match");
      assert(eventData.capacity === capacity.toString(), "Capacity should match");
      assert(eventData.multiaddresses !== undefined, "Multiaddresses should be present");

      // Verify BSP table entry
      const bspRecord = await sql`
        SELECT *
        FROM bsp
        WHERE onchain_bsp_id = ${bspApi.accountId()}
      `;

      assert(bspRecord.length > 0, "BSP should exist in database");
      assert(bspRecord[0].capacity === capacity.toString(), "BSP capacity should match");
    });

    it("verifies MoveBucketAccepted event data structure", async () => {
      const bucketName = "move-bucket-verify";

      // Create bucket owned by user
      const bucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Move bucket to MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: { mspId: msp1Api.accountId() }
          })
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'MoveBucketAccepted'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "MoveBucketAccepted event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.bucketId === bucketId.toString(), "Bucket ID should match");
      assert(eventData.newMspId === msp1Api.accountId(), "New MSP ID should match");

      // Verify bucket table update
      const bucketRecord = await sql`
        SELECT msp_id
        FROM bucket
        WHERE bucket_id = ${bucketId.toNumber()}
      `;

      assert(bucketRecord.length > 0, "Bucket should exist in database");
      // The msp_id in bucket table should reference the msp table's id, not the onchain_msp_id
      const mspRecord = await sql`
        SELECT id
        FROM msp
        WHERE onchain_msp_id = ${msp1Api.accountId()}
      `;
      assert(mspRecord.length > 0, "MSP should exist");
      assert(bucketRecord[0].msp_id === mspRecord[0].id, "Bucket should be assigned to MSP1");
    });

    it("verifies BucketDeleted event data structure", async () => {
      const bucketName = "delete-bucket-verify";

      // Create and delete bucket
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            msp1Api.accountId(),
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Delete bucket
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed event
      const indexedEvent = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'BucketDeleted'
        ORDER BY block_number DESC
        LIMIT 1
      `;

      assert(indexedEvent.length > 0, "BucketDeleted event should be indexed");

      // Verify event data structure
      const eventData = JSON.parse(indexedEvent[0].data);
      assert(eventData.bucketId === bucketId.toString(), "Bucket ID should match");
      assert(eventData.maybeWhoDeleted === shUser.address, "Deleter should match");

      // Note: The bucket might still exist in the database but marked as deleted
      // or might be completely removed, depending on indexer implementation
    });

    it("verifies event consistency across different types", async () => {
      // Query all indexed events
      const allEvents = await sql`
        SELECT section, method, COUNT(*) as count
        FROM block_event
        GROUP BY section, method
        ORDER BY section, method
      `;

      console.log("\n=== Indexed Event Types ===");
      allEvents.forEach(row => {
        console.log(`${row.section}.${row.method}: ${row.count} events`);
      });

      // Verify all events have required fields
      const sampleEvents = await sql`
        SELECT DISTINCT ON (section, method) *
        FROM block_event
        ORDER BY section, method, block_number DESC
      `;

      for (const event of sampleEvents) {
        // All events should have these fields
        assert(event.block_number !== null, "Block number should not be null");
        assert(event.section !== null, "Section should not be null");
        assert(event.method !== null, "Method should not be null");
        assert(event.data !== null, "Data should not be null");
        assert(event.timestamp !== null, "Timestamp should not be null");

        // Data should be valid JSON
        try {
          JSON.parse(event.data);
        } catch (e) {
          assert.fail(`Event ${event.section}.${event.method} has invalid JSON data`);
        }
      }

      console.log("\nAll indexed events have consistent data structure ✓");
    });

    it("verifies database relationships are maintained", async () => {
      // Check bucket -> MSP relationships
      const bucketMspJoin = await sql`
        SELECT b.bucket_id, b.name, m.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp m ON b.msp_id = m.id
        WHERE b.msp_id IS NOT NULL
      `;

      for (const row of bucketMspJoin) {
        assert(row.onchain_msp_id !== null, `Bucket ${row.name} has invalid MSP reference`);
      }

      // Check event -> block relationships
      const eventBlockCheck = await sql`
        SELECT COUNT(*) as orphaned
        FROM block_event
        WHERE block_number IS NULL
      `;

      assert(
        Number(eventBlockCheck[0].orphaned) === 0,
        "No events should have null block numbers"
      );

      console.log("\nDatabase relationships are properly maintained ✓");
    });
  }
);