import assert, { notEqual, strictEqual } from "node:assert";
import { describeBspNet, shUser, sleep, waitFor, type EnrichedBspApi } from "../../../util";
import type { H256 } from "@polkadot/types/interfaces";
import type { Bytes, u64, U8aFixed } from "@polkadot/types";

describeBspNet("Single BSP Volunteering", ({ before, createBspApi, it, createUserApi }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

  const source = "res/whatsup.jpg";
  const destination = "test/whatsup.jpg";
  const bucketName = "nothingmuch-2";

  let file_size: u64;
  let fingerprint: U8aFixed;
  let location: Bytes;
  let bucketId: H256;

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

  it("bsp volunteers when issueStorageRequest sent", async () => {
    const initialBspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);

    strictEqual(
      initialBspForestRoot.toString(),
      "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
    );

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    assert(newBucketEventDataBlob, "Event doesn't match Type");

    bucketId = newBucketEventDataBlob.bucketId;

    const {
      file_metadata: { location: loc, fingerprint: fp, file_size: s }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
      newBucketEventDataBlob.bucketId
    );

    location = loc;
    fingerprint = fp;
    file_size = s;

    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          fingerprint,
          file_size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Custom: 1
          }
        )
      ],
      signer: shUser
    });

    await userApi.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true
    });

    const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    const newStorageRequestDataBlob =
      userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

    assert(newStorageRequestDataBlob, "NewStorageRequest event data does not match expected type");

    await userApi.block.seal();
    const {
      data: {
        bspId: resBspId,
        bucketId: resBucketId,
        location: resLoc,
        fingerprint: resFinger,
        multiaddresses: resMulti,
        size_: resSize
      }
    } = userApi.assert.fetchEvent(
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

    await waitFor({
      lambda: async () =>
        (await bspApi.rpc.storagehubclient.isFileInFileStorage(newStorageRequestDataBlob.fileKey))
          .isFileFound
    });

    // Wait for the BSP confirm extrinsic to be submitted to the TX pool
    await userApi.wait.bspStored({
      expectedExts: 1,
      sealBlock: false
    });

    // Seal the block with the confirm TX
    await userApi.block.seal();
    const {
      data: {
        bspId: bspConfirmRes_bspId,
        confirmedFileKeys: bspConfirmRes_fileKeys,
        newRoot: bspConfirmRes_newRoot
      }
    } = userApi.assert.fetchEvent(
      userApi.events.fileSystem.BspConfirmedStoring,
      await userApi.query.system.events()
    );

    strictEqual(bspConfirmRes_bspId.toHuman(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);

    // TODO: Investigate what needs to be added to poll
    await sleep(1000); // to avoid extFailure- IssueRequest already registered for file
    await waitFor({
      lambda: async () =>
        (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
        initialBspForestRoot.unwrap().toHex()
    });
    const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());
    notEqual(bspForestRootAfterConfirm.toString(), initialBspForestRoot.toString());
    // TODO: check the file key. We need an RPC endpoint to compute the file key.

    await it("downloaded file passed integrity checks", async () => {
      const saveFileToDisk = await bspApi.rpc.storagehubclient.saveFileToDisk(
        bspConfirmRes_fileKeys[0][0],
        "/storage/test/whatsup.jpg"
      );
      assert(saveFileToDisk.isSuccess);
      const sha = await userApi.docker.checkFileChecksum("test/whatsup.jpg");
      strictEqual(sha, userApi.shConsts.TEST_ARTEFACTS["res/whatsup.jpg"].checksum);
    });
  });

  it("bsp skips volunteering for the same file key already being stored", async () => {
    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          bucketId,
          location,
          fingerprint,
          file_size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Custom: 1
          }
        )
      ],
      signer: shUser
    });

    await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    await bspApi.docker.waitForLog({
      containerName: "storage-hub-sh-bsp-1",
      searchString: "Skipping file key",
      timeout: 15000
    });
  });
});

describeBspNet("Single BSP multi-volunteers", ({ before, createBspApi, createUserApi, it }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
    bspApi = await createBspApi();
  });

  it("bsp volunteers multiple files properly", async () => {
    const initialBspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
    // 1 block to maxthreshold (i.e. instant acceptance)
    const tickToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 1]
      }
    };
    await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(
          userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
        )
      ]
    });

    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
    const bucketName = "something-3";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    assert(newBucketEventDataBlob, "Event doesn't match Type");

    const txs = [];
    for (let i = 0; i < source.length; i++) {
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
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
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      );
    }

    await userApi.block.seal({ calls: txs, signer: shUser });

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
      await bspApi.wait.fileStorageComplete(fileKey);
    }

    // The first file to be completed will immediately acquire the forest write lock
    // and send the `bspConfirmStoring` extrinsic. The other two files will be queued.
    // Here we wait for the first `bspConfirmStoring` extrinsic to be submitted to the tx pool,
    // we seal the block and check for the `BspConfirmedStoring` event.
    await userApi.wait.bspStored({ expectedExts: 1 });

    const {
      data: { confirmedFileKeys: bspConfirmRes_fileKeys, newRoot: bspConfirmRes_newRoot }
    } = userApi.assert.fetchEvent(
      userApi.events.fileSystem.BspConfirmedStoring,
      await userApi.query.system.events()
    );

    // Here we expect only 1 file to be confirmed since we always prefer smallest possible latency.
    strictEqual(bspConfirmRes_fileKeys.length, 1);

    // Wait for the BSP to process the BspConfirmedStoring event.
    await waitFor({
      lambda: async () =>
        (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
        initialBspForestRoot.unwrap().toHex()
    });
    const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);

    strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());

    // After the previous block is processed by the BSP, the forest write lock is released and
    // the other pending `bspConfirmStoring` extrinsics are processed and batched into one extrinsic.
    await userApi.wait.bspStored({ expectedExts: 1 });

    const {
      data: { confirmedFileKeys: bspConfirm2Res_fileKeys, newRoot: bspConfirm2Res_newRoot }
    } = userApi.assert.fetchEvent(
      userApi.events.fileSystem.BspConfirmedStoring,
      await userApi.query.system.events()
    );

    // Here we expect 2 batched files to be confirmed.
    strictEqual(bspConfirm2Res_fileKeys.length, 2);

    await waitFor({
      lambda: async () =>
        (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
        bspForestRootAfterConfirm.toHex()
    });

    const bspForestRootAfterConfirm2 = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(bspForestRootAfterConfirm2.toString(), bspConfirm2Res_newRoot.toString());
  });
});
