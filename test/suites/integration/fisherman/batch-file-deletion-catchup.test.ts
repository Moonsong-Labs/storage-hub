import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";

/**
 * Validates fisherman only processes file deletions from FINALIZED blocks, ignoring unfinalized blocks and constructing valid forest proofs
 * based on finalized and unfinalized data.
 * 
 * Test coverage:
 * - Finalized user requested file deletions (URFDs) and incomplete storage requests (ISRs) are deleted by the fisherman.
 * - Unfinalized added and deleted file keys from BSP and MSP forests are caught up by the fisherman to construct valid forest proofs for ISRs.
 */
await describeMspNet(
  "Fisherman Batch File Deletion Catchup",
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
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    // Track file keys and bucket IDs for verification
    let finalizedUserFileKeys: string[];
    let finalizedUserBucketIds: string[];
    let unfinalizedUserFileKeys: string[];
    let unfinalizedUserBucketIds: string[];
    let finalizedIncompleteFileKeys: string[];
    let finalizedIncompleteBucketIds: string[];
    let unfinalizedIncompleteFileKeys: string[];
    let unfinalizedIncompleteBucketIds: string[];

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
      await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
    });

    it("pauses fisherman and creates finalized user deletion requests", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Pause fisherman to control deletion processing
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // === FINALIZED BLOCKS: Create 6 files and request deletions ===
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
        bspApi,
        mspApi: msp1Api
      });

      const { fileKeys, bucketIds, locations, fingerprints, fileSizes } = batchResult;

      // Store finalized user deletion data for verification in test 3
      finalizedUserFileKeys = fileKeys;
      finalizedUserBucketIds = bucketIds;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
        await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
        await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
      }

      // Build all deletion request calls for FINALIZED blocks
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

      // Wait for indexer to finalize these blocks
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Verify deletion signatures are stored in database for the User deletion type
      await indexerApi.indexer.verifyDeletionSignaturesStored({ sql, fileKeys });
    });

    it("creates unfinalized user deletion requests and manually deletes files", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // === UNFINALIZED BLOCKS: Create 6 more files and manually delete 3 ===
      const unfinalizedBatchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b0-f0.txt",
            bucketIdOrName: "test-batch-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b0-f1.txt",
            bucketIdOrName: "test-batch-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b1-f0.txt",
            bucketIdOrName: "test-batch-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b1-f1.txt",
            bucketIdOrName: "test-batch-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b2-f0.txt",
            bucketIdOrName: "test-batch-bucket-2",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/unfinalized-b2-f1.txt",
            bucketIdOrName: "test-batch-bucket-2",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api,
        finaliseBlock: false
      });

      const {
        fileKeys: unfinalizedFileKeys,
        bucketIds: unfinalizedBucketIds,
        locations: unfinalizedLocations,
        fingerprints: unfinalizedFingerprints,
        fileSizes: unfinalizedFileSizes
      } = unfinalizedBatchResult;

      // Store unfinalized user deletion data for verification in test 3
      unfinalizedUserFileKeys = unfinalizedFileKeys;
      unfinalizedUserBucketIds = unfinalizedBucketIds;

      // Request deletions for only 3 files (indices 0, 2, 4 - one from each bucket)
      const unfinalizedDeletionIndices = [0, 2, 4];
      const unfinalizedDeletionCalls = [];

      for (const idx of unfinalizedDeletionIndices) {
        const fileOperationIntention = {
          fileKey: unfinalizedFileKeys[idx],
          operation: { Delete: null }
        };

        const intentionCodec = userApi.createType(
          "PalletFileSystemFileOperationIntention",
          fileOperationIntention
        );
        const intentionPayload = intentionCodec.toU8a();
        const rawSignature = shUser.sign(intentionPayload);
        const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

        unfinalizedDeletionCalls.push(
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            unfinalizedBucketIds[idx],
            unfinalizedLocations[idx],
            unfinalizedFileSizes[idx],
            unfinalizedFingerprints[idx]
          )
        );
      }

      // Seal deletion requests WITHOUT finalizing
      const unfinalizedDeletionResult = await userApi.block.seal({
        calls: unfinalizedDeletionCalls,
        signer: shUser,
        finaliseBlock: false
      });

      // Verify FileDeletionRequested events
      const unfinalizedDeletionEvents = (unfinalizedDeletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.FileDeletionRequested.is(record.event)
      );

      assert.equal(
        unfinalizedDeletionEvents.length,
        unfinalizedDeletionIndices.length,
        `Should have ${unfinalizedDeletionIndices.length} FileDeletionRequested events for unfinalized blocks`
      );

      // Collect deletion signatures from events and map to file keys
      const deletionSignatures = new Map();
      for (const eventRecord of unfinalizedDeletionEvents) {
        const event = eventRecord.event;
        const dataBlob = userApi.events.fileSystem.FileDeletionRequested.is(event) && event.data;

        if (!dataBlob) {
          throw new Error("Event doesn't match FileDeletionRequested type");
        }

        const fileKey = dataBlob.signedDeleteIntention.fileKey.toString();
        deletionSignatures.set(fileKey, {
          signedIntention: dataBlob.signedDeleteIntention,
          signature: dataBlob.signature
        });
      }

      // Manually delete the 3 files from BSP and buckets
      const bspId = bspApi.shConsts.DUMMY_BSP_ID;

      // Build all deletion calls
      const deletionCalls = [];

      // Build FileDeletionRequest objects for BSP deletion
      const bspFileDeletionRequests = [];
      for (const idx of unfinalizedDeletionIndices) {
        const fileKey = unfinalizedFileKeys[idx];
        const deletionData = deletionSignatures.get(fileKey);

        bspFileDeletionRequests.push({
          fileOwner: shUser.address,
          signedIntention: deletionData.signedIntention,
          signature: deletionData.signature,
          bucketId: unfinalizedBucketIds[idx],
          location: unfinalizedLocations[idx],
          size: unfinalizedFileSizes[idx],
          fingerprint: unfinalizedFingerprints[idx]
        });
      }

      // Delete from BSP (all 3 files in one call)
      const bspFileKeys = unfinalizedDeletionIndices.map((idx) => unfinalizedFileKeys[idx]);
      const bspInclusionProof = await bspApi.rpc.storagehubclient.generateForestProof(
        null,
        bspFileKeys
      );
      deletionCalls.push(
        userApi.tx.fileSystem.deleteFiles(bspFileDeletionRequests, bspId, bspInclusionProof)
      );

      // Delete from each bucket (grouped by bucket)
      const bucketDeletions = new Map();
      for (const idx of unfinalizedDeletionIndices) {
        const bucketId = unfinalizedBucketIds[idx];
        const fileKey = unfinalizedFileKeys[idx];
        const deletionData = deletionSignatures.get(fileKey);

        if (!bucketDeletions.has(bucketId)) {
          bucketDeletions.set(bucketId, {
            fileKeys: [],
            deletionRequests: []
          });
        }

        bucketDeletions.get(bucketId).fileKeys.push(fileKey);
        bucketDeletions.get(bucketId).deletionRequests.push({
          fileOwner: shUser.address,
          signedIntention: deletionData.signedIntention,
          signature: deletionData.signature,
          bucketId,
          location: unfinalizedLocations[idx],
          size: unfinalizedFileSizes[idx],
          fingerprint: unfinalizedFingerprints[idx]
        });
      }

      for (const [bucketId, { fileKeys: bucketFileKeys, deletionRequests }] of bucketDeletions) {
        const bucketInclusionProof = await msp1Api.rpc.storagehubclient.generateForestProof(
          bucketId,
          bucketFileKeys
        );
        deletionCalls.push(
          userApi.tx.fileSystem.deleteFiles(deletionRequests, null, bucketInclusionProof)
        );
      }

      // Seal all deletions in a single block
      const deletionResult = await userApi.block.seal({
        calls: deletionCalls,
        finaliseBlock: false
      });

      // Verify BspFileDeletionsCompleted event
      userApi.assert.eventPresent("fileSystem", "BspFileDeletionsCompleted", deletionResult.events);

      // Verify BucketFileDeletionsCompleted events (one per bucket)
      const bucketDeletionEvents = (deletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)
      );

      assert.equal(
        bucketDeletionEvents.length,
        bucketDeletions.size,
        `Should have ${bucketDeletions.size} BucketFileDeletionsCompleted events`
      );
    });

    it("resumes fisherman and verifies only finalized user deletions are processed", async () => {
      // Resume fisherman
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      // Fisherman should only process the 6 files from FINALIZED blocks
      // The 3 manually deleted files from UNFINALIZED blocks should be ignored
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

      // Non-producer nodes must explicitly finalize imported blocks to trigger file deletion
      // Producer node (user) has finalized blocks, but BSP and MSP must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await msp1Api.wait.blockImported(finalisedBlockHash.toString());
      await msp1Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Verify finalized files are deleted from BSP and MSP storage
      await waitFor({
        lambda: async () => {
          for (let i = 0; i < finalizedUserFileKeys.length; i++) {
            const fileKey = finalizedUserFileKeys[i];
            const bucketId = finalizedUserBucketIds[i];

            // Check file is NOT in BSP forest
            const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
            if (bspForestResult.isTrue) {
              return false;
            }

            // Check file is NOT in BSP file storage
            const bspFileStorageResult =
              await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (bspFileStorageResult.isFileFound) {
              return false;
            }

            // Check file is NOT in MSP forest
            const mspForestResult = await msp1Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (mspForestResult.isTrue) {
              return false;
            }

            // Check file is NOT in MSP file storage
            const mspFileStorageResult =
              await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (mspFileStorageResult.isFileFound) {
              return false;
            }
          }
          return true;
        }
      });

      // Verify unfinalized files that were NOT manually deleted (indices 1, 3, 5) are still in storage
      const unfinalizedNonDeletedIndices = [1, 3, 5];
      await waitFor({
        lambda: async () => {
          for (const idx of unfinalizedNonDeletedIndices) {
            const fileKey = unfinalizedUserFileKeys[idx];
            const bucketId = unfinalizedUserBucketIds[idx];

            // Check file IS in BSP forest
            const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
            if (!bspForestResult.isTrue) {
              return false;
            }

            // Check file IS in BSP file storage
            const bspFileStorageResult =
              await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (!bspFileStorageResult.isFileFound) {
              return false;
            }

            // Check file IS in MSP forest
            const mspForestResult = await msp1Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (!mspForestResult.isTrue) {
              return false;
            }

            // Check file IS in MSP file storage
            const mspFileStorageResult =
              await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (!mspFileStorageResult.isFileFound) {
              return false;
            }
          }
          return true;
        }
      });
    });

    it("pauses fisherman and creates finalized incomplete storage requests", async () => {
      // Get value proposition before pausing fisherman
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Pause fisherman to control deletion processing
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // === FINALIZED BLOCKS: Create 6 files and revoke storage requests ===
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
        bspApi,
        mspApi: msp1Api
      });

      const { fileKeys, bucketIds } = batchResult;

      // Store finalized incomplete storage data for verification in test 6
      finalizedIncompleteFileKeys = fileKeys;
      finalizedIncompleteBucketIds = bucketIds;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Build all revocation calls for FINALIZED blocks
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

      // Wait for indexer to finalize these blocks
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
    });

    it("creates unfinalized incomplete storage requests and manually deletes files", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // === UNFINALIZED BLOCKS: Create 6 more files and manually delete 3 ===
      const unfinalizedBatchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b0-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b0-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-0",
            replicationTarget: 2
          },
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b1-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b1-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-1",
            replicationTarget: 2
          },
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b2-f0.txt",
            bucketIdOrName: "test-incomplete-bucket-2",
            replicationTarget: 2
          },
          {
            source: "res/cloud.jpg",
            destination: "test/unfinalized-incomplete-b2-f1.txt",
            bucketIdOrName: "test-incomplete-bucket-2",
            replicationTarget: 2
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api,
        finaliseBlock: false
      });

      const { fileKeys: unfinalizedFileKeys, bucketIds: unfinalizedBucketIds } =
        unfinalizedBatchResult;

      // Store unfinalized incomplete storage data for verification in test 6
      unfinalizedIncompleteFileKeys = unfinalizedFileKeys;
      unfinalizedIncompleteBucketIds = unfinalizedBucketIds;

      // Revoke storage requests for only 3 files (indices 0, 2, 4 - one from each bucket)
      const unfinalizedRevocationIndices = [0, 2, 4];
      const unfinalizedRevocationCalls = unfinalizedRevocationIndices.map((idx) =>
        userApi.tx.fileSystem.revokeStorageRequest(unfinalizedFileKeys[idx])
      );

      // Seal revocation requests WITHOUT finalizing
      await userApi.block.seal({
        calls: unfinalizedRevocationCalls,
        signer: shUser,
        finaliseBlock: false
      });

      // Manually delete the 3 files from BSP and buckets
      const bspId = bspApi.shConsts.DUMMY_BSP_ID;

      // Build all deletion calls
      const deletionCalls = [];

      // Delete from BSP (all 3 files in one call)
      const bspFileKeys = unfinalizedRevocationIndices.map((idx) => unfinalizedFileKeys[idx]);
      const bspInclusionProof = await bspApi.rpc.storagehubclient.generateForestProof(
        null,
        bspFileKeys
      );
      deletionCalls.push(
        userApi.tx.fileSystem.deleteFilesForIncompleteStorageRequest(
          bspFileKeys,
          bspId,
          bspInclusionProof
        )
      );

      // Delete from each bucket (grouped by bucket)
      const bucketDeletions = new Map();
      for (const idx of unfinalizedRevocationIndices) {
        const bucketId = unfinalizedBucketIds[idx];
        if (!bucketDeletions.has(bucketId)) {
          bucketDeletions.set(bucketId, {
            fileKeys: []
          });
        }
        bucketDeletions.get(bucketId).fileKeys.push(unfinalizedFileKeys[idx]);
      }

      for (const [bucketId, { fileKeys: bucketFileKeys }] of bucketDeletions) {
        const bucketInclusionProof = await msp1Api.rpc.storagehubclient.generateForestProof(
          bucketId,
          bucketFileKeys
        );
        deletionCalls.push(
          userApi.tx.fileSystem.deleteFilesForIncompleteStorageRequest(
            bucketFileKeys,
            null,
            bucketInclusionProof
          )
        );
      }

      // Seal all deletions in a single block
      const deletionResult = await userApi.block.seal({
        calls: deletionCalls,
        finaliseBlock: false
      });

      // Verify BspFileDeletionsCompleted event
      userApi.assert.eventPresent("fileSystem", "BspFileDeletionsCompleted", deletionResult.events);

      // Verify BucketFileDeletionsCompleted events (one per bucket)
      const bucketDeletionEvents = (deletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)
      );

      assert.equal(
        bucketDeletionEvents.length,
        bucketDeletions.size,
        `Should have ${bucketDeletions.size} BucketFileDeletionsCompleted events`
      );
    });

    it("resumes fisherman and verifies only finalized incomplete deletions are processed", async () => {
      // Resume fisherman
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      // Fisherman should only process the 6 files from FINALIZED blocks
      // The 3 manually deleted files from UNFINALIZED blocks should be ignored
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

      // Non-producer nodes must explicitly finalize imported blocks to trigger file deletion
      // Producer node (user) has finalized blocks, but BSP and MSP must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await msp1Api.wait.blockImported(finalisedBlockHash.toString());
      await msp1Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Verify finalized files are deleted from BSP and MSP storage
      await waitFor({
        lambda: async () => {
          for (let i = 0; i < finalizedIncompleteFileKeys.length; i++) {
            const fileKey = finalizedIncompleteFileKeys[i];
            const bucketId = finalizedIncompleteBucketIds[i];

            // Check file is NOT in BSP forest
            const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
            if (bspForestResult.isTrue) {
              return false;
            }

            // Check file is NOT in BSP file storage
            const bspFileStorageResult =
              await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (bspFileStorageResult.isFileFound) {
              return false;
            }

            // Check file is NOT in MSP forest
            const mspForestResult = await msp1Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (mspForestResult.isTrue) {
              return false;
            }

            // Check file is NOT in MSP file storage
            const mspFileStorageResult =
              await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (mspFileStorageResult.isFileFound) {
              return false;
            }
          }
          return true;
        }
      });

      // Verify unfinalized files that were NOT manually deleted (indices 1, 3, 5) are still in storage
      const unfinalizedNonDeletedIndices = [1, 3, 5];
      await waitFor({
        lambda: async () => {
          for (const idx of unfinalizedNonDeletedIndices) {
            const fileKey = unfinalizedIncompleteFileKeys[idx];
            const bucketId = unfinalizedIncompleteBucketIds[idx];

            // Check file IS in BSP forest
            const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
            if (!bspForestResult.isTrue) {
              return false;
            }

            // Check file IS in BSP file storage
            const bspFileStorageResult =
              await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (!bspFileStorageResult.isFileFound) {
              return false;
            }

            // Check file IS in MSP forest
            const mspForestResult = await msp1Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (!mspForestResult.isTrue) {
              return false;
            }

            // Check file IS in MSP file storage
            const mspFileStorageResult =
              await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (!mspFileStorageResult.isFileFound) {
              return false;
            }
          }
          return true;
        }
      });
    });
  }
);
