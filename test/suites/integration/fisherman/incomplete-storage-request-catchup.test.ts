import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent,
  mspKey,
  sleep,
  ShConsts
} from "../../../util";

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
await describeMspNet(
  "Fisherman Incomplete Storage Requests with Catchup",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api, createFishermanApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");

      // Wait for user node to be ready
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");
      fishermanApi = await createFishermanApi();

      await userApi.block.seal({ finaliseBlock: true });
    });

    it("processes expired request (BSP only) in unfinalized block", async () => {
      const bucketName = "test-expired-bsp-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/expired-bsp.txt";

      // Pause MSP container to ensure only BSP accepts
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      try {
        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          null,
          ShConsts.DUMMY_MSP_ID,
          shUser,
          1,
          false
        );

        const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
        assert(storageRequest.isSome);
        const expiresAt = storageRequest.unwrap().expiresAt.toNumber();

        // Wait for BSP to volunteer and store
        await userApi.wait.bspVolunteerInTxPool(undefined);
        // Seal unfinalized block
        await userApi.block.seal({ finaliseBlock: false });

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

        const incompleteStorageRequestResult = await userApi.block.skipTo(expiresAt, {
          finalised: false
        });

        assertEventPresent(
          userApi,
          "fileSystem",
          "IncompleteStorageRequest",
          incompleteStorageRequestResult.events
        );

        const incompleteStorageRequests =
          await userApi.query.fileSystem.incompleteStorageRequests.entries();
        const maybeIncompleteStorageRequest = incompleteStorageRequests[0];
        assert(maybeIncompleteStorageRequest !== undefined);
        assert(maybeIncompleteStorageRequest[1].isSome);
        const incompleteStorageRequest = maybeIncompleteStorageRequest[1].unwrap();
        assert(incompleteStorageRequest.pendingBspRemovals.length === 1);
        assert(incompleteStorageRequest.pendingBucketRemoval.isFalse);

        await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

        // Wait for fisherman to process incomplete storage deletions and verify extrinsics are in tx pool
        await userApi.fisherman.waitForBatchDeletions({
          deletionType: "Incomplete",
          expectExt: 1, // 1 BSP only (MSP did not accept)
          sealBlock: false // Seal manually to capture events
        });

        // Seal block to process the extrinsic
        const deletionResult = await userApi.block.seal();

        // Verify BspFileDeletionsCompleted event
        assertEventPresent(
          userApi,
          "fileSystem",
          "BspFileDeletionsCompleted",
          deletionResult.events
        );
      } finally {
        // Always resume MSP container even if test fails
        await userApi.docker.resumeContainer({
          containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
        });
        await userApi.docker.waitForLog({
          searchString: "💤 Idle",
          containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
        });
        await sleep(3000);
      }
    });

    it("processes revoked request (MSP and BSP) in unfinalized block", async () => {
      const bucketName = "test-revoked-catchup";
      const source = "res/smile.jpg";
      const destination = "test/revoked-catchup.txt";

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        2, // Keep the storage request opened to be able to revoke
        false
      );

      await userApi.wait.mspResponseInTxPool(1);

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteerInTxPool(undefined);
      // Seal unfinalized block
      await userApi.block.seal({ finaliseBlock: false });

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

      // Wait for fisherman to process incomplete storage deletions and verify extrinsics are in tx pool
      await userApi.fisherman.waitForBatchDeletions({
        deletionType: "Incomplete",
        expectExt: 2, // 1 BSP + 1 Bucket
        sealBlock: false // Seal manually to capture events
      });

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(
        userApi,
        "fileSystem",
        "BucketFileDeletionsCompleted",
        deletionResult.events
      );
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionsCompleted", deletionResult.events);
    });

    it("processes MSP stop storing bucket with incomplete request in unfinalized block", async () => {
      const bucketName = "test-msp-stop-incomplete-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/msp-stop-incomplete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      // Get value proposition for MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        2, // Keep the storage request opened to be able to revoke
        false
      );

      // Wait for MSP to accept storage request
      await userApi.wait.mspResponseInTxPool(1);

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteerInTxPool(undefined);
      // Seal unfinalized block
      await userApi.block.seal({ finaliseBlock: false });

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

      // Wait for fisherman to process incomplete storage deletions and verify extrinsics are in tx pool
      await userApi.fisherman.waitForBatchDeletions({
        deletionType: "Incomplete",
        expectExt: 2, // 1 BSP + 1 Bucket (no MSP present)
        sealBlock: false // Seal manually to capture events
      });

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(
        userApi,
        "fileSystem",
        "BucketFileDeletionsCompleted",
        deletionResult.events
      );
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionsCompleted", deletionResult.events);

      // Verify BucketFileDeletionsCompleted event with no MSP ID
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionsCompleted,
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
