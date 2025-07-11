import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Test MSP-specific event filtering in lite mode.
 * This test verifies that:
 * 1. ValueProp events are filtered to only index current MSP's events
 * 2. Bucket operations are filtered based on MSP ownership
 * 3. File operations within MSP buckets are handled correctly
 * 4. Provider lifecycle events are properly indexed
 */
describeMspNet(
  "Indexer Lite Mode - MSP-Specific Event Filtering",
  { initialised: false, indexer: true, indexerMode: "lite" },
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
    });

    it("filters ValueProp events for current MSP only", async () => {
      // Add ValueProp for MSP1 (should be indexed)
      const msp1ValuePropId = "msp1-service-premium";
      const msp1Price = 100n;
      
      await msp1Api.block.seal({
        calls: [
          msp1Api.tx.providers.addValueProp(
            msp1Price,
            msp1ValuePropId
          )
        ],
        signer: msp1Api.signer
      });

      // Add ValueProp for MSP2 (should NOT be indexed)
      const msp2ValuePropId = "msp2-service-basic";
      const msp2Price = 50n;
      
      await msp2Api.block.seal({
        calls: [
          msp2Api.tx.providers.addValueProp(
            msp2Price,
            msp2ValuePropId
          )
        ],
        signer: msp2Api.signer
      });

      // Wait for indexing
      await sleep(3000);

      // Check indexed ValueProp events
      const valuePropEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'ValuePropUpserted'
        ORDER BY block_number;
      `;

      // Filter events by MSP
      const msp1ValuePropEvents = valuePropEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.providerId === msp1Api.accountId();
      });

      const msp2ValuePropEvents = valuePropEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.providerId === msp2Api.accountId();
      });

      // In lite mode with MSP1's indexer, only MSP1's ValueProp should be indexed
      assert(
        msp1ValuePropEvents.length === 1,
        `Should have exactly 1 MSP1 ValueProp event, found ${msp1ValuePropEvents.length}`
      );
      
      assert(
        msp2ValuePropEvents.length === 0,
        `Should have 0 MSP2 ValueProp events, found ${msp2ValuePropEvents.length}`
      );

      // Verify the indexed event data
      const msp1Event = JSON.parse(msp1ValuePropEvents[0].data);
      assert(msp1Event.valuePropId === msp1ValuePropId, "ValueProp ID should match");
      assert(msp1Event.price === msp1Price.toString(), "Price should match");
    });

    it("filters bucket operations by MSP ownership", async () => {
      // Create bucket owned by MSP1 (should be indexed)
      const msp1BucketName = "msp1-owned-bucket-specific";
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            msp1Api.accountId(),
            msp1BucketName,
            true
          )
        ],
        signer: shUser
      });

      // Create bucket owned by MSP2 (should NOT be indexed)
      const msp2BucketName = "msp2-owned-bucket-specific";
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            msp2Api.accountId(),
            msp2BucketName,
            true
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed buckets
      const buckets = await sql`
        SELECT name, msp_id
        FROM bucket
        WHERE name IN (${msp1BucketName}, ${msp2BucketName})
      `;

      // Should only find MSP1's bucket
      assert(
        buckets.length === 1,
        `Should only have MSP1's bucket indexed, found ${buckets.length} buckets`
      );
      assert(
        buckets[0].name === msp1BucketName,
        "Only MSP1's bucket should be indexed"
      );

      // Verify bucket events
      const bucketEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'NewBucket'
        AND data::text LIKE '%${msp1BucketName}%' OR data::text LIKE '%${msp2BucketName}%'
      `;

      const msp1BucketEvents = bucketEvents.filter(e => 
        JSON.parse(e.data).name === msp1BucketName
      );
      const msp2BucketEvents = bucketEvents.filter(e => 
        JSON.parse(e.data).name === msp2BucketName
      );

      assert(msp1BucketEvents.length === 1, "MSP1's bucket event should be indexed");
      assert(msp2BucketEvents.length === 0, "MSP2's bucket event should NOT be indexed");
    });

    it("indexes MoveBucket events involving current MSP", async () => {
      // Create a bucket owned by user
      const userBucketName = "user-bucket-for-move";
      const newBucketEvent = await userApi.file.newBucket(userBucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Move bucket to MSP1 (should be indexed)
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: msp1Api.accountId()
            }
          })
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check for MoveBucketAccepted event
      const moveBucketEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'MoveBucketAccepted'
        ORDER BY block_number DESC
      `;

      // Should have the move bucket event since it involves MSP1
      assert(
        moveBucketEvents.length > 0,
        "MoveBucketAccepted event involving MSP1 should be indexed"
      );

      const latestMoveEvent = JSON.parse(moveBucketEvents[0].data);
      assert(
        latestMoveEvent.newMspId === msp1Api.accountId(),
        "Move event should show bucket moved to MSP1"
      );
    });

    it("indexes provider lifecycle events for current MSP", async () => {
      // Test MSP capacity change (should be indexed for MSP1)
      const currentInfo = await msp1Api.query.providers.mainStorageProviders(msp1Api.accountId());
      assert(currentInfo.isSome, "MSP1 should be registered");
      
      const currentCapacity = currentInfo.unwrap().capacity.toBigInt();
      const newCapacity = currentCapacity + 5000000n;

      await msp1Api.block.seal({
        calls: [msp1Api.tx.providers.changeCapacity(newCapacity)],
        signer: msp1Api.signer
      });

      // Change MSP2's capacity (should NOT be indexed)
      await msp2Api.block.seal({
        calls: [msp2Api.tx.providers.changeCapacity(newCapacity + 1000000n)],
        signer: msp2Api.signer
      });

      // Wait for indexing
      await sleep(2000);

      // Check capacity change events
      const capacityEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'CapacityChanged'
        ORDER BY block_number DESC
      `;

      const msp1CapacityEvents = capacityEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.providerId === msp1Api.accountId();
      });

      const msp2CapacityEvents = capacityEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.providerId === msp2Api.accountId();
      });

      // Only MSP1's capacity change should be indexed
      assert(
        msp1CapacityEvents.length > 0,
        "MSP1's capacity change should be indexed"
      );
      
      assert(
        msp2CapacityEvents.length === 0,
        "MSP2's capacity change should NOT be indexed"
      );

      // Verify the capacity value
      const latestMsp1Event = JSON.parse(msp1CapacityEvents[0].data);
      assert(
        latestMsp1Event.newCapacity === newCapacity.toString(),
        "New capacity should match"
      );
    });

    it("filters storage request events by bucket ownership", async () => {
      // Create bucket owned by MSP1
      const msp1StorageBucket = "msp1-storage-bucket";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            msp1Api.accountId(),
            msp1StorageBucket,
            true
          )
        ],
        signer: shUser
      });

      const msp1BucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(msp1BucketId, "Failed to get MSP1 bucket ID");

      // Create storage request in MSP1's bucket (should be indexed)
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            msp1BucketId,
            "msp1-file.txt",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1024,
            msp1Api.accountId(),
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check storage request events
      const storageRequestEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'NewStorageRequest'
      `;

      // Should have the storage request for MSP1's bucket
      const msp1StorageEvents = storageRequestEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.bucketId === msp1BucketId.toString();
      });

      assert(
        msp1StorageEvents.length === 1,
        "Storage request in MSP1's bucket should be indexed"
      );
    });

    it("verifies database state consistency", async () => {
      // Check MSP table
      const msps = await sql`
        SELECT onchain_msp_id, value_prop
        FROM msp
        WHERE onchain_msp_id IN (${msp1Api.accountId()}, ${msp2Api.accountId()})
      `;

      // Should only have MSP1 in the database
      assert(
        msps.length === 1,
        `Should only have MSP1 in database, found ${msps.length} MSPs`
      );
      assert(
        msps[0].onchain_msp_id === msp1Api.accountId(),
        "Only MSP1 should be in the database"
      );

      // Check event summary
      const eventSummary = await sql`
        SELECT section, method, COUNT(*) as count
        FROM block_event
        WHERE section IN ('providers', 'fileSystem')
        GROUP BY section, method
        ORDER BY section, method;
      `;

      console.log("Event summary in lite mode:", eventSummary);

      // Verify no MSP2 data in any events
      const msp2RelatedEvents = await sql`
        SELECT COUNT(*) as count
        FROM block_event
        WHERE data::text LIKE '%${msp2Api.accountId()}%'
      `;

      assert(
        Number(msp2RelatedEvents[0].count) === 0,
        `No MSP2 events should be indexed, found ${msp2RelatedEvents[0].count}`
      );
    });
  }
);