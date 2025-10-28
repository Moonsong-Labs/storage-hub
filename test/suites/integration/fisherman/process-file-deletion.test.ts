import assert, { strictEqual, notEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent,
  mspKey,
  ShConsts
} from "../../../util";
import {
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";
import {
  waitForIndexing,
  waitForFishermanBatchDeletions
} from "../../../util/fisherman/indexerTestHelpers";

/**
 * FISHERMAN PROCESS FILE DELETION - COMPREHENSIVE EVENT PROCESSING
 *
 * Purpose: Tests the fisherman's comprehensive event processing capabilities for various
 *          file deletion scenarios and edge cases.
 *
 * What makes this test unique:
 * - Tests MULTIPLE types of deletion-related events:
 *   * FileDeletionRequested - direct user deletion requests
 *   * StorageRequestExpired - cleanup of expired storage requests
 *   * StorageRequestRevoked - cleanup of user-revoked requests
 *   * StorageRequestRejected - cleanup of provider-rejected requests
 * - Tests multiple provider scenarios (both BSP and MSP for same file)
 * - Includes extensive log verification for fisherman processing
 * - Uses container pausing/resuming to simulate network conditions
 * - Tests fisherman's preparation of delete_files extrinsics
 *
 * Test Scenarios:
 * 1. FileDeletionRequested: Normal user-initiated deletion with multiple providers
 * 2. StorageRequestExpired: Paused providers causing expiration, fisherman cleanup
 * 3. StorageRequestRevoked: User revokes request before acceptance, fisherman cleanup
 * 4. Multiple providers: File stored by both BSP and MSP, deletion affects both
 * 5. StorageRequestRejected: Provider rejection scenarios (placeholder for future)
 */
await describeMspNet(
  "Fisherman Process File Deletion",
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

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");
      fishermanApi = await createFishermanApi();

      await userApi.block.seal({ finaliseBlock: true });
      await waitForIndexing(userApi);
    });

    it("processes FileDeletionRequested event and prepares delete_files extrinsic", async () => {
      const bucketName = "test-fisherman-deletion";
      const source = "res/smile.jpg";
      const destination = "test/fisherman-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          valuePropId,
          ShConsts.DUMMY_MSP_ID,
          shUser,
          1,
          true
        );

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

      // Wait for fisherman to process user deletions
      await waitForFishermanBatchDeletions(userApi, "User");

      // Verify delete_files extrinsics are submitted
      await userApi.assert.extrinsicPresent({
        method: "deleteFiles",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2,
        timeout: 30000
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

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionsCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
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

    it("processes expired storage request when MSP doesn't accept in time", async () => {
      const bucketName = "test-fisherman-expired";
      const source = "res/whatsup.jpg";
      const destination = "test/expired.txt";

      // Pause MSP containers to prevent them from accepting the storage request
      // We don't pause the BSP so that it confirms the storage request so that when we reach
      // the expired block, the storage request will be moved to incomplete.
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

      try {
        const tickRangeToMaximumThreshold = (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              TickRangeToMaximumThreshold: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asTickRangeToMaximumThreshold.toNumber();

        const storageRequestTtlRuntimeParameter = {
          RuntimeConfig: {
            StorageRequestTtl: [null, tickRangeToMaximumThreshold]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(storageRequestTtlRuntimeParameter)
            )
          ]
        });

        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          null,
          ShConsts.DUMMY_MSP_ID,
          shUser,
          1,
          true
        );

        const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
        assert(storageRequest.isSome);
        const expiresAt = storageRequest.unwrap().expiresAt.toNumber();

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

        const incompleteStorageRequestResult = await userApi.block.skipTo(expiresAt);

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

        await waitForIndexing(userApi, false);
        await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

        // Wait for fisherman to process incomplete storage deletions
        await waitForFishermanBatchDeletions(userApi, "Incomplete");

        // Verify delete_files_for_incomplete_storage_request extrinsic is submitted
        await userApi.assert.extrinsicPresent({
          method: "deleteFilesForIncompleteStorageRequest",
          module: "fileSystem",
          checkTxPool: true,
          assertLength: 1,
          timeout: 30000
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
        // Resume containers for cleanup - always execute
        await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
      }
    });

    it("processes revoked storage request and prepares deletion", async () => {
      const bucketName = "test-fisherman-revoked";
      const source = "res/smile.jpg";
      const destination = "test/revoked.txt";

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        2, // Keep the storage request opened to be able to revoke
        true
      );

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

      // Revoke the storage request
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      // Do not seal block
      await waitForIndexing(userApi, false);

      // Wait for fisherman to process incomplete storage deletions
      await waitForFishermanBatchDeletions(userApi, "Incomplete");

      // Verify 2 extrsinsics submitted for each MSP and BSP
      await userApi.assert.extrinsicPresent({
        method: "deleteFilesForIncompleteStorageRequest",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2,
        timeout: 30000
      });

      // Seal block to process the extrinsic
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(
        userApi,
        "fileSystem",
        "BucketFileDeletionsCompleted",
        deletionResult.events
      );
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionsCompleted", deletionResult.events);

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionsCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
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

    it("processes multiple providers for same file deletion", async () => {
      const bucketName = "test-fisherman-multiple";
      const source = "res/whatsup.jpg";
      const destination = "test/multiple.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          valuePropId,
          ShConsts.DUMMY_MSP_ID,
          shUser,
          1,
          true
        );

      // Wait for both MSP and BSP to store the file
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();
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

      // Create and submit file deletion request
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

      // Wait for fisherman to process user deletions
      await waitForFishermanBatchDeletions(userApi, "User");

      // Verify TWO delete_files extrinsics are submitted (one for BSP and one for MSP)
      await userApi.assert.extrinsicPresent({
        method: "deleteFiles",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2,
        timeout: 30000
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

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionsCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
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

    it("processes MSP stop storing bucket during incomplete storage request", async () => {
      const bucketName = "test-msp-stop-incomplete";
      const source = "res/smile.jpg";
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
        true
      );

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

      // Get initial bucket root for comparison
      const initialBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(
        bucketId.toString()
      );
      assert(initialBucketRoot.isSome, "Initial bucket root should exist");

      // MSP stops storing the bucket (while incomplete request exists)
      const stopStoringResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.mspStopStoringBucket(bucketId)],
        signer: mspKey
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "MspStoppedStoringBucket",
        stopStoringResult.events
      );

      // Revoke the storage request to create incomplete state
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      assertEventPresent(
        userApi,
        "fileSystem",
        "IncompleteStorageRequest",
        revokeStorageRequestResult.events
      );

      // Check that the bucket no longer has an MSP
      const bucketMsp = (await userApi.query.providers.buckets(bucketId)).unwrap().mspId;
      assert(bucketMsp.isNone, "Bucket should have no MSP after stop storing");

      await waitForIndexing(userApi, false);

      // Wait for fisherman to process incomplete storage deletions
      await waitForFishermanBatchDeletions(userApi, "Incomplete");

      // Verify 2 delete extrinsics are submitted (bucket and BSP)
      await userApi.assert.extrinsicPresent({
        method: "deleteFilesForIncompleteStorageRequest",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2,
        timeout: 30000
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

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionsCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
        deletionResult.events
      );

      // Verify MSP deletion event has no MSP ID
      assert(mspDeletionEvent.data.mspId.isNone, "MSP ID should be None since bucket has no MSP");

      // Verify bucket root changed (even without MSP)
      notEqual(
        mspDeletionEvent.data.oldRoot.toString(),
        mspDeletionEvent.data.newRoot.toString(),
        "Bucket forest root should have changed after file deletion"
      );

      // Verify BSP root changed
      notEqual(
        bspDeletionEvent.data.oldRoot.toString(),
        bspDeletionEvent.data.newRoot.toString(),
        "BSP forest root should have changed after file deletion"
      );

      // Verify current BSP root matches event
      await waitFor({
        lambda: async () => {
          const currentBspRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
          strictEqual(
            currentBspRoot.toString(),
            bspDeletionEvent.data.newRoot.toString(),
            "Current BSP forest root should match the new root from deletion event"
          );
          return true;
        }
      });

      // Verify the incomplete storage request has been fully processed
      const incompleteRequest = await userApi.query.fileSystem.incompleteStorageRequests(fileKey);
      assert(
        incompleteRequest.isNone,
        "Incomplete storage request should be removed after all providers deleted"
      );
    });
  }
);
