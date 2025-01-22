import { describeBspNet, shUser, type EnrichedBspApi, sleep } from "../../../util";
import assert from "node:assert";

describeBspNet(
  "BSP Exclude list tests",
  { only: true },
  ({ before, createUserApi, it, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Adding bucket to exclude list and make sure it doesnt volunteer for it", async () => {
      const newBucketEventEvent = await userApi.createBucket("exclude-list");
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      // !!! It has to be called on `bspApi`
      await bspApi.rpc.storagehubclient.addToExcludeList(newBucketEventDataBlob.bucketId, "bucket");

      await bspApi.assert.log({
        searchString: "Key added to the exclude list",
        containerName: "docker-sh-bsp-1"
      });

      const { file_metadata: FileMetadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/whatsup.jpg",
        "test/whatsup.jpg",
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            FileMetadata.location,
            FileMetadata.fingerprint,
            FileMetadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            1
          )
        ],
        signer: shUser
      });

      await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      await bspApi.assert.log({
        searchString: "Bucket is in the exclude list",
        containerName: "docker-sh-bsp-1"
      });
    });
  }
);
