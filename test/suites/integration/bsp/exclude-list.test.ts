import { describeBspNet, sleep, type EnrichedBspApi } from "../../../util";
import assert from "node:assert";

describeBspNet(
  "BSP Exclude list tests",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, createUserApi, it, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Adding file fingerprint to exclude list", async () => {
      const newBucketEventEvent = await userApi.createBucket("exclude-list");
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const { file_metadata: fileMetadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/cloud.jpg",
        "cat/cloud.jpg",
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId,
      );

      await userApi.rpc.storagehubclient.addToExcludeList(fileMetadata.fingerprint, "fingerprint")

    });

    it("BSP not answering the request to store file on the exclude list", async () => {

    });
  }
);
