import assert, { strictEqual } from "node:assert";
import { bspKey, describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeBspNet("Single BSP Volunteering", ({ before, createBspApi, it, createUserApi }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
    bspApi = await createBspApi();
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await userApi.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
  });

  it("Volunteer for multiple files and delete them", async () => {
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
    const bucketName = "something-3";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    assert(newBucketEventDataBlob, "Event doesn't match Type");

    const files = [];
    const txs = [];
    for (let i = 0; i < source.length; i++) {
      const { fingerprint, file_size, location } =
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          userApi.shConsts.NODE_INFOS.user.AddressId,
          newBucketEventDataBlob.bucketId
        );

      files.push({ fingerprint, file_size, location });
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
      assert(dataBlob, "Event doesn't match Type");
      return dataBlob.fileKey;
    });

    // Wait for the BSP to volunteer
    await userApi.wait.bspVolunteer(source.length);
    for (const fileKey of fileKeys) {
      await bspApi.wait.bspFileStorageComplete(fileKey);
    }

    // Waiting for a confirmation of the first file to be stored
    await sleep(500);
    await userApi.wait.bspStored(1);

    // Here we expect the 2 others files to be batched
    await sleep(500);
    await userApi.wait.bspStored(1);

    const stopStroringTxs = [];
    for (let i = 0; i < fileKeys.length; i++) {
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKeys[i]
      ]);
      stopStroringTxs.push(
        userApi.tx.fileSystem.bspRequestStopStoring(
          fileKeys[i],
          newBucketEventDataBlob.bucketId,
          files[i].location,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          files[i].fingerprint,
          files[i].file_size,
          false,
          inclusionForestProof.toString()
        )
      );
    }

    await userApi.sealBlock(stopStroringTxs, bspKey);

    await userApi.assert.eventMany("fileSystem", "BspRequestedToStopStoring");

    // Wait enough blocks for the deletion to be allowed.
    const currentBlock = await userApi.rpc.chain.getBlock();
    const currentBlockNumber = currentBlock.block.header.number.toNumber();
    const cooldown = currentBlockNumber + bspApi.consts.fileSystem.minWaitForStopStoring.toNumber();
    await userApi.block.skipTo(cooldown);

    for (let i = 0; i < fileKeys.length; i++) {
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKeys[i]
      ]);
      await userApi.sealBlock(
        userApi.tx.fileSystem.bspConfirmStopStoring(fileKeys[i], inclusionForestProof.toString()),
        bspKey
      );

      // Check for the confirm stopped storing event.
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");
    }
  });

  it(
    "Volunteer for multiple files and delete them (failing to batch when confirming)",
    { skip: "cannot store files again after they have been deleted once" },
    async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
      const bucketName = "something-4";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const files = [];
      const txs = [];
      for (let i = 0; i < source.length; i++) {
        const { fingerprint, file_size, location } =
          await userApi.rpc.storagehubclient.loadFileInStorage(
            source[i],
            destination[i],
            userApi.shConsts.NODE_INFOS.user.AddressId,
            newBucketEventDataBlob.bucketId
          );

        files.push({ fingerprint, file_size, location });
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
      const storageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequest"
      );
      strictEqual(storageRequestEvents.length, 3);

      // Get the file keys from the storage request events
      const fileKeys = storageRequestEvents.map((event) => {
        const dataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
        assert(dataBlob, "Event doesn't match Type");
        return dataBlob.fileKey;
      });

      // Wait for the BSP to volunteer
      await userApi.wait.bspVolunteer(source.length);
      for (const fileKey of fileKeys) {
        await bspApi.wait.bspFileStorageComplete(fileKey);
      }

      // Waiting for a confirmation of the first file to be stored
      await sleep(500);
      await userApi.wait.bspStored(1);

      // Here we expect the 2 others files to be batched
      await sleep(500);
      await userApi.wait.bspStored(1);

      const stopStroringTxs = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
          fileKeys[i]
        ]);
        stopStroringTxs.push(
          userApi.tx.fileSystem.bspRequestStopStoring(
            fileKeys[i],
            newBucketEventDataBlob.bucketId,
            files[i].location,
            userApi.shConsts.NODE_INFOS.user.AddressId,
            files[i].fingerprint,
            files[i].file_size,
            false,
            inclusionForestProof.toString()
          )
        );
      }

      await userApi.sealBlock(stopStroringTxs, bspKey);

      await userApi.assert.eventMany("fileSystem", "BspRequestedToStopStoring");

      // Wait enough blocks for the deletion to be allowed.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const cooldown =
        currentBlockNumber + bspApi.consts.fileSystem.minWaitForStopStoring.toNumber();
      await userApi.block.skipTo(cooldown);

      // Batching the delete confirmation should fail because of the wrong inclusionForestProof for extrinsinc 2 and 3
      const confirmStopStoringTxs = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
          fileKeys[i]
        ]);
        confirmStopStoringTxs.push(
          userApi.tx.fileSystem.bspConfirmStopStoring(fileKeys[i], inclusionForestProof.toString())
        );
      }

      await userApi.sealBlock(confirmStopStoringTxs, bspKey);

      // Check for the confirm stopped storing event.
      const confirmStopStoringEvents = await userApi.assert.eventMany(
        "fileSystem",
        "BspConfirmStoppedStoring"
      );

      assert(
        confirmStopStoringEvents.length === 1,
        "two of the extrinsincs should fail because of wrong inclusion proof"
      );
    }
  );
});