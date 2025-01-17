import { describeBspNet, shUser, type EnrichedBspApi, sleep } from "../../../util";
import assert from "node:assert";

describeBspNet(
  "BSP Exclude list tests",
  { initialised: "multi", networkConfig: "standard", only: true },
  ({ before, createUserApi, it, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Adding file fingerprint to exclude list and make sure it doesnt volunteer for it", async () => {
      const newBucketEventEvent = await userApi.createBucket("exclude-list");
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      // !!! It has to be called on `bspApi`
      await bspApi.rpc.storagehubclient.addToExcludeList(newBucketEventDataBlob.bucketId, "bucket")

      await bspApi.assert.log({
        searchString: "Key added to the exclude list",
        containerName: "docker-sh-bsp-1"
      });

      const { file_metadata: FileMetadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/cloud.jpg",
        "cat/cloud.jpg",
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId,
      );

      await sleep(5000);

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            FileMetadata.bucketId,
            FileMetadata.location,
            FileMetadata.fingerprint,
            FileMetadata.fileSize,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            null
          )
        ],
        signer: shUser
      });

      // waiting for bsp to see the request
      await sleep(5000);

      await bspApi.assert.log({
        searchString: "Bucket is in the exclude list",
        containerName: "docker-sh-bsp-1"
      });

    });
  }
);
