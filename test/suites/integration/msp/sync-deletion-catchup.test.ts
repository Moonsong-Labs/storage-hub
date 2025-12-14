import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  type SqlClient,
  shUser,
  waitFor,
  getContainerPeerId
} from "../../../util";

/**
 * Tests that MSP and BSP correctly apply file deletion mutations that occurred while they were offline.
 *
 * When a provider restarts after missing enough blocks, it should enter sync mode,
 * which should detect any forest mutations (including file deletions) that happened in the missed blocks
 * and apply them to their local forest storage, as well as detect finality events for mutations to
 * clean up their file storage.
 *
 * Test flow:
 * 1. Create an initial storage request, MSP accepts, BSP confirms
 * 2. Pause MSP, request file deletion, advance blocks until the fisherman completes deletion
 * 3. Advance enough blocks to trigger initial sync when MSP restarts
 * 4. Restart MSP and verify it correctly syncs the deletion mutation
 * 5. Create another storage request, verify MSP accepts and BSP confirms (proves MSP is functional)
 * 6. Repeat steps 2-5 for BSP to verify BSP initial sync handles deletions correctly
 */
await describeMspNet(
  "Provider sync catches up file deletions that occurred while offline",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    networkConfig: [{ noisy: false, rocksdb: true }]
  },
  ({
    before,
    after,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createFishermanApi,
    createIndexerApi,
    createApi,
    createSqlClient
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    // Track file metadata for verification
    let file1: FileMetadata;
    let file2: FileMetadata;
    let file3: FileMetadata;

    // Track reconnected APIs
    let newMspApi: EnrichedBspApi;
    let newBspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

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
      sql = createSqlClient();

      // Wait for indexer to be ready
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    after(async () => {
      if (newMspApi) {
        await newMspApi.disconnect();
      }
      if (newBspApi) {
        await newBspApi.disconnect();
      }
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("MSP and BSP accept first storage request", async () => {
      const source = "res/smile.jpg";
      const destination = "test/smile.jpg";
      const bucketName = "sync-deletion-test-bucket";

      file1 = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // MSP completes file storage locally
      await mspApi.wait.fileStorageComplete(file1.fileKey);

      // Ensure acceptance and BSP volunteer -> stored
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, sealBlock: true });

      // Verify file is in MSP forest and file storage
      await waitFor({
        lambda: async () => {
          const inMspForest = await mspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inMspFileStorage = await mspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          return inMspFileStorage.isFileFound;
        }
      });

      // Verify file is in BSP forest and file storage
      await waitFor({
        lambda: async () => {
          const inBspForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file1.fileKey);
          return inBspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspFileStorage = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          return inBspFileStorage.isFileFound;
        }
      });

      // Wait for indexer to process the finalized block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Pauses MSP, requests file deletion, and fisherman completes deletion", async () => {
      // Pause MSP container so it misses the deletion
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Request file deletion
      const fileOperationIntention = {
        fileKey: file1.fileKey,
        operation: { Delete: null }
      };

      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            file1.bucketId,
            file1.location,
            file1.fileSize,
            file1.fingerprint
          )
        ],
        signer: shUser
      });

      // Verify FileDeletionRequested event
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested");

      // Wait for indexer to finalize
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process User deletions (should delete from BSP and bucket)
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2, // 1 BSP + 1 bucket
        userApi,
        bspApi,
        expectedBspCount: 1,
        // Skip MSP verification since it's paused
        expectedBucketCount: 1,
        maxRetries: 3,
        skipBucketIds: [file1.bucketId] // MSP is paused, can't verify its forest root
      });

      // Verify file is deleted from BSP forest and file storage
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await waitFor({
        lambda: async () => {
          const inBspForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file1.fileKey);
          return !inBspForest.isTrue;
        },
        iterations: 10,
        delay: 1000
      });

      await waitFor({
        lambda: async () => {
          const inBspFileStorage = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          return !inBspFileStorage.isFileFound;
        },
        iterations: 10,
        delay: 1000
      });
    });

    it("Advances blocks to trigger MSP initial sync, restarts MSP, and verifies deletion is applied", async () => {
      // Advance enough blocks to ensure MSP triggers initial sync when it restarts
      await userApi.block.skip(20);

      // Get the on-chain bucket root before restarting MSP
      const bucketOnChain = await userApi.query.providers.buckets(file1.bucketId);
      assert(bucketOnChain.isSome, "Bucket should exist on-chain");
      const expectedBucketRoot = bucketOnChain.unwrap().root.toString();

      // Disconnect MSP API before restarting container
      await mspApi.disconnect();

      // Restart MSP container (this will restart the node from scratch)
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for MSP RPC to respond
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);

      // Wait for MSP to be idle again
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 30000,
        tail: 50
      });

      // Reconnect MSP API
      newMspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Wait for MSP to log that it's handling coming out of sync mode
      await userApi.docker.waitForLog({
        searchString: "🥱 Handling coming out of sync mode",
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        timeout: 30000
      });

      // Ensure MSP catches up to chain tip
      await userApi.wait.nodeCatchUpToChainTip(newMspApi);

      // Seal a finalized block
      await userApi.block.seal({ finaliseBlock: true });

      // Propagate finality to MSP. We have to do this since in manual seal mode, finality is tracked locally per node.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await newMspApi.wait.blockImported(finalisedBlockHash.toString());
      await newMspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // CRITICAL: Verify that after initial sync, the MSP correctly applied the deletion mutation
      // The file should NOT be in the MSP's forest storage anymore
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return !inMspForest.isTrue;
        }
      });

      // The file should NOT be in MSP's file storage anymore
      await waitFor({
        lambda: async () => {
          const inMspFileStorage = await newMspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          return !inMspFileStorage.isFileFound;
        }
      });

      // Verify that MSP's local bucket root matches the on-chain root
      await waitFor({
        lambda: async () => {
          const localBucketRoot = await newMspApi.rpc.storagehubclient.getForestRoot(
            file1.bucketId
          );
          return localBucketRoot.toString() === expectedBucketRoot;
        }
      });
    });

    it("MSP accepts a new storage request after restart (proves MSP is functional)", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup-after-msp-restart.jpg";

      // Use existing bucket from file1
      const bucketIdH256 = userApi.createType("H256", file1.bucketId);
      file2 = await userApi.file.newStorageRequest(
        source,
        destination,
        bucketIdH256,
        undefined,
        undefined,
        1 // replication target = 1 so storage request gets fulfilled
      );

      // MSP completes file storage locally
      await newMspApi.wait.fileStorageComplete(file2.fileKey);

      // Ensure MSP accepts the storage request
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, sealBlock: true });

      // Verify file is in MSP forest and file storage
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file2.bucketId,
            file2.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inMspFileStorage = await newMspApi.rpc.storagehubclient.isFileInFileStorage(
            file2.fileKey
          );
          return inMspFileStorage.isFileFound;
        }
      });

      // Wait for indexer to process
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Pauses BSP, requests file deletion, and fisherman completes deletion", async () => {
      // Pause BSP container so it misses the deletion
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.bsp.containerName);

      // Request file deletion for file2
      const fileOperationIntention = {
        fileKey: file2.fileKey,
        operation: { Delete: null }
      };

      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            file2.bucketId,
            file2.location,
            file2.fileSize,
            file2.fingerprint
          )
        ],
        signer: shUser
      });

      // Verify FileDeletionRequested event
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested");

      // Wait for indexer to finalize
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process User deletions (should delete from BSP and bucket)
      // Skip BSP verification since it's paused
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2, // 1 BSP + 1 bucket
        userApi,
        bspApi: undefined, // Don't pass bspApi to avoid RPC calls to the paused node
        mspApi: newMspApi,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      // Verify file is deleted from MSP forest and file storage
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await newMspApi.wait.blockImported(finalisedBlockHash.toString());
      await newMspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file2.bucketId,
            file2.fileKey
          );
          return !inMspForest.isTrue;
        },
        iterations: 10,
        delay: 1000
      });

      await waitFor({
        lambda: async () => {
          const inMspFileStorage = await newMspApi.rpc.storagehubclient.isFileInFileStorage(
            file2.fileKey
          );
          return !inMspFileStorage.isFileFound;
        },
        iterations: 10,
        delay: 1000
      });
    });

    it("Advances blocks to trigger BSP initial sync, restarts BSP, and verifies deletion is applied", async () => {
      // Advance enough blocks to ensure BSP triggers initial sync when it restarts
      await userApi.block.skip(20);

      // Get the on-chain BSP root before restarting
      const bspId = userApi.shConsts.DUMMY_BSP_ID;
      const bspOnChain = await userApi.query.providers.backupStorageProviders(bspId);
      assert(bspOnChain.isSome, "BSP should exist on-chain");
      const expectedBspRoot = bspOnChain.unwrap().root.toString();

      // Disconnect BSP API before restarting container
      await bspApi.disconnect();

      // Restart BSP container
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
      });

      // Wait for BSP RPC to respond
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`, true);

      // Wait for BSP to be idle again
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        searchString: "💤 Idle",
        timeout: 30000,
        tail: 50
      });

      // Reconnect BSP API
      newBspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);

      // Wait for the BSP to log that it's handling coming out of sync mode
      await userApi.docker.waitForLog({
        searchString: "🥱 Handling coming out of sync mode",
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        timeout: 30000
      });

      // Ensure BSP catches up to chain tip
      await userApi.wait.nodeCatchUpToChainTip(newBspApi);

      // Seal a finalized block
      await userApi.block.seal({ finaliseBlock: true });

      // Propagate finality to BSP. We have to do this since in manual seal mode, finality is tracked locally per node.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await newBspApi.wait.blockImported(finalisedBlockHash.toString());
      await newBspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // CRITICAL: Verify that after initial sync, the BSP correctly applied the deletion mutation
      // The file should NOT be in the BSP's forest storage anymore
      await waitFor({
        lambda: async () => {
          const inBspForest = await newBspApi.rpc.storagehubclient.isFileInForest(
            null,
            file2.fileKey
          );
          return !inBspForest.isTrue;
        }
      });

      // The file should NOT be in BSP's file storage anymore
      await waitFor({
        lambda: async () => {
          const inBspFileStorage = await newBspApi.rpc.storagehubclient.isFileInFileStorage(
            file2.fileKey
          );
          return !inBspFileStorage.isFileFound;
        }
      });

      // Verify that BSP's local forest root matches the on-chain root
      await waitFor({
        lambda: async () => {
          const localBspRoot = await newBspApi.rpc.storagehubclient.getForestRoot(null);
          return localBspRoot.toString() === expectedBspRoot;
        }
      });
    });

    it("BSP accepts a new storage request after restart (proves BSP is functional)", async () => {
      const source = "res/cloud.jpg";
      const destination = "test/cloud-after-bsp-restart.jpg";

      // Use existing bucket from file1
      const bucketIdH256 = userApi.createType("H256", file1.bucketId);
      file3 = await userApi.file.newStorageRequest(
        source,
        destination,
        bucketIdH256,
        undefined,
        undefined,
        1
      );

      // MSP completes file storage locally
      await newMspApi.wait.fileStorageComplete(file3.fileKey);

      // Ensure MSP accepts and BSP volunteers/stores
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteerInTxPool(1);
      await userApi.block.seal();

      // Wait for BSP to complete file storage and confirm storing
      await newBspApi.wait.fileStorageComplete(file3.fileKey);
      await userApi.wait.bspStored({ expectedExts: 1, sealBlock: true });

      // Verify file is in BSP forest and file storage
      await waitFor({
        lambda: async () => {
          const inBspForest = await newBspApi.rpc.storagehubclient.isFileInForest(
            null,
            file3.fileKey
          );
          return inBspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspFileStorage = await newBspApi.rpc.storagehubclient.isFileInFileStorage(
            file3.fileKey
          );
          return inBspFileStorage.isFileFound;
        }
      });

      // Verify file is in MSP forest and file storage
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file3.bucketId,
            file3.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inMspFileStorage = await newMspApi.rpc.storagehubclient.isFileInFileStorage(
            file3.fileKey
          );
          return inMspFileStorage.isFileFound;
        }
      });
    });

    it("Final verification: all local roots match on-chain roots", async () => {
      // Verify MSP bucket root matches on-chain
      const bucketOnChain = await userApi.query.providers.buckets(file1.bucketId);
      assert(bucketOnChain.isSome, "Bucket should exist on-chain");
      const expectedBucketRoot = bucketOnChain.unwrap().root.toString();

      const localBucketRoot = await newMspApi.rpc.storagehubclient.getForestRoot(file1.bucketId);
      strictEqual(
        localBucketRoot.toString(),
        expectedBucketRoot,
        "MSP local bucket root should match on-chain root"
      );

      // Verify BSP forest root matches on-chain
      const bspId = userApi.shConsts.DUMMY_BSP_ID;
      const bspOnChain = await userApi.query.providers.backupStorageProviders(bspId);
      assert(bspOnChain.isSome, "BSP should exist on-chain");
      const expectedBspRoot = bspOnChain.unwrap().root.toString();

      const localBspRoot = await newBspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(
        localBspRoot.toString(),
        expectedBspRoot,
        "BSP local forest root should match on-chain root"
      );
    });
  }
);
