import "@storagehub/api-augment";
import { strictEqual, notEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  TEST_ARTEFACTS,
  createApiObject,
  fetchEventData,
  runBspNet,
  shUser,
  checkFileChecksum,
  type BspNetApi,
  type BspNetConfig,
  closeBspNet,
  sleep
} from "../../util";
import { hexToString } from "@polkadot/util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
  // { noisy: true, rocksdb: false }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: BSP Volunteer", () => {
    let user_api: BspNetApi;
    let bsp_api: BspNetApi;

    before(async () => {
      await runBspNet(bspNetConfig);
      user_api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
      bsp_api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await user_api.disconnect();
      await bsp_api.disconnect();
      await closeBspNet();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await user_api.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

      const bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      await bspApi.disconnect();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("file is finger printed correctly", async () => {
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "nothingmuch-0";

      const newBucketEventEvent = await user_api.createBucket(bucketName);
      const newBucketEventDataBlob =
        user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const { fingerprint, size, location } = await user_api.loadFile(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      strictEqual(hexToString(location), destination);
      strictEqual(fingerprint, TEST_ARTEFACTS[source].fingerprint);
      strictEqual(size, TEST_ARTEFACTS[source].size);
    });

    it("issueStorageRequest sent correctly", async () => {
      // const source = "res/smile.jpg";
      const destination = "test/smile.jpg";
      const bucketName = "nothingmuch-1";

      const newBucketEventEvent = await user_api.createBucket(bucketName);
      const newBucketEventDataBlob =
        user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const issueStorageRequestResult = await user_api.sealBlock(
        user_api.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
          TEST_ARTEFACTS["res/smile.jpg"].size,
          DUMMY_MSP_ID,
          [NODE_INFOS.user.expectedPeerId]
        ),
        shUser
      );
      await sleep(500); // wait for the bsp to volunteer

      const { event } = user_api.assertEvent(
        "fileSystem",
        "NewStorageRequest",
        issueStorageRequestResult.events
      );

      const dataBlob = user_api.events.fileSystem.NewStorageRequest.is(event) && event.data;

      if (!dataBlob) {
        throw new Error("Event doesn't match Type");
      }

      strictEqual(dataBlob.who.toString(), NODE_INFOS.user.AddressId);
      strictEqual(dataBlob.location.toHuman(), destination);
      strictEqual(dataBlob.fingerprint.toString(), TEST_ARTEFACTS["res/smile.jpg"].fingerprint);
      strictEqual(dataBlob.size_.toBigInt(), TEST_ARTEFACTS["res/smile.jpg"].size);
      strictEqual(dataBlob.peerIds.length, 1);
      strictEqual(dataBlob.peerIds[0].toHuman(), NODE_INFOS.user.expectedPeerId);
    });

    it.only("bsp volunteers when issueStorageRequest sent", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "nothingmuch-2";

      const initial_bsp_forest_root = await bsp_api.getForestRoot();
      strictEqual(
        initial_bsp_forest_root.toString(),
        "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
      );

      const newBucketEventEvent = await user_api.createBucket(bucketName);
      const newBucketEventDataBlob =
        user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const { fingerprint, size, location } = await user_api.loadFile(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      await user_api.sealBlock(
        user_api.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          fingerprint,
          size,
          DUMMY_MSP_ID,
          [NODE_INFOS.user.expectedPeerId]
        ),
        shUser
      );

      await sleep(500); // wait for the bsp to volunteer
      const volunteer_pending = await user_api.rpc.author.pendingExtrinsics();
      strictEqual(
        volunteer_pending.length,
        1,
        "There should be one pending extrinsic from BSP (volunteer)"
      );

      await user_api.sealBlock();
      const [resBspId, resBucketId, resLoc, resFinger, resMulti, _, resSize] = fetchEventData(
        user_api.events.fileSystem.AcceptedBspVolunteer,
        await user_api.query.system.events()
      );

      strictEqual(resBspId.toHuman(), TEST_ARTEFACTS[source].fingerprint);
      strictEqual(resBucketId.toString(), newBucketEventDataBlob.bucketId.toString());
      strictEqual(resLoc.toHuman(), destination);
      strictEqual(resFinger.toString(), fingerprint);
      strictEqual(resMulti.length, 1);
      strictEqual((resMulti[0].toHuman() as string).includes(NODE_INFOS.bsp.expectedPeerId), true);
      strictEqual(resSize.toBigInt(), size);

      await sleep(5000); // wait for the bsp to download the file
      const confirm_pending = await user_api.rpc.author.pendingExtrinsics();
      strictEqual(
        confirm_pending.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store)"
      );

      await user_api.sealBlock();
      const [
        _bspConfirmRes_who,
        bspConfirmRes_bspId,
        bspConfirmRes_fileKey,
        bspConfirmRes_newRoot
      ] = fetchEventData(
        user_api.events.fileSystem.BspConfirmedStoring,
        await user_api.query.system.events()
      );

      strictEqual(bspConfirmRes_bspId.toHuman(), TEST_ARTEFACTS[source].fingerprint);

      await sleep(1000); // wait for the bsp to process the BspConfirmedStoring event
      const bsp_forest_root_after_confirm = await bsp_api.getForestRoot();
      strictEqual(bsp_forest_root_after_confirm.toString(), bspConfirmRes_newRoot.toString());
      notEqual(bsp_forest_root_after_confirm.toString(), initial_bsp_forest_root.toString());
      // TODO: check the file key. We need an RPC endpoint to compute the file key.

      await it("downloaded file passed integrity checks", async () => {
        await bsp_api.saveFile(bspConfirmRes_fileKey, "/storage/test/whatsup.jpg");
        const sha = await checkFileChecksum("test/whatsup.jpg");
        strictEqual(sha, TEST_ARTEFACTS["res/whatsup.jpg"].checksum);
      });
    });
  });
}
