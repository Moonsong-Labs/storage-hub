import { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";

describeMspNet(
  "Single MSP accepting storage request",
  { networkConfig: "standard" },
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
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

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
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
      strictEqual(newStorageRequestDataBlob.peerIds.length, 1);
      strictEqual(
        newStorageRequestDataBlob.peerIds[0].toHuman(),
        userApi.shConsts.NODE_INFOS.user.expectedPeerId
      );

      const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(event.data.fileKey);

      if (!result.isFileFound) {
        throw new Error("File not found in storage");
      }

      // Seal block containing the MSP's transaction response to the storage request
      const responses = await userApi.wait.mspResponse();

      if (responses.length !== 1) {
        throw new Error(
          "Expected 1 response since there is only a single bucket and should have been accepted"
        );
      }

      const response = responses[0].asAccepted;

      strictEqual(response.bucketId.toString(), newBucketEventDataBlob.bucketId.toString());
      strictEqual(response.fileKeys[0].toString(), newStorageRequestDataBlob.fileKey.toString());

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        response.bucketId.toString()
      );

      strictEqual(response.newBucketRoot.toString(), local_bucket_root.toString());

      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        response.bucketId.toString(),
        response.fileKeys[0]
      );

      invariant(isFileInForest.isTrue, "File is not in forest");
    });
  }
);
