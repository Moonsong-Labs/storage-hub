import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

describeMspNet(
  "Indexer Lite Mode - Basic Functionality",
  { initialised: false, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createSqlClient, createUserApi, createMsp2Api }) => {
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
    });

    it("indexes only MSP-relevant events", async () => {
      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      // Create buckets with different MSPs to test filtering
      const msp1BucketName = "msp1-bucket-lite-base";
      const msp2BucketName = "msp2-bucket-lite-base";

      // Create a bucket with MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            msp1BucketName,
            true
          )
        ],
        signer: shUser
      });

      // Create a bucket with MSP2
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            msp2BucketName,
            true
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(3000);

      // Query indexed buckets
      const bucketEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'NewBucket'
        ORDER BY block_number;
      `;

      // In lite mode, MSP1's indexer should only see MSP1's bucket
      const msp1BucketEvents = bucketEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.name === msp1BucketName;
      });

      const msp2BucketEvents = bucketEvents.filter(event => {
        const eventData = JSON.parse(event.data);
        return eventData.name === msp2BucketName;
      });

      // Since we're running with MSP1's indexer in lite mode, it should only index MSP1's events
      assert(
        msp1BucketEvents.length === 1,
        `MSP1's bucket should be indexed. Found ${msp1BucketEvents.length} events`
      );
      
      assert(
        msp2BucketEvents.length === 0,
        `MSP2's bucket should NOT be indexed in lite mode. Found ${msp2BucketEvents.length} events`
      );
    });

    it("indexes provider events for current MSP only", async () => {
      // Get current MSP1 capacity
      const msp1InfoBefore = await msp1Api.query.providers.mainStorageProviders(userApi.shConsts.NODE_INFOS.msp1.AddressId);
      assert(msp1InfoBefore.isSome, "MSP1 should be registered");

      // Change MSP1 capacity
      const newCapacity = msp1InfoBefore.unwrap().capacity.toBigInt() + 1000000n;
      await msp1Api.block.seal({
        calls: [msp1Api.tx.providers.changeCapacity(newCapacity)],
        signer: msp1Api.signer
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed provider events
      const providerEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'providers'
        AND method = 'CapacityChanged'
        ORDER BY block_number DESC
        LIMIT 10;
      `;

      // Should have the capacity change event
      assert(
        providerEvents.length > 0,
        "Should have indexed MSP1's capacity change event"
      );

      const latestEvent = providerEvents[0];
      const eventData = JSON.parse(latestEvent.data);
      
      assert(
        eventData.providerId === userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "Indexed event should be for MSP1"
      );
    });

    it("indexes file events only for current MSP's buckets", async () => {
      const msp1BucketName = "msp1-file-bucket-lite";
      
      // Create bucket with MSP1
      const newBucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            msp1BucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Create a storage request in MSP1's bucket
      const fileSize = 1024;
      const fileLocation = "test/file/path.txt";
      const fileFingerprint = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            fileLocation,
            fileFingerprint,
            fileSize,
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Wait for indexing
      await sleep(2000);

      // Check indexed file events
      const fileEvents = await sql`
        SELECT *
        FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'NewStorageRequest'
        ORDER BY block_number DESC;
      `;

      assert(
        fileEvents.length > 0,
        "Should have indexed storage request for MSP1's bucket"
      );

      const latestFileEvent = fileEvents[0];
      const eventData = JSON.parse(latestFileEvent.data);
      
      assert(
        eventData.bucketId === bucketId.toString(),
        "Indexed event should be for MSP1's bucket"
      );
    });

    it("ignores events from non-relevant pallets", async () => {
      // Query for events from pallets that should be ignored in lite mode
      const ignoredPalletEvents = await sql`
        SELECT section, method, COUNT(*) as count
        FROM block_event
        WHERE section IN ('bucketNfts', 'paymentStreams', 'proofsDealer', 'randomness')
        GROUP BY section, method;
      `;

      // In lite mode, these pallets should not have any indexed events
      assert(
        ignoredPalletEvents.length === 0,
        `No events from ignored pallets should be indexed. Found: ${JSON.stringify(ignoredPalletEvents)}`
      );
    });

    it("maintains database consistency in lite mode", async () => {
      // Check that referenced data is consistent
      const buckets = await sql`
        SELECT b.*, msp.onchain_msp_id
        FROM bucket b
        LEFT JOIN msp ON b.msp_id = msp.id
        WHERE b.msp_id IS NOT NULL;
      `;

      // All buckets with MSP should have valid MSP references
      for (const bucket of buckets) {
        assert(
          bucket.onchain_msp_id !== null,
          `Bucket ${bucket.name} has invalid MSP reference`
        );
      }

      // Check event integrity
      const eventCounts = await sql`
        SELECT section, method, COUNT(*) as count
        FROM block_event
        GROUP BY section, method
        ORDER BY count DESC;
      `;

      // Log event distribution for verification
      console.log("Event distribution in lite mode:", eventCounts);
    });
  }
);