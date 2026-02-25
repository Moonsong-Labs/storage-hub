import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeMspNet(
  "Single MSP accepting storage request",
  { networkConfig: "standard" },
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

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

    it("MSP receives file from user after issued storage request", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

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

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
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

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequestV2");

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequestV2.is(event) && event.data;

      assert(
        newStorageRequestDataBlob,
        "NewStorageRequestV2 event data does not match expected type"
      );

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

      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(event.data.fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      let mspAcceptedStorageRequestDataBlob: any;
      let storageRequestFulfilledDataBlob: any;

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

      assert(
        acceptedFileKey,
        "Neither MspAcceptedStorageRequest nor StorageRequestFulfilled events were found"
      );
      strictEqual(acceptedFileKey.toString(), event.data.fileKey.toString());

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

      // Allow time for the MSP to update the local forest root
      await waitFor({
        lambda: async () =>
          (
            await mspApi.rpc.storagehubclient.getForestRoot(
              newBucketEventDataBlob.bucketId.toString()
            )
          ).isSome
      });
      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        newBucketEventDataBlob.bucketId.toString()
      );

      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), local_bucket_root.toString());

      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        newBucketEventDataBlob.bucketId.toString(),
        event.data.fileKey.toString()
      );

      assert(isFileInForest.isTrue, "File is not in forest");
    });
  }
);
