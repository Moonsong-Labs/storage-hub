import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  createApiObject,
  fetchEventData,
  runSimpleBspNet,
  shUser,
  type BspNetApi,
  type BspNetConfig,
  closeSimpleBspNet,
  sleep
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe(`BSPNet: Multiple BSPs volunteer (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
    let userApi: BspNetApi;
    let bspApi: BspNetApi;

    before(async () => {
      await runSimpleBspNet(bspNetConfig);
      userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
      bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await userApi.disconnect();
      await bspApi.disconnect();
      await closeSimpleBspNet();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

      const bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      await bspApi.disconnect();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("bsp volunteers multiple files properly", async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg"];
      const bucketName = "nothingmuch-3";

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
            NODE_INFOS.user.AddressId,
            newBucketEventDataBlob.bucketId
          );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            DUMMY_MSP_ID,
            [NODE_INFOS.user.expectedPeerId]
          )
        );
      }

      await userApi.sealBlock(txs, shUser);

      await sleep(500); // wait for the bsp to volunteer
      const volunteer_pending = await userApi.rpc.author.pendingExtrinsics();
      strictEqual(
        volunteer_pending.length,
        source.length,
        "There should be pending extrinsics for all files from BSP (volunteer)"
      );

      await userApi.sealBlock();

      await sleep(5000); // wait for the bsp to download the files
      const confirm_pending_1 = await userApi.rpc.author.pendingExtrinsics();
      strictEqual(
        confirm_pending_1.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store) for the first file"
      );

      await userApi.sealBlock();
      const [
        _bspConfirmRes_who,
        _bspConfirmRes_bspId,
        bspConfirmRes_fileKeys,
        bspConfirmRes_newRoot
      ] = fetchEventData(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      strictEqual(bspConfirmRes_fileKeys.length, 1);

      await sleep(500); // wait for the bsp to process the BspConfirmedStoring event
      const bsp_forest_root_after_confirm = await bspApi.rpc.storagehubclient.getForestRoot();
      strictEqual(bsp_forest_root_after_confirm.toString(), bspConfirmRes_newRoot.toString());

      // This block should trigger the next file to be confirmed.
      await userApi.sealBlock();

      // Even though we didn't sent a new file, the BSP client should process the rest of the files.
      // We wait for the BSP to send the confirm transaction.
      await sleep(500);
      const confirm_pending_2 = await userApi.rpc.author.pendingExtrinsics();
      strictEqual(
        confirm_pending_2.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store) for the second file"
      );

      await userApi.sealBlock();

      const [
        _bspConfirm2Res_who,
        _bspConfirm2Res_bspId,
        bspConfirm2Res_fileKeys,
        bspConfirm2Res_newRoot
      ] = fetchEventData(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      strictEqual(bspConfirm2Res_fileKeys.length, 1);

      await sleep(500); // wait for the bsp to process the BspConfirmedStoring event
      const bsp_forest_root_after_confirm2 = await bspApi.rpc.storagehubclient.getForestRoot();
      strictEqual(bsp_forest_root_after_confirm2.toString(), bspConfirm2Res_newRoot.toString());
    });
  });
}
