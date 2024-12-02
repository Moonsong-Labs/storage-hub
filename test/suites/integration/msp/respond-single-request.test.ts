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
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          null
        ),
        shUser
      );

      // Allow time for the MSP to receive and store the file from the user
      await sleep(3000);

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

      if (!newStorageRequestDataBlob) {
        throw new Error("NewStorageRequest event data does not match expected type");
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

      await userApi.wait.mspResponseInTxPool();
      await userApi.sealBlock();

      let mspAcceptedStorageRequestDataBlob: any = undefined;
      let storageRequestFulfilledDataBlob: any = undefined;

      try {
        const { event: mspAcceptedStorageRequestEvent } = await userApi.assert.eventPresent(
          "fileSystem",
          "MspAcceptedStorageRequest"
        );
        mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedStorageRequestEvent) &&
          mspAcceptedStorageRequestEvent.data;
      } catch {
        // Event not found, continue
      }

      try {
        const { event: storageRequestFulfilledEvent } = await userApi.assert.eventPresent(
          "fileSystem",
          "StorageRequestFulfilled"
        );
        storageRequestFulfilledDataBlob =
          userApi.events.fileSystem.StorageRequestFulfilled.is(storageRequestFulfilledEvent) &&
          storageRequestFulfilledEvent.data;
      } catch {
        // Event not found, continue
      }

      let acceptedFileKey: string | null = null;
      // We expect either the MSP accepted the storage request or the storage request was fulfilled
      if (mspAcceptedStorageRequestDataBlob) {
        acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
      } else if (storageRequestFulfilledDataBlob) {
        acceptedFileKey = storageRequestFulfilledDataBlob.fileKey.toString();
      }

      if (!acceptedFileKey) {
        throw new Error(
          "Neither MspAcceptedStorageRequest nor StorageRequestFulfilled events were found"
        );
      }

      strictEqual(acceptedFileKey.toString(), event.data.fileKey.toString());

      const { event: bucketRootChangedEvent } = await userApi.assert.eventPresent(
        "providers",
        "BucketRootChanged"
      );

      const bucketRootChangedDataBlob =
        userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent) &&
        bucketRootChangedEvent.data;

      if (!bucketRootChangedDataBlob) {
        throw new Error("Expected BucketRootChanged event but received event of different type");
      }

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        newBucketEventDataBlob.bucketId.toString()
      );

      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), local_bucket_root.toString());

      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        newBucketEventDataBlob.bucketId.toString(),
        event.data.fileKey.toString()
      );

      invariant(isFileInForest.isTrue, "File is not in forest");
    });
  }
);
