import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent,
  assertEventMany,
  mspKey,
  sleep
} from "../../../util";
import { createBucketAndSendNewStorageRequest } from "../../../util/bspNet/fileHelpers";
import { waitForIncompleteStorageRequestExtrinsic } from "../../../util/fisherman/fishermanHelpers";

/**
 * FISHERMAN INCOMPLETE STORAGE REQUESTS WITH CATCHUP
 *
 * Purpose: Tests the fisherman's ability to process incomplete storage request events
 *          (Expired, Revoked) from UNFINALIZED blocks during blockchain catchup scenarios.
 *
 * What makes this test unique:
 * - Creates incomplete storage request scenarios (expired, revoked) in unfinalized blocks.
 * - Tests fisherman indexer's catchup mechanism for these specific events.
 * - Verifies that the fisherman correctly identifies which providers (MSP, BSP, or both)
 *   need to perform a deletion and submits the appropriate extrinsics.
 */
describeMspNet(
  "Fisherman Incomplete Storage Requests with Catchup",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();

      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);
    });

    // TODO: fix race condition somewhere (fails in CI)
    it.skip("processes expired request (BSP only) in unfinalized block", async () => {
      const bucketName = "test-expired-bsp-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/expired-bsp.txt";

      // Pause MSP container to ensure only BSP accepts
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1,
        false // Do not finalize
      );

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(undefined, false);
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        finalizeBlock: false
      });

      // Skip ahead to trigger expiration
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const storageRequestTtl = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            StorageRequestTtl: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asStorageRequestTtl.toNumber();

      await userApi.block.skipTo(currentBlockNumber + storageRequestTtl, { finalised: false });

      // Verify only one delete extrinsic is submitted (for the BSP)
      const deleteIncompleteFileFound = await waitForIncompleteStorageRequestExtrinsic(
        userApi,
        1,
        30000
      );
      assert(
        deleteIncompleteFileFound,
        "Should find 1 delete_file_for_incomplete_storage_request extrinsic in transaction pool"
      );

      // Seal block to process the extrinsic
      const deletionResult = await userApi.block.seal();

      // Verify FileDeletedFromIncompleteStorageRequest event
      assertEventPresent(
        userApi,
        "fileSystem",
        "FileDeletedFromIncompleteStorageRequest",
        deletionResult.events
      );

      // Resume MSP container
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-msp-1"
      });
      await sleep(3000);
    });

    it("processes revoked request (MSP and BSP) in unfinalized block", async () => {
      const bucketName = "test-revoked-catchup";
      const source = "res/smile.jpg";
      const destination = "test/revoked-catchup.txt";

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        2, // Keep the storage request opened to be able to revoke
        false // Do not finalize
      );

      await userApi.wait.mspResponseInTxPool(1);

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(undefined, false);
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        finalizeBlock: false
      });

      // Revoke the storage request in an unfinalized block
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
        finaliseBlock: false
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      // Verify two delete extrinsics are submitted (for MSP and BSP)
      const deleteIncompleteFileFound = await waitForIncompleteStorageRequestExtrinsic(
        userApi,
        2,
        30000
      );
      assert(
        deleteIncompleteFileFound,
        "Should find 2 delete_file_for_incomplete_storage_request extrinsics in transaction pool"
      );

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify FileDeletedFromIncompleteStorageRequest events
      assertEventMany(
        userApi,
        "fileSystem",
        "FileDeletedFromIncompleteStorageRequest",
        deletionResult.events
      );
    });

    it("processes MSP stop storing bucket with incomplete request in unfinalized block", async () => {
      const bucketName = "test-msp-stop-incomplete-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/msp-stop-incomplete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      // Get value proposition for MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        valuePropId,
        mspId,
        null,
        2, // Keep the storage request opened to be able to revoke
        false // Do not finalize
      );

      // Wait for MSP to accept storage request
      await userApi.wait.mspResponseInTxPool(1);

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(undefined, false);
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        finalizeBlock: false
      });

      // MSP stops storing the bucket before revoke storage request so the incomplete storage request will have
      // no MSP storing the bucket at the time of file deletion
      const stopStoringResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.mspStopStoringBucket(bucketId)],
        signer: mspKey,
        finaliseBlock: false
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "MspStoppedStoringBucket",
        stopStoringResult.events
      );

      // Revoke the storage request to create incomplete state
      const revokeResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
        finaliseBlock: false
      });

      assertEventPresent(userApi, "fileSystem", "StorageRequestRevoked", revokeResult.events);
      assertEventPresent(userApi, "fileSystem", "IncompleteStorageRequest", revokeResult.events);

      // Verify two delete extrinsics are submitted:
      // 1. For the bucket (no MSP present)
      // 2. For the BSP
      const deleteIncompleteFileFound = await waitForIncompleteStorageRequestExtrinsic(
        userApi,
        2,
        30000
      );
      assert(
        deleteIncompleteFileFound,
        "Should find 2 delete_file_for_incomplete_storage_request extrinsics in transaction pool"
      );

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify FileDeletedFromIncompleteStorageRequest events
      assertEventMany(
        userApi,
        "fileSystem",
        "FileDeletedFromIncompleteStorageRequest",
        deletionResult.events
      );

      // Verify BucketFileDeletionCompleted event with no MSP ID
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionCompleted,
        deletionResult.events
      );

      // Verify that msp_id is None in the deletion event
      assert(mspDeletionEvent.data.mspId.isNone, "MSP ID should be None since bucket has no MSP");

      // Verify bucket root changed
      assert(
        mspDeletionEvent.data.oldRoot.toString() !== mspDeletionEvent.data.newRoot.toString(),
        "Bucket root should have changed after file deletion"
      );
    });
  }
);
