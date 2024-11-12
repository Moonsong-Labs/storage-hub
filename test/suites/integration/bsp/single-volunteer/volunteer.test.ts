import assert, { notEqual, strictEqual } from "node:assert";
import { describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../../util";

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

  it("file is finger printed correctly", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketName = "nothingmuch-0";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const { location, fingerprint, file_size } =
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);
  });

  it("issueStorageRequest sent correctly", async () => {
    // const source = "res/smile.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    await userApi.sealBlock(
      userApi.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        destination,
        userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
        userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size,
        userApi.shConsts.DUMMY_MSP_ID,
        [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );
    await sleep(500); // wait for the bsp to volunteer

    const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    const dataBlob = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), userApi.shConsts.NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), destination);
    strictEqual(
      dataBlob.fingerprint.toString(),
      userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint
    );
    strictEqual(dataBlob.size_.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size);
    strictEqual(dataBlob.peerIds.length, 1);
    strictEqual(dataBlob.peerIds[0].toHuman(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
  });

  it("bsp volunteers when issueStorageRequest sent", async () => {
    const source = "res/whatsup.jpg";
    const destination = "test/whatsup.jpg";
    const bucketName = "nothingmuch-2";

    const initialBspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);

    strictEqual(
      initialBspForestRoot.toString(),
      "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
    );

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

    await userApi.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true
    });

    await userApi.sealBlock();
    const [resBspId, resBucketId, resLoc, resFinger, resMulti, _, resSize] =
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.AcceptedBspVolunteer,
        await userApi.query.system.events()
      );

    strictEqual(resBspId.toHuman(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(resBucketId.toString(), newBucketEventDataBlob.bucketId.toString());
    strictEqual(resLoc.toHuman(), destination);
    strictEqual(resFinger.toString(), fingerprint.toString());
    strictEqual(resMulti.length, 1);
    strictEqual(
      (resMulti[0].toHuman() as string).includes(userApi.shConsts.NODE_INFOS.bsp.expectedPeerId),
      true
    );
    strictEqual(resSize.toBigInt(), file_size.toBigInt());

    await sleep(5000); // wait for the bsp to download the file
    await userApi.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspConfirmStoring",
      checkTxPool: true
    });

    await userApi.sealBlock();
    const [_bspConfirmRes_who, bspConfirmRes_bspId, bspConfirmRes_fileKeys, bspConfirmRes_newRoot] =
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

    strictEqual(bspConfirmRes_bspId.toHuman(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);

    await sleep(1000); // wait for the bsp to process the BspConfirmedStoring event
    const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());
    notEqual(bspForestRootAfterConfirm.toString(), initialBspForestRoot.toString());
    // TODO: check the file key. We need an RPC endpoint to compute the file key.

    await it("downloaded file passed integrity checks", async () => {
      const saveFileToDisk = await bspApi.rpc.storagehubclient.saveFileToDisk(
        bspConfirmRes_fileKeys[0],
        "/storage/test/whatsup.jpg"
      );
      assert(saveFileToDisk.isSuccess);
      const sha = await userApi.docker.checkFileChecksum("test/whatsup.jpg");
      strictEqual(sha, userApi.shConsts.TEST_ARTEFACTS["res/whatsup.jpg"].checksum);
    });
  });
});
