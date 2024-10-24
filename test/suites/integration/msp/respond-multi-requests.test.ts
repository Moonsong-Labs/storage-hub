import { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";

describeMspNet(
  "Single MSP accepting multiple storage requests",
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

    it("MSP receives files from user after issued storage requests", async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
      const bucketName = "nothingmuch-3";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const txs = [];
      for (let i = 0; i < source.length; i++) {
        const { fingerprint, file_size, location } =
          await userApi.rpc.storagehubclient.loadFileInStorage(
            source[i],
            destination[i],
            userApi.shConsts.NODE_INFOS.user.AddressId,
            newBucketEventDataBlob.bucketId
          );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
          )
        );
      }

      await userApi.sealBlock(txs, shUser, false);

      // Allow time for the MSP to receive and store the file from the user
      await sleep(3000);

      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");

      const matchedEvents = events.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );

      if (matchedEvents.length !== source.length) {
        throw new Error(`Expected ${source.length} NewStorageRequest events`);
      }

      const issuedFileKeys = [];
      for (const e of matchedEvents) {
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

        if (!newStorageRequestDataBlob) {
          throw new Error("Event doesn't match Type");
        }

        const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
          newStorageRequestDataBlob.fileKey
        );

        if (!result.isFileFound) {
          throw new Error(
            `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
          );
        }

        issuedFileKeys.push(newStorageRequestDataBlob.fileKey);
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

      // There is only a single key being accepted since it is the first file key to be processed and there is nothing to batch.
      strictEqual(
        issuedFileKeys.some((key) => key.toString() === response.fileKeys[0].toString()),
        true
      );

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

      // Seal block containing the MSP's transaction response to the storage request
      const responses2 = await userApi.wait.mspResponse();

      if (responses2.length !== 1) {
        throw new Error(
          "Expected 1 response since there is only a single bucket and should have been accepted"
        );
      }

      const response2 = responses2[0].asAccepted;

      strictEqual(response2.bucketId.toString(), newBucketEventDataBlob.bucketId.toString());

      // There are two keys being accepted at once since they are batched.
      strictEqual(
        issuedFileKeys.some((key) => key.toString() === response2.fileKeys[0].toString()),
        true
      );
      strictEqual(
        issuedFileKeys.some((key) => key.toString() === response2.fileKeys[1].toString()),
        true
      );

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root2 = await mspApi.rpc.storagehubclient.getForestRoot(
        response2.bucketId.toString()
      );

      strictEqual(response2.newBucketRoot.toString(), local_bucket_root2.toString());

      for (const fileKey of response2.fileKeys) {
        const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
          response2.bucketId.toString(),
          fileKey
        );
        invariant(isFileInForest.isTrue, "File is not in forest");
      }
    });
  }
);
