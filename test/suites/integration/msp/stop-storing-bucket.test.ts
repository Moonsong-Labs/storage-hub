import assert, { strictEqual } from "node:assert";
import type { H256 } from "@polkadot/types/interfaces";
import { describeMspNet, type EnrichedBspApi, mspKey, waitFor } from "../../../util";

await describeMspNet(
  "MSP deleting bucket when stop storing bucket is called",
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bucketId: H256;
    let fileKey: string;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
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
      fileKey = fileMetadata.fileKey;

      // Wait for MSP to download file from user
      await mspApi.wait.fileStorageComplete(fileMetadata.fileKey);

      // Seal block containing the MSP's transaction response to the storage request
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
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toHex())).isSome
      });

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
      const block = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.mspStopStoringBucket(bucketId)],
        signer: mspKey,
        finaliseBlock: false
      });

      // Check that bucket root exists before finalization
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString())).isSome
      });

      const bucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      // Bucket root should still exist since the block is not finalized
      strictEqual(bucketRoot.isSome, true);

      // Finalise block in MSP node to trigger the event to delete the bucket.
      await mspApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);

      // Check if the bucket root is deleted
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString())).isNone
      });

      // Verify the file is no longer in the MSP's file storage
      await mspApi.wait.fileDeletionFromFileStorage(fileKey);

      // Verify the forest storage has been fully removed
      const isPresent = await mspApi.rpc.storagehubclient.isForestStoragePresent(
        bucketId.toString()
      );
      strictEqual(isPresent.isFalse, true, "Forest storage should no longer be present");
    });
  }
);
