import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  ShConsts
} from "../../../util";

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
 * - Tests multiple provider scenarios (both BSP and MSP for same file)
 * - Uses container pausing/resuming to simulate network conditions
 * - Tests fisherman's preparation of delete_files extrinsics
 * - Verifies database state and forest root updates after deletions
 *
 * Test Scenarios:
 * 1. FileDeletionRequested: Normal user-initiated deletion with multiple providers (BSP + MSP)
 * 2. StorageRequestExpired: MSP paused causing expiration, fisherman cleanup (BSP only)
 * 3. StorageRequestRevoked: User revokes request after BSP acceptance, fisherman cleanup (BSP + MSP)
 * 4. Multiple providers: File stored by both BSP and MSP, deletion affects both forests
 */
await describeMspNet(
  "Fisherman Process File Deletion",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createFishermanApi,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
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
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");
      fishermanApi = await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
      await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
      await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });

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

      await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
      });
    });

    it("processes expired storage request when MSP doesn't accept in time", async () => {
      const bucketName = "test-fisherman-expired";
      const source = "res/whatsup.jpg";
      const destination = "test/expired.txt";

      // Pause MSP containers to prevent them from accepting the storage request
      // We don't pause the BSP so that it confirms the storage request so that when we reach
      // the expired block, the storage request will be moved to incomplete.
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

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

      await userApi.assert.eventPresent(
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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
      await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 1, // 1 BSP only (MSP paused)
        userApi,
        bspApi,
        expectedBspCount: 1,
        maxRetries: 3
      });

      // Resume containers for cleanup - always execute
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      await msp1Api.wait.nodeCatchUpToChainTip(userApi);
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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Revoke the storage request in a finalized block
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
        finaliseBlock: true
      });

      await userApi.assert.eventPresent(
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      // Wait for indexer to process the finalized revocation event
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
      await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
      await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });

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
        signer: shUser,
        finaliseBlock: true
      });

      await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
      });
    });
  }
);
