import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser } from "../../../util";

/**
 * FISHERMAN RESTART PENDING DELETIONS INTEGRATION TESTS
 *
 * Validates fisherman resilience by ensuring pending deletion requests (both user-requested
 * deletions and incomplete storage requests) are properly processed after a full Docker
 * container restart using the retry API helper function.
 *
 * Test Flow Pattern:
 * 1. Create and index files
 * 2. Pause fisherman container (prevents processing)
 * 3. Submit deletion/revocation requests (creates pending work)
 * 4. Disconnect API and restart container (full restart from scratch)
 * 5. Verify fisherman picks up and processes pending deletions using retry helper
 *
 * Test 1: User-Requested Deletions After Restart
 * - Setup: 3 buckets × 2 files = 6 files stored with BSP and MSP
 * - Flow: Pause fisherman → Submit `requestDeleteFile` extrinsics → Restart fisherman
 * - Fisherman submits after restart: 1 `deleteFiles` for BSP (6 files) + 3 `deleteFiles` for buckets (2 each)
 * - Events: `FileDeletionRequested`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Database signatures, forest root updates, batch grouping after restart
 *
 * Test 2: Incomplete Storage Requests After Restart
 * - Setup: 3 buckets × 2 files = 6 files with replicationTarget: 2 (keeps storage request alive)
 * - Flow: Pause fisherman → Revoke via `revokeStorageRequest` → Restart fisherman
 * - Fisherman submits after restart: 1 `deleteFilesForIncompleteStorageRequest` for BSP + 3 for buckets
 * - Events: `StorageRequestRevoked`, `IncompleteStorageRequest`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Incomplete storage cleanup, forest root updates after restart
 *
 * Key Implementation Details:
 * - Pausing fisherman before deletions ensures work is truly pending at restart time
 * - API must be disconnected before restart to prevent test runner hang
 * - Restart command handles unpausing and starting container from scratch
 * - Retry helper with maxRetries=3 handles timing issues and ForestProofVerificationFailed errors
 */
await describeMspNet(
  "Fisherman Restart Pending Deletions",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    logLevel: "debug"
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
      assert(
        createFishermanApi,
        "Fisherman API not available. Ensure `fisherman` is set to `true` in the network configuration."
      );
      fishermanApi = await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("processes user-requested file deletions after fisherman container restart", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create 3 buckets with 2 files each (6 files total)
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b0-f0.txt",
            bucketIdOrName: "test-restart-user-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b0-f1.txt",
            bucketIdOrName: "test-restart-user-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b1-f0.txt",
            bucketIdOrName: "test-restart-user-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b1-f1.txt",
            bucketIdOrName: "test-restart-user-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b2-f0.txt",
            bucketIdOrName: "test-restart-user-bucket-2",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/restart-user-b2-f1.txt",
            bucketIdOrName: "test-restart-user-bucket-2",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApis: [bspApi],
        mspApi: msp1Api
      });

      const { fileKeys, bucketIds, locations, fingerprints, fileSizes } = batchResult;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
        await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
        await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
      }

      // Pause fisherman to prevent it from processing deletions before restart
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // Build all deletion request calls
      const deletionCalls = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const fileOperationIntention = {
          fileKey: fileKeys[i],
          operation: { Delete: null }
        };

        const intentionCodec = userApi.createType(
          "PalletFileSystemFileOperationIntention",
          fileOperationIntention
        );
        const intentionPayload = intentionCodec.toU8a();
        const rawSignature = shUser.sign(intentionPayload);
        const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

        deletionCalls.push(
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketIds[i],
            locations[i],
            fileSizes[i],
            fingerprints[i]
          )
        );
      }

      // Seal a single block with all deletion requests
      const deletionRequestResult = await userApi.block.seal({
        calls: deletionCalls,
        signer: shUser
      });

      // Verify all FileDeletionRequested events are present (one per file)
      const deletionRequestedEvents = (deletionRequestResult.events || []).filter((record) =>
        userApi.events.fileSystem.FileDeletionRequested.is(record.event)
      );

      assert.equal(
        deletionRequestedEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} FileDeletionRequested events`
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Verify deletion signatures are stored in database for the User deletion type
      await indexerApi.indexer.verifyDeletionSignaturesStored({ sql, fileKeys });

      // Disconnect fisherman API before restart to prevent test runner hang
      // IMPORTANT: If this is not done, the api connection cannot close properly and the test
      // runner will hang.
      await fishermanApi.disconnect();

      // Restart fisherman container (this will unpause and restart it from scratch)
      // The pending deletions should be picked up after restart
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      // Wait for fisherman to come back online
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName,
        timeout: 30000
      });

      // Use retry helper to verify fisherman processes the pending deletions after restart
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 4, // 1 BSP + 3 buckets
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 3,
        maxRetries: 3
      });
    });

    it("processes incomplete storage request deletions after fisherman container restart", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create 3 buckets with 2 files each (6 files total) that will become incomplete
      // Using replicationTarget: 2 to keep storage request alive so user can revoke it
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b0-f0.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b0-f1.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b1-f0.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b1-f1.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b2-f0.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-2",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/restart-incomplete-b2-f1.txt",
            bucketIdOrName: "test-restart-incomplete-bucket-2",
            replicationTarget: 2
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApis: undefined, // Intentionally skip BSP checks; replicationTarget keeps request incomplete
        mspApi: msp1Api
      });

      const { fileKeys } = batchResult;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
        await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
        await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
      }

      // Pause fisherman to prevent it from processing incomplete deletions before restart
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // Build all revocation calls
      const revocationCalls = fileKeys.map((fileKey) =>
        userApi.tx.fileSystem.revokeStorageRequest(fileKey)
      );

      // Seal a single block with all revocation requests
      const revokeResult = await userApi.block.seal({
        calls: revocationCalls,
        signer: shUser
      });

      // Verify all StorageRequestRevoked events are present (one per file)
      const revokedEvents = (revokeResult.events || []).filter((record) =>
        userApi.events.fileSystem.StorageRequestRevoked.is(record.event)
      );

      assert.equal(
        revokedEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} StorageRequestRevoked events`
      );

      // Verify all IncompleteStorageRequest events are present (one per file)
      const incompleteEvents = (revokeResult.events || []).filter((record) =>
        userApi.events.fileSystem.IncompleteStorageRequest.is(record.event)
      );

      assert.equal(
        incompleteEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} IncompleteStorageRequest events`
      );

      // Verify incomplete storage request state
      const incompleteStorageRequests =
        await userApi.query.fileSystem.incompleteStorageRequests.entries();
      assert(incompleteStorageRequests.length > 0, "Should have incomplete storage requests");

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Disconnect fisherman API before restart to prevent test runner hang
      // IMPORTANT: If this is not done, the api connection cannot close properly and the test
      // runner will hang.
      await fishermanApi.disconnect();

      // Restart fisherman container (this will unpause and restart it from scratch)
      // The pending incomplete deletions should be picked up after restart
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      // Wait for fisherman to come back online
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName,
        timeout: 30000
      });

      // Use retry helper to verify fisherman processes the pending incomplete deletions after restart
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 4, // 1 BSP + 3 buckets
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 3,
        maxRetries: 3
      });
    });
  }
);
