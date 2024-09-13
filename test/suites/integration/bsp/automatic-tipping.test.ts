import { strictEqual } from "node:assert";
import { describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeBspNet("BSP Automatic Tipping", {only: true, networkConfig: "standard" }, ({ before, it, createUserApi }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("Confirm storing failure results in increased tip", async () => {
    const source = "res/whatsup.jpg";
    const destination = "test/whatsup.jpg";
    const bucketName = "nothingmuch-2";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const { fingerprint, file_size, location } =
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

    await userApi.sealBlock(
      userApi.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        fingerprint,
        file_size,
        userApi.shConsts.DUMMY_MSP_ID,
        [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    await sleep(500); // wait for the bsp to download the files
    await userApi.sealBlock();

    await sleep(5000); // wait for the bsp to download the files

    const confirmPending2 = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
        confirmPending2.length,
      1,
      "There should be one pending extrinsic from BSP (confirm store)"
    );

    await sleep(67500); // wait for the bsp to send confirm again
    const confirmPending3 = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
        confirmPending3.length,
      3,
      "There should be one pending extrinsic from BSP (confirm store)"
    );
    console.log("confirmPending3", confirmPending3[0].tip.unwrap().toNumber());
  });
});
