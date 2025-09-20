import assert, { strictEqual, notEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent
} from "../../../util";
import { createBucketAndSendNewStorageRequest } from "../../../util/bspNet/fileHelpers";
import {
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";
import { waitForIndexing } from "../../../util/fisherman/indexerTestHelpers";
import { waitForFishermanSync } from "../../../util/fisherman/fishermanHelpers";

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
 * 1. Creates storage request with single replication target (BSP and MSP)
 * 2. BSP volunteers and confirms storage (using whatsup.jpg for automatic volunteering)
 * 3. MSP accepts storage request and confirms storage
 * 4. User sends file deletion request
 * 5. Verifies fisherman indexes all events correctly and processes deletions
 * 6. Verifies both BSP and MSP forest root changes after deletion
 *
 * Note: The user node is running the indexer, so any finalize blocks we seal on the user node, directly affects the data that is being
 * indexed in the database.
 */
await describeMspNet(
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
    createSqlClient,
    createFishermanApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let sql: SqlClient;
    let fileKey: string;
    let bucketId: string;
    let location: string;
    let fingerprint: string;
    let fileSize: number;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      // Ensure fisherman node is ready if available
      if (createFishermanApi) {
        fishermanApi = await createFishermanApi();
        await waitForFishermanSync(userApi, fishermanApi);
      }

      await userApi.rpc.engine.createBlock(true, true);

      await waitForIndexing(userApi);
    });

    it("creates storage request, waits for MSP and BSP to accept and confirm, verifies indexer database", async () => {
      const bucketName = "test-fisherman-deletion";
      const source = "res/whatsup.jpg";
      const destination = "test/fisherman-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const fileMetadata = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        valuePropId,
        mspId,
        1,
        true
      );

      fileKey = fileMetadata.fileKey;
      bucketId = fileMetadata.bucketId;
      location = fileMetadata.location;
      fingerprint = fileMetadata.fingerprint;
      fileSize = fileMetadata.fileSize;

      // Wait for MSP to store the file
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer();
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      await waitForIndexing(userApi);
      await waitForFileIndexed(sql, fileKey);
      await waitForMspFileAssociation(sql, fileKey);
      await waitForBspFileAssociation(sql, fileKey);
    });

    it("user sends file deletion request and fisherman submits delete_file extrinsics", async () => {
      // Create file deletion request
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

      // Create the user signature for the file deletion intention
      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

      // Submit the file deletion request
      const deletionRequestResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketId,
            location,
            fileSize,
            fingerprint
          )
        ],
        signer: shUser
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      await waitForIndexing(userApi, false);

      // Verify delete_file extrinsics are submitted
      await waitFor({
        lambda: async () => {
          const deleteFileMatch = await userApi.assert.extrinsicPresent({
            method: "deleteFile",
            module: "fileSystem",
            checkTxPool: true,
            assertLength: 2
          });
          return deleteFileMatch.length >= 2;
        },
        iterations: 300,
        delay: 100
      });

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(
        userApi,
        "fileSystem",
        "BucketFileDeletionCompleted",
        deletionResult.events
      );
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionCompleted,
        deletionResult.events
      );

      // Verify MSP root changed
      await waitFor({
        lambda: async () => {
          notEqual(
            mspDeletionEvent.data.oldRoot.toString(),
            mspDeletionEvent.data.newRoot.toString(),
            "MSP forest root should have changed after file deletion"
          );
          const currentBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(
            mspDeletionEvent.data.bucketId.toString()
          );
          strictEqual(
            currentBucketRoot.toString(),
            mspDeletionEvent.data.newRoot.toString(),
            "Current bucket forest root should match the new root from deletion event"
          );
          return true;
        }
      });

      // Verify BSP root changed
      await waitFor({
        lambda: async () => {
          notEqual(
            bspDeletionEvent.data.oldRoot.toString(),
            bspDeletionEvent.data.newRoot.toString(),
            "BSP forest root should have changed after file deletion"
          );
          const currentBspRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
          strictEqual(
            currentBspRoot.toString(),
            bspDeletionEvent.data.newRoot.toString(),
            "Current BSP forest root should match the new root from deletion event"
          );
          return true;
        }
      });
    });
  }
);
