import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";

/**
 * FISHERMAN BATCH FILE DELETION INTEGRATION TESTS
 *
 * Validates fisherman batch processing for file deletions, ensuring files are grouped by
 * target (BSP/Bucket) and submitted in batched extrinsics.
 *
 * Test 1: User-Requested Deletions
 * - Setup: 3 buckets × 2 files = 6 files, users submit `requestDeleteFile` extrinsics
 * - Fisherman submits: 1 `deleteFiles` for BSP (6 files) + 3 `deleteFiles` for buckets (2 each)
 * - Events: `FileDeletionRequested`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Database signatures, forest root updates, batch grouping
 *
 * Test 2: Incomplete Storage Deletions
 * - Setup: 3 buckets × 2 files = 6 files, users revoke via `revokeStorageRequest`
 * - Fisherman submits: 1 `deleteFilesForIncompleteStorageRequest` for BSP (6 files) + 3 for buckets
 * - Events: `StorageRequestRevoked`, `IncompleteStorageRequest`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Incomplete storage cleanup, forest root updates
 *
 * Batch interval: 5 seconds (test config), 60 seconds (default)
 */
await describeMspNet(
  "Fisherman Batch File Deletion",
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

    it("batches user-requested file deletions across multiple buckets with parallel BSP and bucket processing", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create 3 buckets with 2 files each (6 files total)
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/batch-b0-f0.txt",
            bucketIdOrName: "test-batch-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/batch-b0-f1.txt",
            bucketIdOrName: "test-batch-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/batch-b1-f0.txt",
            bucketIdOrName: "test-batch-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/batch-b1-f1.txt",
            bucketIdOrName: "test-batch-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/batch-b2-f0.txt",
            bucketIdOrName: "test-batch-bucket-2",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/batch-b2-f1.txt",
            bucketIdOrName: "test-batch-bucket-2",
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

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 4,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 3,
        maxRetries: 3
      });
    });

    it("batches incomplete storage request deletions across multiple buckets with parallel BSP and bucket processing", async () => {
      // Get value proposition before pausing MSP
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create 3 buckets with 2 files each (6 files total) that will become incomplete
      // Using replicationTarget: 2 to keep storage request alive so user can revoke it
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b0-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b0-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b1-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b1-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b2-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-2",
            replicationTarget: 2
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/incomplete-b2-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-2",
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

      // Ensure the BSP confirms to store all files before continuing
      // Due to race conditions, the BSP confirmations might come in multiple blocks, so we need to wait
      // for all confirmations to complete.
      const bspAddress = userApi.createType("Address", bspApi.accounts.bspKey.address);
      let stillConfirming = true;
      while (stillConfirming) {
        try {
          await userApi.wait.bspStored({
            expectedExts: 1,
            bspAccount: bspAddress,
            timeoutMs: 6000
          });
        } catch (_) {
          stillConfirming = false;
        }
      }

      // Sanity check: this test assumes the (single) BSP is actually storing these files,
      // otherwise fisherman won't be able to batch a BSP-side deletion for "Incomplete" requests.
      for (const [index, fileKey] of fileKeys.entries()) {
        try {
          await waitFor({
            lambda: async () => {
              const bspFileStorageResult =
                await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
              return bspFileStorageResult.isFileFound;
            }
          });
        } catch (error) {
          throw new Error(
            `BSP has not stored file in file storage: ${fileKey} at index ${index}: ${error}`
          );
        }

        try {
          await waitFor({
            lambda: async () => {
              const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(
                null,
                fileKey
              );
              return bspForestResult.isTrue;
            }
          });
        } catch (error) {
          throw new Error(
            `BSP is not storing file in forest: ${fileKey} at index ${index}: ${error}`
          );
        }
      }

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

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

      // Seal and finalize block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to catch up with chain
      await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 4,
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
