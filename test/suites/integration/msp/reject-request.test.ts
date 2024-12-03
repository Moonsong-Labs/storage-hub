import assert, { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeMspNet(
  "Single MSP rejecting storage request",
  { initialised: true },
  ({ before, createMspApi, it, createUserApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMspApi();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP rejects storage request since it is already being stored", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/smile.jpg";
      const initialised = await getLaunchResponse();
      const bucketId = initialised?.fileMetadata.bucketId;

      assert(bucketId, "Bucket ID not found");

      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      await userApi.sealBlock(
        userApi.tx.fileSystem.issueStorageRequest(
          bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS[source].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
        ),
        shUser
      );

      // Allow time for the MSP to receive and store the file from the user
      await sleep(3000);

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
      await userApi.sealBlock();

      const { event: storageRequestRejectedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "StorageRequestRejected"
      );

      const storageRequestRejectedDataBlob =
        userApi.events.fileSystem.StorageRequestRejected.is(storageRequestRejectedEvent) &&
        storageRequestRejectedEvent.data;

      assert(storageRequestRejectedDataBlob, "Event doesn't match Type");

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      // Check that the MSP has not updated the local forest root of the bucket
      strictEqual(
        localBucketRoot.toString(),
        (await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString())).toString()
      );
    });
  }
);
