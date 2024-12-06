import assert, { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeMspNet(
  "Single MSP accepting multiple storage requests",
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
    const bucketName = "nothingmuch-3";
    let bucketId: string;

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

    it("User submits 3 storage requests in the same bucket", async () => {
      // Get value propositions form the MSP to use, and use the first one (can be any).
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;

      // Create a new bucket where all the files will be stored.
      const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "NewBucket event data does not match expected type");
      bucketId = newBucketEventDataBlob.bucketId.toString();

      // Seal block with 3 storage requests.
      const txs = [];
      for (let i = 0; i < source.length; i++) {
        const { fingerprint, file_size, location } =
          await userApi.rpc.storagehubclient.loadFileInStorage(
            source[i],
            destination[i],
            userApi.shConsts.NODE_INFOS.user.AddressId,
            bucketId
          );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            null
          )
        );
      }
      await userApi.sealBlock(txs, shUser);
    });

    it("MSP receives files from user and accepts them", async () => {
      // Get the events of the storage requests to extract the file keys and check
      // that the MSP received them.
      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
      const matchedEvents = events.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );
      assert(
        matchedEvents.length === source.length,
        `Expected ${source.length} NewStorageRequest events`
      );

      // Allow time for the MSP to receive and store the files from the user
      // TODO: Ideally, this should be turned into a polling helper function.
      await sleep(3000);

      // Check if the MSP received the files.
      for (const e of matchedEvents) {
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

        assert(newStorageRequestDataBlob, "Event doesn't match NewStorageRequest type");

        const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
          newStorageRequestDataBlob.fileKey
        );

        assert(
          result.isFileFound,
          `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
        );
      }

      // Seal block containing the MSP's first response.
      // MSPs batch responses to achieve higher throughput in periods of high demand. But they
      // also prioritise a fast response, so if the Forest Write Lock is available, it will send
      // the first response it can immediately.
      await userApi.wait.mspResponseInTxPool();
      await userApi.sealBlock();

      // Give time for the MSP to update the local forest root.
      // TODO: Ideally, this should be turned into a polling helper function.
      await sleep(1000);

      // Check that the local forest root is updated, and matches th on-chain root.
      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);

      const { event: bucketRootChangedEvent } = await userApi.assert.eventPresent(
        "providers",
        "BucketRootChanged"
      );
      const bucketRootChangedDataBlob =
        userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent) &&
        bucketRootChangedEvent.data;
      assert(
        bucketRootChangedDataBlob,
        "Expected BucketRootChanged event but received event of different type"
      );

      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), localBucketRoot.toString());

      // The MSP should have accepted exactly one file.
      // Register how many were accepted in the last block sealed.
      const acceptedFileKeys: string[] = [];
      const mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      for (const e of mspAcceptedStorageRequestEvents) {
        const mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
        if (mspAcceptedStorageRequestDataBlob) {
          acceptedFileKeys.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
        }
      }
      assert(
        acceptedFileKeys.length === 1,
        "Expected 1 file key accepted in first block after storage requests"
      );

      // An additional block needs to be sealed to accept the rest of the files.
      // There should be a pending transaction to accept the rest of the files.
      await userApi.wait.mspResponseInTxPool();
      await userApi.sealBlock();

      // Give time for the MSP to update the local forest root.
      // TODO: Ideally, this should be turned into a polling helper function.
      await sleep(1000);

      // Check that the local forest root is updated, and matches th on-chain root.
      const localBucketRoot2 = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);

      const { event: bucketRootChangedEvent2 } = await userApi.assert.eventPresent(
        "providers",
        "BucketRootChanged"
      );
      const bucketRootChangedDataBlob2 =
        userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent2) &&
        bucketRootChangedEvent2.data;
      assert(
        bucketRootChangedDataBlob2,
        "Expected BucketRootChanged event but received event of different type"
      );

      strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), localBucketRoot2.toString());

      // The MSP should have accepted at least one file.
      // Register how many were accepted in the last block sealed.
      const mspAcceptedStorageRequestEvents2 = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      for (const e of mspAcceptedStorageRequestEvents2) {
        const mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
        if (mspAcceptedStorageRequestDataBlob) {
          acceptedFileKeys.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
        }
      }

      // Now for sure, the total number of accepted files should be `source.length`.
      assert(acceptedFileKeys.length === source.length, `Expected ${source.length} file keys`);

      // And they should be in the Forest storage of the MSP, in the Forest corresponding
      // to the bucket ID.
      for (const fileKey of acceptedFileKeys) {
        const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
        assert(isFileInForest.isTrue, "File is not in forest");
      }
    });
  }
);
