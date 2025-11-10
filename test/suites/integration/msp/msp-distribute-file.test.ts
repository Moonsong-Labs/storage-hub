import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeMspNet(
  "MSP distributes files to BSPs",
  ({ before, createMsp1Api, createBspApi, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("MSP distributes file to BSP correctly", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "distribution-test-bucket";

      // Pause the BSP so it cannot volunteer initially
      await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");

      // Create bucket and issue storage request
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHex,
        newBucketEventDataBlob.bucketId
      );

      strictEqual(location.toHuman(), destination);
      strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
      strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);

      // Issue storage request with both user and MSP peer IDs
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            destination,
            userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
            userApi.shConsts.TEST_ARTEFACTS[source].size,
            userApi.shConsts.DUMMY_MSP_ID,
            [
              userApi.shConsts.NODE_INFOS.user.expectedPeerId,
              userApi.shConsts.NODE_INFOS.msp1.expectedPeerId
            ],
            {
              Basic: null
            }
          )
        ],
        signer: shUser
      });

      // Get the new storage request event
      const { event: newStorageRequestEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent) &&
        newStorageRequestEvent.data;

      assert(
        newStorageRequestDataBlob,
        "NewStorageRequest event data does not match expected type"
      );

      // And get the file key from it
      const fileKey = newStorageRequestDataBlob.fileKey.toString();

      // Wait for MSP to download and accept the storage request
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool(1);
      await userApi.block.seal();

      // Get the MSP accepted storage request event
      const { event: mspAcceptedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const mspAcceptedDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedEvent) &&
        mspAcceptedEvent.data;

      assert(mspAcceptedDataBlob, "MspAcceptedStorageRequest event data does not match type");

      // Ensure the file key matches the one we issued
      strictEqual(mspAcceptedDataBlob.fileKey.toString(), fileKey);

      // Verify that the MSP has the file in its bucket forest
      await waitFor({
        lambda: async () =>
          (
            await mspApi.rpc.storagehubclient.isFileInForest(
              newBucketEventDataBlob.bucketId.toString(),
              fileKey
            )
          ).isTrue
      });

      // Delete the file from the user node's file storage
      // This ensures that the BSP can only receive the file from the MSP
      await userApi.rpc.storagehubclient.removeFilesFromFileStorage([fileKey]);

      // Verify that the file was deleted from the user's file storage
      await waitFor({
        lambda: async () => {
          const result = await userApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
          return result.isFileNotFound;
        }
      });

      // Resume the BSP node so it can detect and volunteer for the storage request
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-bsp-1" });

      // Wait for BSP to catch up to chain tip
      await userApi.wait.nodeCatchUpToChainTip(bspApi);

      // Wait for BSP to volunteer for the storage request
      await userApi.wait.bspVolunteerInTxPool(1);
      await userApi.block.seal();

      const { event: bspVolunteerEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "AcceptedBspVolunteer"
      );

      const bspVolunteerDataBlob =
        userApi.events.fileSystem.AcceptedBspVolunteer.is(bspVolunteerEvent) &&
        bspVolunteerEvent.data;

      assert(bspVolunteerDataBlob, "AcceptedBspVolunteer event data does not match type");

      // Ensure the fingerprint matches the one we issued
      strictEqual(
        bspVolunteerDataBlob.fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS[source].fingerprint
      );
      strictEqual(bspVolunteerDataBlob.bspId.toString(), userApi.shConsts.DUMMY_BSP_ID);

      // Wait for BSP to confirm storing the file (this means the MSP distributed the file
      // as that's the only way the file could have gotten to the BSP)
      await userApi.wait.bspStored({ expectedExts: 1, timeoutMs: 12000, sealBlock: true });

      // Get the BSP confirmed storing event
      const { event: bspConfirmedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmedStoring"
      );

      const bspConfirmedDataBlob =
        userApi.events.fileSystem.BspConfirmedStoring.is(bspConfirmedEvent) &&
        bspConfirmedEvent.data;

      assert(bspConfirmedDataBlob, "BspConfirmedStoring event data does not match type");

      // Ensure the file key matches the one we issued
      strictEqual(bspConfirmedDataBlob.confirmedFileKeys[0][0].toString(), fileKey);
      strictEqual(bspConfirmedDataBlob.bspId.toString(), userApi.shConsts.DUMMY_BSP_ID);

      // Verify that the file is in the BSP's file storage
      const isFileInStorage = await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
      assert(isFileInStorage.isFileFound, "File is not in BSP file storage");

      // Verify that the file is in the BSP's forest
      await waitFor({
        lambda: async () => {
          const isInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          return isInForest.isTrue;
        }
      });
    });
  }
);
