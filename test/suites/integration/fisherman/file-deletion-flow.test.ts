import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
  waitFor,
  ShConsts,
  assertEventPresent
} from "../../../util";
import { waitForDeleteFileExtrinsic } from "../../../util/fisherman/fishermanHelpers";
import type { H256 } from "@polkadot/types/interfaces";
import { waitForIndexing } from "../../../util/fisherman/indexerTestHelpers";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";

/**
 * FISHERMAN FILE DELETION FLOW - BASIC HAPPY PATH
 *
 * Purpose: Tests the standard, straightforward file deletion workflow using finalized blocks.
 *          This is the foundation test for fisherman file deletion functionality.
 *
 * What makes this test unique:
 * - Uses finalized blocks throughout (standard blockchain behavior)
 * - Tests basic file storage and deletion workflow step-by-step
 * - Creates storage requests with single replication target
 * - Focuses on core functionality without edge cases or complex scenarios
 *
 * Test Scenario:
 * 1. Creates storage request with single replication target (MSP only initially)
 * 2. BSP volunteers and confirms storage (using whatsup.jpg for automatic volunteering)
 * 3. MSP accepts storage request and confirms storage
 * 4. User sends file deletion request
 * 5. Verifies fisherman indexes all events correctly
 */
describeMspNet(
  "Fisherman File Deletion Flow",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createFishermanApi,
    createSqlClient
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let sql: SqlClient;
    let bucketId: H256;
    let fileKey: H256;
    let fileLocation: string;
    let fileFingerprint: H256;
    let fileSize: number;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for fisherman node to be ready
      await userApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "💤 Idle",
        timeout: 30000
      });

      // Create fisherman and MSP APIs
      assert(createFishermanApi, "Fisherman API should be available when fisherman is enabled");
      fishermanApi = (await createFishermanApi()) as EnrichedBspApi;
      assert(fishermanApi, "Fisherman API should be created successfully");

      // Wait for fisherman node to be ready and connected to database
      await fishermanApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "💤 Idle",
        timeout: 15000
      });

      assert(createMsp1Api, "MSP1 API should be available");
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      // Initialize blockchain state
      await userApi.rpc.engine.createBlock(true, true);
      await sleep(1000);
      await userApi.block.seal();
      await userApi.block.seal();
    });

    it("creates storage request with single replication target and indexes events", async () => {
      const bucketName = "test-deletion-bucket";
      const source = "res/smile.jpg";
      const destination = "test/file-to-delete.txt";

      // Create bucket
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      assert(newBucketEventData, "NewBucket event data not found");
      bucketId = newBucketEventData.bucketId;

      // Load file
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHex,
        bucketId
      );

      // Issue storage request with single replication target (MSP only)
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: shUser
      });

      // Get file key from event
      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
      const eventData = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      fileKey = eventData.fileKey;

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify file is indexed
      const files = await sql`
        SELECT * FROM file
        WHERE bucket_id = (
          SELECT id FROM bucket WHERE name = ${bucketName}
        )
      `;

      assert.equal(files.length, 1);
      const dbFileKey = `0x${files[0].file_key.toString("hex")}`;
      assert.equal(dbFileKey, fileKey);
    });

    it("BSP confirms storage and fisherman indexes the event", async () => {
      // Use a file that matches DUMMY_BSP_ID for automatic volunteering
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-stored-file.txt";

      // Load file for BSP storage
      const ownerHexBsp = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHexBsp,
        bucketId
      );

      // Issue storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: shUser
      });

      // Get the file key
      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
      const eventData = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const bspFileKey = eventData.fileKey;

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(bspFileKey)).isFileFound
      });

      // Wait for BSP to confirm storage
      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Wait for the indexer to process the events
      await waitFor({
        lambda: async () => {
          const files = await sql`
            SELECT * FROM file WHERE file_key = ${bspFileKey}
          `;
          return files.length > 0;
        }
      });

      // Verify BSP-file association is indexed
      const bspFiles = await sql`
        SELECT * FROM bsp_file
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${bspFileKey}
        )
      `;

      assert(bspFiles.length > 0, "BSP file association should be indexed");

      // Update our test file key and metadata to use this one for MSP and deletion tests
      fileKey = bspFileKey;
      fileLocation = location;
      fileFingerprint = fingerprint;
      fileSize = file_size;
    });

    it("MSP accepts storage request and fisherman indexes the event", async () => {
      // Wait for MSP to receive the file
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for MSP to accept the storage request
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Get the MspAcceptedStorageRequest event
      const { event: mspAcceptedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const mspAcceptedEventDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedEvent) &&
        mspAcceptedEvent.data;

      assert(
        mspAcceptedEventDataBlob,
        "MspAcceptedStorageRequest event data does not match expected type"
      );

      const acceptedFileKey = mspAcceptedEventDataBlob.fileKey.toString();
      assert.equal(acceptedFileKey, fileKey);

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Wait for the indexer to process the events
      await waitFor({
        lambda: async () => {
          const files = await sql`
            SELECT * FROM file WHERE file_key = ${fileKey}
          `;
          return files.length > 0;
        }
      });

      // Verify MSP-file association is indexed
      const mspFiles = await sql`
        SELECT * FROM msp_file
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey}
        )
      `;

      assert(mspFiles.length > 0, "MSP file association should be indexed");
    });

    it("user sends file deletion request and fisherman indexes it", async () => {
      // Ensure file is in MSP's forest storage before deletion attempt
      await waitFor({
        lambda: async () => {
          const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
            bucketId.toString(),
            fileKey.toString()
          );
          return isFileInForest.isTrue;
        }
      });

      // Create file operation intention for deletion
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

      // Create signature for the intention - encode the object
      const intentionType = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const encodedIntention = intentionType.toHex();
      const rawSignature = shUser.sign(encodedIntention);

      // Create the signature object with Sr25519 variant
      const signature = {
        Sr25519: rawSignature
      };

      // Submit file deletion request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            signature,
            bucketId,
            fileLocation,
            fileSize,
            fileFingerprint
          )
        ],
        signer: shUser
      });

      // Verify FileDeletionRequested event
      const { event: deletionEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested"
      );

      const deletionEventData =
        userApi.events.fileSystem.FileDeletionRequested.is(deletionEvent) && deletionEvent.data;

      assert(deletionEventData, "FileDeletionRequested event data not found");
      // The event data contains signedDeleteIntention which has the fileKey
      const eventFileKey = deletionEventData.signedDeleteIntention.fileKey;
      assert.equal(eventFileKey.toString(), fileKey.toString());

      // Wait for indexing to process the deletion request
      // Don't seal a new block since we already sealed the deletion request above
      await waitForIndexing(userApi, false);

      // Verify delete_file extrinsics are submitted (should be 2: one for BSP and one for MSP)
      const deleteFileFound = await waitForDeleteFileExtrinsic(fishermanApi, 2);
      assert(
        deleteFileFound,
        "Should find 2 delete_file extrinsics in transaction pool (BSP and MSP)"
      );

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify deletion completion events
      assertEventPresent(userApi, "fileSystem", "MspFileDeletionCompleted", deletionResult.events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);

      console.log(
        "✓ Fisherman successfully submitted delete_file extrinsics and deletion completed"
      );
    });
  }
);
