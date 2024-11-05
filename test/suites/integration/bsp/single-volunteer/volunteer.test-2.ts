import { strictEqual } from "node:assert";
import { describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../../util";

describeBspNet("Single BSP multi-volunteers", ({ before, createBspApi, createUserApi, it }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
    bspApi = await createBspApi();
  });

  it("bsp volunteers multiple files properly", async () => {
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
    const bucketName = "something-3";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

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

    // Get the new storage request events, making sure we have 3
    const storageRequestEvents = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
    strictEqual(storageRequestEvents.length, 3);

    // Get the file keys from the storage request events
    const fileKeys = storageRequestEvents.map((event) => {
      const dataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
      if (!dataBlob) {
        throw new Error("Event doesn't match Type");
      }
      return dataBlob.fileKey;
    });

    // Wait for the BSP to volunteer
    await userApi.wait.bspVolunteer(source.length);

    // Wait for the BSP to receive and store all files
    for (let i = 0; i < source.length; i++) {
      const fileKey = fileKeys[i];
      await bspApi.wait.bspFileStorageComplete(fileKey);
    }

    // The first file to be completed will immediately acquire the forest write lock
    // and send the `bspConfirmStoring` extrinsic. The other two files will be queued.
    // Here we wait for the first `bspConfirmStoring` extrinsic to be submitted to the tx pool,
    // we seal the block and check for the `BspConfirmedStoring` event.
    await userApi.wait.bspStored(1);

    const [
      _bspConfirmRes_who,
      _bspConfirmRes_bspId,
      bspConfirmRes_fileKeys,
      bspConfirmRes_newRoot
    ] = userApi.assert.fetchEventData(
      userApi.events.fileSystem.BspConfirmedStoring,
      await userApi.query.system.events()
    );

    // Here we expect only 1 file to be confirmed since we always prefer smallest possible latency.
    strictEqual(bspConfirmRes_fileKeys.length, 1);

    // Wait for the BSP to process the BspConfirmedStoring event.
    await sleep(500);
    const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());

    // After the previous block is processed by the BSP, the forest write lock is released and
    // the other pending `bspConfirmStoring` extrinsics are processed and batched into one extrinsic.
    await userApi.wait.bspStored(1);

    const [
      _bspConfirm2Res_who,
      _bspConfirm2Res_bspId,
      bspConfirm2Res_fileKeys,
      bspConfirm2Res_newRoot
    ] = userApi.assert.fetchEventData(
      userApi.events.fileSystem.BspConfirmedStoring,
      await userApi.query.system.events()
    );

    // Here we expect 2 batched files to be confirmed.
    strictEqual(bspConfirm2Res_fileKeys.length, 2);

    await sleep(500); // wait for the bsp to process the BspConfirmedStoring event
    const bspForestRootAfterConfirm2 = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(bspForestRootAfterConfirm2.toString(), bspConfirm2Res_newRoot.toString());
  });
});
