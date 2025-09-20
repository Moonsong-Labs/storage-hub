import assert, { strictEqual, notEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
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
import {
  waitForFishermanProcessing,
  waitForFishermanReady
} from "../../../util/fisherman/fishermanHelpers";

/**
 * FISHERMAN FILE DELETION FLOW WITH CATCHUP
 *
 * Purpose: Tests the fisherman's ability to process file deletion events from UNFINALIZED blocks
 *          during blockchain catchup scenarios.
 *
 * What makes this test unique:
 * - Creates unfinalized blocks with blockchain activity (transfers)
 * - Sends file deletion requests in unfinalized blocks (finaliseBlock: false)
 * - Tests fisherman indexer's catchup mechanism when processing events from non-finalized portions
 * - Verifies the gap between finalized head and current head during processing
 *
 * Test Scenario:
 * 1. Sets up file storage with both BSP and MSP confirming storage
 * 2. Creates 3 unfinalized blocks with transfer activity
 * 3. Sends file deletion request in an unfinalized block
 * 4. Verifies fisherman can index and process events from unfinalized blocks
 */
await describeMspNet(
  "Fisherman File Deletion Flow with Catchup",
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

    // Track files created in unfinalized blocks
    const unfinalizedFiles: Array<{
      fileKey: string;
      bucketId: string;
      location: string;
      fingerprint: string;
      fileSize: number;
    }> = [];

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      // Ensure fisherman node is ready if available
      if (createFishermanApi) {
        fishermanApi = await createFishermanApi();
        await waitForFishermanReady(userApi, fishermanApi);
      }

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);

      await waitForIndexing(userApi);
    });

    it("creates finalized block with storage request and BSP & MSP confirming", async () => {
      const bucketName = "test-deletion-catchup-bucket";
      const source = "res/whatsup.jpg";
      const destination = "test/file-to-delete-catchup.txt";
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

    it("creates 3 unfinalized blocks with storage requests and MSP & BSP confirmations", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create 3 unfinalized blocks, each with a storage request
      for (let i = 0; i < 3; i++) {
        const bucketName = `test-deletion-catchup-bucket-${i}`;
        const source = "res/whatsup.jpg";
        const destination = `test/file-to-delete-catchup-${i}.txt`;

        const fileMetadata = await createBucketAndSendNewStorageRequest(
          userApi,
          source,
          destination,
          bucketName,
          null,
          valuePropId,
          mspId,
          1,
          false
        );

        // Store file metadata for later use
        unfinalizedFiles.push(fileMetadata);

        // Wait for MSP to store the file
        await waitFor({
          lambda: async () =>
            (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey))
              .isFileFound
        });

        await userApi.wait.mspResponseInTxPool();

        // Wait for BSP to volunteer and store
        await userApi.wait.bspVolunteer();
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey))
              .isFileFound
        });

        const bspAddress = userApi.createType("Address", bspKey.address);
        await userApi.wait.bspStored({
          expectedExts: 1,
          sealBlock: false, // Don't seal/finalize the block
          bspAccount: bspAddress
        });

        await userApi.block.seal({
          finaliseBlock: false
        });

        // Add a small delay between blocks
        await sleep(500);
      }

      // Verify blocks were created but not finalized
      const finalizedHead = await userApi.rpc.chain.getFinalizedHead();
      const currentHead = await userApi.rpc.chain.getHeader();

      // There should be a gap between finalized and current head
      assert(
        currentHead.number.toNumber() >
          (await userApi.rpc.chain.getHeader(finalizedHead)).number.toNumber(),
        "Current head should be ahead of finalized head"
      );

      // Verify we created 3 files
      assert.equal(unfinalizedFiles.length, 3, "Should have created 3 files in unfinalized blocks");
    });

    it("sends file deletion request in unfinalized block and verifies fisherman processes with ephemeral trie", async () => {
      // Use the first file created in unfinalized blocks for deletion
      assert(unfinalizedFiles.length > 0, "Should have files created in unfinalized blocks");
      const fileToDelete = unfinalizedFiles[0];

      // NOTE: We don't wait for indexing here because the files are in unfinalized blocks
      // The fisherman indexer won't have these files in its database, but it will:
      // 1. Build an ephemeral trie from indexed (finalized) data
      // 2. Query unfinalized blocks for additional files
      // 3. Add unfinalized files to the ephemeral trie
      // 4. Create proof of inclusion for the deletion

      // Ensure file is in MSP's forest storage before deletion attempt
      // The providers store files regardless of block finalization
      await waitFor({
        lambda: async () => {
          const isFileInForest = await msp1Api.rpc.storagehubclient.isFileInForest(
            fileToDelete.bucketId.toString(),
            fileToDelete.fileKey.toString()
          );
          return isFileInForest.isTrue;
        }
      });

      // Create file operation intention for deletion
      const fileOperationIntention = {
        fileKey: fileToDelete.fileKey,
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

      // Submit file deletion request in an unfinalized block
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            fileToDelete.bucketId,
            fileToDelete.location,
            fileToDelete.fileSize,
            fileToDelete.fingerprint
          )
        ],
        signer: shUser,
        finaliseBlock: false
      });

      // Verify FileDeletionRequested event
      const { event: deletionEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested"
      );

      const deletionEventData =
        userApi.events.fileSystem.FileDeletionRequested.is(deletionEvent) && deletionEvent.data;

      assert(deletionEventData, "FileDeletionRequested event data not found");
      const eventFileKey = deletionEventData.signedDeleteIntention.fileKey;
      assert.equal(eventFileKey.toString(), fileToDelete.fileKey.toString());

      // Verify fisherman processes the FileDeletionRequested event even from unfinalized blocks
      const processingFound = await waitForFishermanProcessing(
        userApi,
        `Processing file deletion request for signed intention file key: ${fileToDelete.fileKey}`
      );
      assert(processingFound, "Should find fisherman processing log even from unfinalized blocks");

      // Verify delete_file extrinsics are submitted (should be 2: one for BSP and one for MSP)
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

      // Now finalize the blocks to process the extrinsics
      const { events } = await userApi.block.seal();

      assertEventPresent(userApi, "fileSystem", "BucketFileDeletionCompleted", events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", events);

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionCompleted,
        events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionCompleted,
        events
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
