import { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedShApi } from "../../../util";
import invariant from "tiny-invariant";

describeMspNet(
  "Single MSP accepting storage request",
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedShApi;
    let mspApi: EnrichedShApi;

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
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp.expectedPeerId);
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

describeMspNet(
  "Single MSP accepting multiple storage requests",
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedShApi;
    let mspApi: EnrichedShApi;

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
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp.expectedPeerId);
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

      await userApi.sealBlock(txs, shUser);

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

describeMspNet(
  "Single MSP rejecting storage request",
  { initialised: true },
  ({ before, createMspApi, it, createUserApi, getLaunchResponse }) => {
    let userApi: EnrichedShApi;
    let mspApi: EnrichedShApi;

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
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp.expectedPeerId);
    });

    it("MSP rejects storage request since it is already being stored", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/smile.jpg";
      const initialised = await getLaunchResponse();
      const bucketId = initialised?.bucketIds[0];

      if (!bucketId) {
        throw new Error("Bucket ID not found");
      }

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        bucketId.toString()
      );

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

      // Seal block containing the MSP's transaction response to the storage request
      const responses = await userApi.wait.mspResponse();

      if (responses.length !== 1) {
        throw new Error(
          "Expected 1 response since there is only a single bucket and should have been accepted"
        );
      }

      const response = responses[0].asRejected;

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      // Check that the MSP has not updated the local forest root of the bucket
      strictEqual(
        local_bucket_root.toString(),
        (await mspApi.rpc.storagehubclient.getForestRoot(response.bucketId.toString())).toString()
      );

      strictEqual(response.bucketId.toString(), bucketId.toString());

      strictEqual(response.fileKeys[0][0].toString(), newStorageRequestDataBlob.fileKey.toString());
      strictEqual(response.fileKeys[0][1].toString(), "FileKeyAlreadyStored");
    });
  }
);
