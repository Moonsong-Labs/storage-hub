import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  shUser,
  sleep,
  waitFor
} from "../../../util";

await describeMspNet(
  "Single MSP accepting subsequent storage request for same file key",
  {
    initialised: true,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, createMsp1Api, it, createUserApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    const bucketId1 = "cloud-bucket-1";
    const bucketId2 = "cloud-bucket-2";
    let file1: FileMetadata;
    let file2: FileMetadata;

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
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
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

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

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

    it("User deletes first file and Fisherman deletes it from MSP's forest", async () => {
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

      // Fisherman submits delete_files extrinsics (MSP + BSP)
      await userApi.assert.extrinsicPresent({
        method: "deleteFiles",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2 // 1 for MSP and 1 for BSP
      });
      await userApi.block.seal();

      // Verify file removed from first bucket's forest
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInForest(file1.bucketId, file1.fileKey)).isFalse
      });

      // Verify MSP still has the file in file storage via the second file key
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(file2.fileKey)).isFileFound
      });
    });

    it("User deletes second file and Fisherman deletes it from MSP's forest", async () => {
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

      await userApi.assert.extrinsicPresent({
        method: "deleteFiles",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 2 // 1 for MSP and 1 for BSP
      });
      const block = await userApi.block.seal({ finaliseBlock: true });

      // Finalising block in MSP node to trigger the event to delete the file.
      // First we make sure the MSP is caught up to the block.
      await mspApi.wait.nodeCatchUpToChainTip(userApi);
      await mspApi.block.finaliseBlock(block.blockReceipt.blockHash.toString());

      // Wait until MSP file storage no longer contains the file
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInForest(file2.bucketId, file2.fileKey)).isFalse
      });
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(file2.fileKey)).isFileNotFound
      });
    });
  }
);
