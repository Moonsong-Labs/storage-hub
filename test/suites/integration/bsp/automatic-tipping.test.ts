import { strictEqual } from "node:assert";
import { describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import { assert } from "node:console";

describeBspNet(
  "BSP Automatic Tipping",
  { extrinsicRetryTimeout: 2 },
  ({ before, it, createUserApi }) => {
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

      await sleep(500); // wait for the bsp to volunteer
      await userApi.sealBlock();

      await sleep(1000); // wait for the bsp to download the files

      const confirmPending = await userApi.rpc.author.pendingExtrinsics();
      strictEqual(
        confirmPending.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store)"
      );

      await sleep(6000); // wait for the bsp to send confirm again
      const confirmPending2 = await userApi.rpc.author.pendingExtrinsics();
      const expectedRetryCount = 4;
      strictEqual(
        confirmPending2.length,
        expectedRetryCount,
        "There should be 4 pending extrinsic from BSP (confirm store) with increasing tip"
      );
      for (let i = 1; i < confirmPending2.length; ++i) {
        assert(
          confirmPending2[i].tip > confirmPending2[i - 1].tip,
          "Tip should increase with each retry"
        );
      }
    });
  }
);
