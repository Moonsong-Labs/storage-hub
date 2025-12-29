import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";

/**
 * This test was used to reproduce a deadlock between the `msp_upload_file.rs` task and the Blockchain Service.
 *
 * The deadlock happens when the MSP's Blockchain Service falls behind processing imported blocks, and there is
 * a new storage request in a bucket, followed by a deletion confirmation for another file in the same bucket.
 * What used to happen, before the fix, was that in some specific timing conditions, the task to handle the new
 * storage request would ask for the read lock for the bucket's forest storage, and then send some command to
 * the Blockchain Service. If the Blockchain Service started processing the block with the file deletion before
 * answering the command, when applying mutations to the forest storage, the Blockchain Service would ask for
 * the write lock of that bucket's forest storage. The Blockchain Service cannot acquire the write lock, because
 * the task to handle the new storage request already has the read lock. The task handling the new storage request
 * would keep waiting for the command response from the Blockchain Service, and the deadlock would happen.
 *
 * This deadlock is essentially a race condition, so it happens sometimes when running this test, so long as it
 * is run against a running MSP that does not has the fix for it. Additionally, to simulate long processing blocks,
 * and artifically make the MSP's Blockchain Service fall behind, this code needs to be added to the function
 * `msp_process_block_import_events` in `handler_msp.rs`:
 *
 * ```rust
 *         match self.role {
            MultiInstancesNodeRole::Leader | MultiInstancesNodeRole::Standalone => {
                match event {
                    // NEW EVENT HANDLER FOR A REMARK EVENT, THAT JUST SLEEPS FOR 6 SECONDS
                    StorageEnableEvents::System(frame_system::Event::Remarked {
                        sender: _,
                        hash,
                    }) => {
                        // Artificially sleep for 6 seconds to simulate a long block processing time.
                        std::thread::sleep(std::time::Duration::from_secs(6));
                        info!(target: LOG_TARGET, "Received remark event with hash: {:?}", hash);
                    }
                    // END OF NEW CODE
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::MoveBucketAccepted {
                            bucket_id,
                            old_msp_id: _,
                            new_msp_id,
                            value_prop_id,
                        },
                    ) => {
                        [...]
                    }
 * ```
 **/
await describeMspNet(
  "MSP deletes a file and can accept a new storage request afterwards",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    networkConfig: "standard",
    skip: true
  },
  ({
    before,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createFishermanApi,
    createIndexerApi,
    it,
    createSqlClient
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    let file1: FileMetadata;
    let file2: FileMetadata;

    const bucketName = "delete-and-new-request-bucket";

    const buildSignedDelete = (fileKey: string) => {
      const fileOperationIntention = { fileKey, operation: { Delete: null } };
      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });
      return { fileOperationIntention, userSignature } as const;
    };

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();

      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      assert(createFishermanApi, "Fisherman API not available");
      await createFishermanApi();

      assert(createIndexerApi, "Indexer API not available");
      indexerApi = await createIndexerApi();

      sql = createSqlClient();
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("Creates a storage request for file1 and waits for MSP and BSP confirmations", async () => {
      const source1 = "res/smile.jpg";
      const destination1 = "test/delete-flow-file1.jpg";

      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      file1 = await userApi.file.createBucketAndSendNewStorageRequest(
        source1,
        destination1,
        bucketName,
        valuePropId,
        mspId,
        shUser,
        1,
        true
      );

      // MSP completes file storage locally
      await mspApi.wait.fileStorageComplete(file1.fileKey);

      // Ensure acceptance and BSP volunteer -> stored
      await userApi.wait.mspResponseInTxPool();
      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      // Assert MSP and BSP forests contain the file
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
          const inBspForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file1.fileKey);
          return inBspForest.isTrue;
        }
      });
    });

    it("Creates a storage request for file2, in between long processing blocks", async () => {
      const source2 = "res/cloud.jpg";
      const destination2 = "test/delete-flow-file2.jpg";

      const bucketIdH256 = userApi.createType("H256", file1.bucketId);

      // Build a bunch of blocks with remark events to fill up the block import queue
      const blocksToBuild = 10;
      for (let i = 0; i < blocksToBuild; i++) {
        await userApi.block.seal({
          calls: [userApi.tx.system.remarkWithEvent("Remark event")]
        });
      }

      file2 = await userApi.file.newStorageRequest(source2, destination2, bucketIdH256, shUser);

      // Build a bunch of blocks with remark events to fill up the block import queue
      const blocksToBuild2 = 4;
      for (let i = 0; i < blocksToBuild2; i++) {
        await userApi.block.seal({
          calls: [userApi.tx.system.remarkWithEvent("Remark event")]
        });
      }
    });

    it("Sends a deletion request for file1 and waits for BSP to delete it", async () => {
      const { fileOperationIntention, userSignature } = buildSignedDelete(file1.fileKey);

      const deletionResult = await userApi.block.seal({
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

      // Ensure the FileDeletionRequested event is emitted
      const deletionRequestedEvents = (deletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.FileDeletionRequested.is(record.event)
      );

      strictEqual(
        deletionRequestedEvents.length,
        1,
        "Should have 1 FileDeletionRequested event for file1"
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process the file deletion (BSP + bucket)
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      // Non-producer nodes must explicitly finalise imported blocks to trigger file deletion
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await mspApi.wait.blockImported(finalisedBlockHash.toString());
      await mspApi.block.finaliseBlock(finalisedBlockHash.toString());
    });

    it("Add empty blocks to see if the blockchain service processes them", async () => {
      const blocksToBuild = 5;
      for (let i = 0; i < blocksToBuild; i++) {
        await userApi.block.seal({
          calls: []
        });
      }
    });

    it("Check that MSP responds to the new storage request", async () => {
      // MSP completes file storage locally for file2
      await mspApi.wait.fileStorageComplete(file2.fileKey);

      // Ensure acceptance and BSP volunteer -> stored
      await userApi.wait.mspResponseInTxPool();
      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      // Assert MSP and BSP forests contain file2
      await waitFor({
        lambda: async () => {
          const inMspForest = await mspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file2.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file2.fileKey);
          return inBspForest.isTrue;
        }
      });
    });
  }
);
