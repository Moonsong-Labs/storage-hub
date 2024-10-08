import { strictEqual } from "node:assert";
import {
  describeMspNet,
  shUser,
  sleep,
  type EnrichedBspApi,
} from "../../../util";
import type { Bytes } from "@polkadot/types";

describeMspNet(
  "Single MSP responding to storage request",
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
      strictEqual(
        userNodePeerId.toString(),
        userApi.shConsts.NODE_INFOS.user.expectedPeerId
      );

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(
        mspNodePeerId.toString(),
        userApi.shConsts.NODE_INFOS.msp.expectedPeerId
      );
    });

    it("MSP receives file from user after issued storage request", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) &&
        newBucketEventEvent.data;

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
      strictEqual(
        fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS[source].fingerprint
      );
      strictEqual(
        file_size.toBigInt(),
        userApi.shConsts.TEST_ARTEFACTS[source].size
      );

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

      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );

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

      const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
        event.data.fileKey
      );

      if (!result.isFileFound) {
        throw new Error("File not found in storage");
      }

      // Seal block containing the MSP's transaction response to the storage request
      await userApi.wait.mspResponse();

      const mspRespondEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "MspRespondedToStorageRequests"
      );

      const mspRespondDataBlob =
        userApi.events.fileSystem.MspRespondedToStorageRequests.is(
          mspRespondEvent.event
        ) && mspRespondEvent.event.data;

      if (!mspRespondDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const responses = mspRespondDataBlob.results.responses;
      if (responses.length !== 1) {
        throw new Error("Expected 1 response");
      }

      const response = responses[0].asAccepted;

      strictEqual(
        response.bucketId.toString(),
        newBucketEventDataBlob.bucketId.toString()
      );
      strictEqual(
        response.fileKeys[0].toString(),
        newStorageRequestDataBlob.fileKey.toString()
      );

      //Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        response.bucketId.toString()
      );

      strictEqual(
        response.newBucketRoot.toString(),
        local_bucket_root.toString()
      );
    });
  }
);

describeMspNet(
  "Single MSP responding to multiple storage requests",
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
      strictEqual(
        userNodePeerId.toString(),
        userApi.shConsts.NODE_INFOS.user.expectedPeerId
      );

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(
        mspNodePeerId.toString(),
        userApi.shConsts.NODE_INFOS.msp.expectedPeerId
      );
    });

    it("MSP receives files from user after issued storage requests", async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
      const destination = [
        "test/whatsup.jpg",
        "test/adolphus.jpg",
        "test/smile.jpg",
      ];
      const bucketName = "nothingmuch-3";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) &&
        newBucketEventEvent.data;

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

      const events = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequest"
      );

      const matchedEvents = events.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );

      if (matchedEvents.length !== source.length) {
        throw new Error(`Expected ${source.length} NewStorageRequest events`);
      }

      let file_keys = [];
      for (const e of matchedEvents) {
        const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
          e.event.data.fileKey
        );

        if (!result.isFileFound) {
          throw new Error(
            `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
          );
        }

        file_keys.push(e.event.data.fileKey);
      }

      // Seal block containing the MSP's transaction response to the storage request
      await userApi.wait.mspResponse();

      const mspRespondEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "MspRespondedToStorageRequests"
      );

      const mspRespondDataBlob =
        userApi.events.fileSystem.MspRespondedToStorageRequests.is(
          mspRespondEvent.event
        ) && mspRespondEvent.event.data;

      if (!mspRespondDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const responses = mspRespondDataBlob.results.responses;
      if (responses.length !== 1) {
        throw new Error(
          `Expected 1 response since there is only a single bucket and should have been accepted`
        );
      }

      const response = responses[0].asAccepted;

      strictEqual(
        response.bucketId.toString(),
        newBucketEventDataBlob.bucketId.toString()
      );

      // There is only a single key being accepted since it is the first file key to be processed and there is nothing to batch.
      strictEqual(response.fileKeys[0].toString(), file_keys[0].toString());

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        response.bucketId.toString()
      );

      strictEqual(
        response.newBucketRoot.toString(),
        local_bucket_root.toString()
      );

      // Advance the block to free up the queue for the next set of storage requests to be processed.
      await userApi.sealBlock();

      // Seal block containing the MSP's transaction response to the storage request
      await userApi.wait.mspResponse();

      const mspRespondEvent2 = await userApi.assert.eventPresent(
        "fileSystem",
        "MspRespondedToStorageRequests"
      );

      const mspRespondDataBlob2 =
        userApi.events.fileSystem.MspRespondedToStorageRequests.is(
          mspRespondEvent2.event
        ) && mspRespondEvent2.event.data;

      if (!mspRespondDataBlob2) {
        throw new Error("Event doesn't match Type");
      }

      const responses2 = mspRespondDataBlob2.results.responses;
      if (responses2.length !== 1) {
        throw new Error(
          `Expected 1 response since there is only a single bucket and should have been accepted`
        );
      }

      const response2 = responses2[0].asAccepted;

      strictEqual(
        response2.bucketId.toString(),
        newBucketEventDataBlob.bucketId.toString()
      );

      // There are two keys being accepted at once since they are batched.
      strictEqual(response2.fileKeys[0].toString(), file_keys[1].toString());
      strictEqual(response2.fileKeys[1].toString(), file_keys[2].toString());

      // Allow time for the MSP to update the local forest root
      await sleep(3000);

      const local_bucket_root2 =
        await mspApi.rpc.storagehubclient.getForestRoot(
          response2.bucketId.toString()
        );

      strictEqual(
        response2.newBucketRoot.toString(),
        local_bucket_root2.toString()
      );
    });
  }
);
