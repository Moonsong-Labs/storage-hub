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
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe(`BSPNet: BSP Volunteer (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
    let user_api: BspNetApi;
    let bsp_api: BspNetApi;

    before(async () => {
      await runSimpleBspNet(bspNetConfig);
      user_api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
      bsp_api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await user_api.disconnect();
      await bsp_api.disconnect();
      await closeSimpleBspNet();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await user_api.rpc.system.localPeerId();
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

      const newBucketEventEvent = await user_api.createBucket(bucketName);
      const newBucketEventDataBlob =
        user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      let txs: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];
      for (let i = 0; i < source.length; i++) {
        const { fingerprint, file_size, location } =
          await user_api.rpc.storagehubclient.loadFileInStorage(
            source[i],
            destination[i],
            NODE_INFOS.user.AddressId,
            newBucketEventDataBlob.bucketId
          );

        txs.push(
          user_api.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            DUMMY_MSP_ID,
            [NODE_INFOS.user.expectedPeerId]
          )
        );
      }

      await user_api.sealBlock(txs, shUser);

      await sleep(500); // wait for the bsp to volunteer
      const volunteer_pending = await user_api.rpc.author.pendingExtrinsics();
      strictEqual(
        volunteer_pending.length,
        source.length,
        "There should be pending extrinsics for all files from BSP (volunteer)"
      );

      await user_api.sealBlock();

      await sleep(5000); // wait for the bsp to download the files
      const confirm_pending_1 = await user_api.rpc.author.pendingExtrinsics();
      strictEqual(
        confirm_pending_1.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store) for the first file"
      );

      await user_api.sealBlock();
      const [
        _bspConfirmRes_who,
        _bspConfirmRes_bspId,
        bspConfirmRes_fileKeys,
        bspConfirmRes_newRoot
      ] = fetchEventData(
        user_api.events.fileSystem.BspConfirmedStoring,
        await user_api.query.system.events()
      );

      strictEqual(bspConfirmRes_fileKeys.length, 1);

      await sleep(500); // wait for the bsp to process the BspConfirmedStoring event
      const bsp_forest_root_after_confirm = await bsp_api.rpc.storagehubclient.getForestRoot();
      strictEqual(bsp_forest_root_after_confirm.toString(), bspConfirmRes_newRoot.toString());

      // This block should trigger the next file to be confirmed.
      await user_api.sealBlock();

      // Even though we didn't sent a new file, the BSP client should process the rest of the files.
      // We wait for the BSP to send the confirm transaction.
      await sleep(500);
      const confirm_pending_2 = await user_api.rpc.author.pendingExtrinsics();
      strictEqual(
        confirm_pending_2.length,
        1,
        "There should be one pending extrinsic from BSP (confirm store) for the second file"
      );

      await user_api.sealBlock();

      const [
        _bspConfirm2Res_who,
        _bspConfirm2Res_bspId,
        bspConfirm2Res_fileKeys,
        bspConfirm2Res_newRoot
      ] = fetchEventData(
        user_api.events.fileSystem.BspConfirmedStoring,
        await user_api.query.system.events()
      );

      strictEqual(bspConfirm2Res_fileKeys.length, 1);

      await sleep(500); // wait for the bsp to process the BspConfirmedStoring event
      const bsp_forest_root_after_confirm2 = await bsp_api.rpc.storagehubclient.getForestRoot();
      strictEqual(bsp_forest_root_after_confirm2.toString(), bspConfirm2Res_newRoot.toString());
    });
  });
}
