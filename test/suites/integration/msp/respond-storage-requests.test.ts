import { notEqual, strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeMspNet(
  "Single MSP responding to storage requests",
  ({ before, createBspApi, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMspApi();
      if (maybeMspApi) {
        mspApi = maybeMspApi;
      } else {
        throw new Error("MSP API not available");
      }
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("MSP receives file from user after issued storage request", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const { location, fingerprint, file_size } =
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          newBucketEventDataBlob.bucketId
        );

      strictEqual(location.toHuman(), destination);
      strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
      strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);

      await userApi.sealBlock(
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS[source].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
        ),
        shUser
      );

      // Allow time for the MSP to receive and store the file from the user
      await sleep(6000);

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      const newStorageRequestDataBlob = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

      if (!newStorageRequestDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      strictEqual(newStorageRequestDataBlob.who.toString(), userApi.shConsts.NODE_INFOS.user.AddressId);
      strictEqual(newStorageRequestDataBlob.location.toHuman(), destination);
      strictEqual(
        newStorageRequestDataBlob.fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS[source].fingerprint
      );
      strictEqual(newStorageRequestDataBlob.size_.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);
      strictEqual(newStorageRequestDataBlob.peerIds.length, 1);
      strictEqual(newStorageRequestDataBlob.peerIds[0].toHuman(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(event.data.fileKey);

      if (!result.isFileFound) {
        throw new Error("File not found in storage");
      }

      const { event: respondEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspRespondedToStorageRequests"
      );

      const respondDataBlob =
        userApi.events.fileSystem.MspRespondedToStorageRequests.is(respondEvent) && respondEvent.data;

      if (!respondDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const responses = respondDataBlob.results.responses;
      if (responses.length !== 1) {
        throw new Error("Expected 1 response");
      }

      const response = responses[0].asAccepted;

      strictEqual(response.bucketId.toString(), newBucketEventDataBlob.bucketId.toString());
      strictEqual(response.fileKeys[0].toString(), newStorageRequestDataBlob.fileKey.toString());
      strictEqual(response.newBucketRoot.toString(), await mspApi.rpc.storagehubclient.getForestRoot(response.bucketId));
    });
  }
);