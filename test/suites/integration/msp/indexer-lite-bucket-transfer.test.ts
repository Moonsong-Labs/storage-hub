import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Enhanced Indexer Lite Mode - Bucket Transfer Tests
 * 
 * This test verifies that the enhanced lite mode correctly indexes bucket transfers
 * between MSPs, ensuring complete visibility of transferred buckets and their files.
 */
describeMspNet(
  "Indexer Lite Mode - Bucket Transfer Support",
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

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      await userApi.docker.waitForLog({
        containerName: "docker-sh-msp-1",
        searchString: "IndexerService starting up in",
        timeout: 10000
      });

      // Give indexer time to sync
      await sleep(5000);
    });

    it("indexes bucket transferred from MSP2 to MSP1", async () => {
      // Create bucket owned by MSP2
      const bucketName = "msp2-bucket-for-transfer";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Add files to MSP2's bucket
      const file1 = {
        location: "file1.txt",
        fingerprint: "0x1111111111111111111111111111111111111111111111111111111111111111",
        size: 1024
      };
      const file2 = {
        location: "file2.txt", 
        fingerprint: "0x2222222222222222222222222222222222222222222222222222222222222222",
        size: 2048
      };

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file1.location,
            file1.fingerprint,
            file1.size,
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            [userApi.alice.publicKey],
            null
          ),
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file2.location,
            file2.fingerprint,
            file2.size,
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Wait for initial indexing
      await sleep(3000);

      // Verify bucket and files are indexed (even though owned by MSP2)
      const bucketsBefore = await sql`
        SELECT * FROM bucket WHERE id = ${bucketId.toString()}
      `;
      assert(bucketsBefore.length === 1, "Bucket should be indexed before transfer");
      assert(bucketsBefore[0].name === bucketName, "Bucket name should match");

      const filesBefore = await sql`
        SELECT * FROM file WHERE bucket_id = ${bucketId.toString()} ORDER BY location
      `;
      assert(filesBefore.length === 2, "Both files should be indexed before transfer");

      // Transfer bucket from MSP2 to MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: userApi.shConsts.NODE_INFOS.msp1.AddressId
            }
          })
        ],
        signer: shUser
      });

      // Wait for transfer to be indexed
      await sleep(3000);

      // Verify bucket ownership is updated
      const bucketsAfter = await sql`
        SELECT b.*, m.onchain_msp_id 
        FROM bucket b
        JOIN msp m ON b.msp_id = m.id
        WHERE b.id = ${bucketId.toString()}
      `;
      assert(bucketsAfter.length === 1, "Bucket should still be indexed after transfer");
      assert(
        bucketsAfter[0].onchain_msp_id === userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "Bucket should now be owned by MSP1"
      );

      // Verify files are still indexed
      const filesAfter = await sql`
        SELECT * FROM file WHERE bucket_id = ${bucketId.toString()} ORDER BY location
      `;
      assert(filesAfter.length === 2, "Both files should remain indexed after transfer");
      assert(filesAfter[0].location === file1.location, "File1 location should match");
      assert(filesAfter[0].fingerprint === file1.fingerprint, "File1 fingerprint should match");
      assert(filesAfter[1].location === file2.location, "File2 location should match");
      assert(filesAfter[1].fingerprint === file2.fingerprint, "File2 fingerprint should match");

      // Verify MoveBucketAccepted event is indexed
      const moveEvents = await sql`
        SELECT * FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'MoveBucketAccepted'
        AND data::text LIKE '%${bucketId.toString()}%'
      `;
      assert(moveEvents.length === 1, "MoveBucketAccepted event should be indexed");
      
      const moveEventData = JSON.parse(moveEvents[0].data);
      assert(moveEventData.bucketId === bucketId.toString(), "Event bucket ID should match");
      assert(moveEventData.newMspId === userApi.shConsts.NODE_INFOS.msp1.AddressId, "New MSP should be MSP1");
      assert(moveEventData.previousMspId === userApi.shConsts.NODE_INFOS.msp2.AddressId, "Previous MSP should be MSP2");
    });

    it("indexes bucket transferred from MSP1 to MSP2", async () => {
      // Create bucket owned by MSP1
      const bucketName = "msp1-bucket-for-transfer";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Add a file to MSP1's bucket
      const file = {
        location: "msp1-file.dat",
        fingerprint: "0x3333333333333333333333333333333333333333333333333333333333333333",
        size: 4096
      };

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file.location,
            file.fingerprint,
            file.size,
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Wait for initial indexing
      await sleep(3000);

      // Transfer bucket from MSP1 to MSP2
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: userApi.shConsts.NODE_INFOS.msp2.AddressId
            }
          })
        ],
        signer: shUser
      });

      // Wait for transfer to be indexed
      await sleep(3000);

      // Verify bucket is still indexed but now owned by MSP2
      const buckets = await sql`
        SELECT b.*, m.onchain_msp_id 
        FROM bucket b
        JOIN msp m ON b.msp_id = m.id
        WHERE b.id = ${bucketId.toString()}
      `;
      assert(buckets.length === 1, "Bucket should remain indexed after transfer");
      assert(
        buckets[0].onchain_msp_id === userApi.shConsts.NODE_INFOS.msp2.AddressId,
        "Bucket should now be owned by MSP2"
      );

      // Verify file is still indexed
      const files = await sql`
        SELECT * FROM file WHERE bucket_id = ${bucketId.toString()}
      `;
      assert(files.length === 1, "File should remain indexed after bucket transfer");
      assert(files[0].location === file.location, "File location should match");
    });

    it("tracks multiple bucket transfers", async () => {
      // Create bucket owned by user
      const bucketName = "multi-transfer-bucket";
      const bucketEvent = await userApi.file.newBucket(bucketName);
      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Transfer: User → MSP2
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: userApi.shConsts.NODE_INFOS.msp2.AddressId
            }
          })
        ],
        signer: shUser
      });

      await sleep(2000);

      // Transfer: MSP2 → MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: userApi.shConsts.NODE_INFOS.msp1.AddressId
            }
          })
        ],
        signer: shUser
      });

      await sleep(2000);

      // Verify complete transfer history
      const moveEvents = await sql`
        SELECT * FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'MoveBucketAccepted'
        AND data::text LIKE '%${bucketId.toString()}%'
        ORDER BY block_number
      `;

      assert(moveEvents.length >= 2, "Should have at least 2 transfer events");

      // Verify final ownership
      const finalBucket = await sql`
        SELECT b.*, m.onchain_msp_id 
        FROM bucket b
        JOIN msp m ON b.msp_id = m.id
        WHERE b.id = ${bucketId.toString()}
      `;
      assert(finalBucket.length === 1, "Bucket should be indexed");
      assert(
        finalBucket[0].onchain_msp_id === userApi.shConsts.NODE_INFOS.msp1.AddressId,
        "Final owner should be MSP1"
      );
    });

    it("indexes MoveBucketRequested events from any MSP", async () => {
      // Create bucket on MSP2
      const bucketName = "request-tracking-bucket";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Request bucket transfer (this might be rejected or expire)
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestMoveBucket(
            bucketId,
            userApi.shConsts.NODE_INFOS.msp1.AddressId
          )
        ],
        signer: shUser
      });

      await sleep(2000);

      // Verify MoveBucketRequested event is indexed
      const requestEvents = await sql`
        SELECT * FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'MoveBucketRequested'
        AND data::text LIKE '%${bucketId.toString()}%'
      `;

      assert(requestEvents.length === 1, "MoveBucketRequested event should be indexed");
      const requestData = JSON.parse(requestEvents[0].data);
      assert(requestData.bucketId === bucketId.toString(), "Request bucket ID should match");
      assert(requestData.requester === userApi.shConsts.NODE_INFOS.msp1.AddressId, "Requester should be MSP1");
    });

    it("maintains database integrity with cross-MSP references", async () => {
      // Verify MSP records exist for referenced MSPs
      const msps = await sql`
        SELECT onchain_msp_id FROM msp
        WHERE onchain_msp_id IN (
          ${userApi.shConsts.NODE_INFOS.msp1.AddressId},
          ${userApi.shConsts.NODE_INFOS.msp2.AddressId}
        )
        ORDER BY onchain_msp_id
      `;

      // At least MSP1 should exist (running the indexer)
      assert(msps.length >= 1, "At least MSP1 should be indexed");
      
      // If buckets were transferred, MSP2 record should also exist
      const bucketsWithMsp2 = await sql`
        SELECT COUNT(*) as count
        FROM bucket b
        JOIN msp m ON b.msp_id = m.id
        WHERE m.onchain_msp_id = ${userApi.shConsts.NODE_INFOS.msp2.AddressId}
      `;

      if (Number(bucketsWithMsp2[0].count) > 0) {
        assert(
          msps.some(m => m.onchain_msp_id === userApi.shConsts.NODE_INFOS.msp2.AddressId),
          "MSP2 record should exist if referenced by buckets"
        );
      }

      console.log("✓ Database integrity maintained with cross-MSP references");
    });
  }
);