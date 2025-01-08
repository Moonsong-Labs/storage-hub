import assert, { strictEqual } from "node:assert";
import { describeMspNet, mspKey, sleep, type EnrichedBspApi } from "../../../util";
import type { H256 } from "@polkadot/types/interfaces";

describeMspNet(
  "MSP deleting bucket when stop storing bucket is called",
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bucketId: H256;

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

    it("Create bucket and issue storage request", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");
      bucketId = newBucketEventDataBlob.bucketId;

      const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketId);

      // Wait for MSP to download file from user
      await mspApi.wait.fileStorageComplete(fileMetadata.fileKey);

      // Seal block containing the MSP's transaction response to the storage request
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

      assert(
        acceptedFileKey,
        "Neither MspAcceptedStorageRequest nor StorageRequestFulfilled events were found"
      );
      strictEqual(acceptedFileKey.toString(), fileMetadata.fileKey.toString());

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
      await sleep(3000);

      const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
        newBucketEventDataBlob.bucketId.toString()
      );

      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), local_bucket_root.toString());

      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        newBucketEventDataBlob.bucketId.toString(),
        acceptedFileKey
      );

      assert(isFileInForest.isTrue, "File is not in forest");
    });

    it("MSP stops storing bucket and deletes bucket from storage", async () => {
      const block = await userApi.sealBlock(
        userApi.tx.fileSystem.mspStopStoringBucket(bucketId),
        mspKey,
        false
      );

      await sleep(1500);

      const bucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      // Bucket root should still exist since the block is not finalized
      strictEqual(bucketRoot.isSome, true);

      // Finalise block in MSP node to trigger the event to delete the bucket.
      await mspApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);

      // Allow time for the MSP to delete files and bucket from storage
      await sleep(1500);

      const nonExistantBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(
        bucketId.toString()
      );

      strictEqual(nonExistantBucketRoot.isNone, true);
    });
  }
);
