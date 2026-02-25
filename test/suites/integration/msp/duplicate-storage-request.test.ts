import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  shUser,
  sleep,
  type SqlClient,
  waitFor
} from "../../../util";

await describeMspNet(
  "Single MSP accepting subsequent storage request for same file key",
  {
    initialised: true,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true
  },
  ({
    before,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createFishermanApi,
    it,
    getLaunchResponse,
    createSqlClient,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let sql: SqlClient;

    const bucketId1 = "cloud-bucket-1";
    const bucketId2 = "cloud-bucket-2";
    let file1: FileMetadata;
    let file2: FileMetadata;
    let indexerApi: EnrichedBspApi;

    // Helper to build and sign a file deletion intention
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
      sql = createSqlClient();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      assert(createFishermanApi, "Fisherman API not available");
      await createFishermanApi();

      assert(createIndexerApi, "Indexer API not available");
      indexerApi = await createIndexerApi();
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP accepts subsequent storage request for the same file key", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/smile.jpg";
      const initialised = await getLaunchResponse();
      const bucketId = initialised?.fileMetadata.bucketId;

      assert(bucketId, "Bucket ID not found");

      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            destination,
            userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
            userApi.shConsts.TEST_ARTEFACTS[source].size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Basic: null
            }
          )
        ],
        signer: shUser
      });

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequestV2");

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequestV2.is(event) && event.data;

      if (!newStorageRequestDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      strictEqual(
        newStorageRequestDataBlob.who.toString(),
        userApi.shConsts.NODE_INFOS.user.AddressId
      );
      strictEqual(newStorageRequestDataBlob.location.toHuman(), destination);
      strictEqual(
        newStorageRequestDataBlob.fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS[source].fingerprint
      );
      strictEqual(
        newStorageRequestDataBlob.size_.toBigInt(),
        userApi.shConsts.TEST_ARTEFACTS[source].size
      );

      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      const { event: storageRequestAccepted } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const storageRequestAcceptedDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(storageRequestAccepted) &&
        storageRequestAccepted.data;

      if (!storageRequestAcceptedDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      // Allow time for the MSP to update the local forest root
      await sleep(3000); // Mandatory sleep to check nothing has changed

      // Check that the MSP has not updated the local forest root of the bucket
      strictEqual(
        localBucketRoot.toString(),
        (await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString())).toString()
      );

      await mspApi.wait.fileStorageComplete(newStorageRequestDataBlob.fileKey);
    });

    it("MSP accepts same file in different buckets", async () => {
      const source = "res/cloud.jpg";
      const destination1 = "test/cloud-a.jpg";
      const destination2 = "test/cloud-b.jpg";

      // Query a value proposition to use when creating buckets
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Store same file in two different buckets
      file1 = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination1,
        bucketId1,
        valuePropId,
        mspId,
        shUser,
        1,
        true
      );
      await mspApi.wait.fileStorageComplete(file1.fileKey);
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer(1); // This seals the block as well

      const { event: storageRequestAccepted } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      const storageRequestAcceptedDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(storageRequestAccepted) &&
        storageRequestAccepted.data;

      if (!storageRequestAcceptedDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      strictEqual(storageRequestAcceptedDataBlob.fileKey.toString(), file1.fileKey.toString());

      file2 = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination2,
        bucketId2,
        valuePropId,
        mspId,
        shUser,
        1,
        true
      );

      // Check that both files have the same fingerprint.
      strictEqual(file2.fingerprint.toString(), file1.fingerprint.toString());

      await mspApi.wait.fileStorageComplete(file2.fileKey);
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer(1); // This seals the block as well

      const { event: storageRequestAccepted2 } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      const storageRequestAcceptedDataBlob2 =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(storageRequestAccepted2) &&
        storageRequestAccepted2.data;

      if (!storageRequestAcceptedDataBlob2) {
        throw new Error("Event doesn't match Type");
      }

      strictEqual(storageRequestAcceptedDataBlob2.fileKey.toString(), file2.fileKey.toString());
    });

    it("User deletes first file and Fisherman deletes it from Bucket's forest and BSP's forest", async () => {
      const { fileOperationIntention, userSignature } = buildSignedDelete(file1.fileKey);
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

      // Finalize the block on the indexer node and wait for the indexer to process the block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process the file deletions
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: mspApi,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      // Non-producer nodes must explicitly finalize imported blocks to trigger file deletion
      // Producer node (user) has finalized blocks, but BSP and MSP must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await mspApi.wait.blockImported(finalisedBlockHash.toString());
      await mspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // Verify that the file metadata from the second file was removed from
      // the file storage of the BSP and MSP, meaning that it should respond that this
      // fileKey is not in the file storage. Now the file content should have
      // been deleted from the file storage as well.
      await waitFor({
        lambda: async () => {
          // Check file is NOT in BSP forest
          const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            file1.fileKey
          );
          if (bspForestResult.isTrue) {
            return false;
          }

          // Check file is NOT in BSP file storage
          const bspFileStorageResult = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          if (bspFileStorageResult.isFileFound) {
            return false;
          }

          // Check file is NOT in MSP forest
          const mspForestResult = await mspApi.rpc.storagehubclient.isFileInForest(
            file2.bucketId,
            file1.fileKey
          );
          if (mspForestResult.isTrue) {
            return false;
          }

          // Check file is NOT in MSP file storage
          const mspFileStorageResult = await mspApi.rpc.storagehubclient.isFileInFileStorage(
            file1.fileKey
          );
          if (mspFileStorageResult.isFileFound) {
            return false;
          }
          return true;
        }
      });
    });

    it("Second file can still be downloaded from the MSP and BSP", async () => {
      // Download the file to the disk of the MSP.
      const saveFileToDiskMsp = await mspApi.rpc.storagehubclient.saveFileToDisk(
        file2.fileKey,
        "/storage/test/cloud-b-msp.jpg"
      );
      assert(saveFileToDiskMsp.isSuccess);

      // Check that the file checksum is correct.
      const shaMsp = await mspApi.docker.checkFileChecksum("test/cloud-b-msp.jpg", {
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });
      strictEqual(shaMsp, userApi.shConsts.TEST_ARTEFACTS["res/cloud.jpg"].checksum);

      // Download the file to the disk of the BSP.
      const saveFileToDiskBsp = await bspApi.rpc.storagehubclient.saveFileToDisk(
        file2.fileKey,
        "/storage/test/cloud-b-bsp.jpg"
      );
      assert(saveFileToDiskBsp.isSuccess);

      // Check that the file checksum is correct.
      const shaBsp = await bspApi.docker.checkFileChecksum("test/cloud-b-bsp.jpg", {
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
      });
      strictEqual(shaBsp, userApi.shConsts.TEST_ARTEFACTS["res/cloud.jpg"].checksum);
    });

    it("User deletes second file and Fisherman deletes it from Bucket's forest and BSP's forest", async () => {
      const { fileOperationIntention, userSignature } = buildSignedDelete(file2.fileKey);
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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: mspApi,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      // Non-producer nodes must explicitly finalize imported blocks to trigger file deletion
      // Producer node (user) has finalized blocks, but BSP and MSP must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      await mspApi.wait.blockImported(finalisedBlockHash.toString());
      await mspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // Verify that the file metadata from the second file was removed from
      // the file storage of the BSP and MSP, meaning that it should respond that this
      // fileKey is not in the file storage. Now the file content should have
      // been deleted from the file storage as well.

      // Check file is NOT in BSP forest
      await waitFor({
        lambda: async () => {
          const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            file2.fileKey
          );
          return !bspForestResult.isTrue;
        }
      });

      // Check file is NOT in BSP file storage
      await waitFor({
        lambda: async () => {
          const bspFileStorageResult = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            file2.fileKey
          );
          return !bspFileStorageResult.isFileFound;
        }
      });

      // Check file is NOT in MSP forest
      await waitFor({
        lambda: async () => {
          const mspForestResult = await mspApi.rpc.storagehubclient.isFileInForest(
            file2.bucketId,
            file2.fileKey
          );
          return !mspForestResult.isTrue;
        }
      });

      // Check file is NOT in MSP file storage
      await waitFor({
        lambda: async () => {
          const mspFileStorageResult = await mspApi.rpc.storagehubclient.isFileInFileStorage(
            file2.fileKey
          );
          return !mspFileStorageResult.isFileFound;
        }
      });
    });

    it("Second file can no longer be downloaded from the MSP and BSP", async () => {
      // Download the file to the disk of the MSP.
      const saveFileToDiskMsp = await mspApi.rpc.storagehubclient.saveFileToDisk(
        file2.fileKey,
        "/storage/test/cloud-b-msp.jpg"
      );
      assert(saveFileToDiskMsp.isFileNotFound);

      // Download the file to the disk of the BSP.
      const saveFileToDiskBsp = await bspApi.rpc.storagehubclient.saveFileToDisk(
        file2.fileKey,
        "/storage/test/cloud-b-bsp.jpg"
      );
      assert(saveFileToDiskBsp.isFileNotFound);
    });
  }
);
