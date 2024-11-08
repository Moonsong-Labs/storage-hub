import { strictEqual } from "node:assert";
import { describeMspNet, mspKey, sleep, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";
import type { H256 } from "@polkadot/types/interfaces";

describeMspNet(
  "MSP deleting bucket when stop storing bucket is called",
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bucketId: H256;

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

    it("Create bucket and issue storage request", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      bucketId = newBucketEventDataBlob.bucketId;

      const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketId);

      const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey);

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

      strictEqual(response.bucketId.toString(), bucketId.toString());
      strictEqual(response.fileKeys[0].toString(), fileMetadata.fileKey.toString());

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

    it("MSP stops storing bucket and deletes bucket from storage", async () => {
      const block = await userApi.sealBlock(
        mspApi.tx.fileSystem.mspStopStoringBucket(bucketId),
        mspKey
      );

      // Finalise block in MSP node to trigger the event to delete the bucket.
      await mspApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);

      // Allow time for the MSP to delete files and bucket from storage
      await sleep(3000);

      const maybeBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      strictEqual(maybeBucketRoot.isNone, true);
    });
  }
);
