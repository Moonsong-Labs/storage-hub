import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  type BspNetApi,
  type BspNetConfig,
  closeSimpleBspNet,
  runMultipleInitialisedBspsNet
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false }
  // { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe(
    `BSPNet: BSP Submits Proofs (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`,
    {
      only: true
    },
    () => {
      let userApi: BspNetApi;
      let bspApi: BspNetApi;
      let bspTwoApi: BspNetApi;
      let bspThreeApi: BspNetApi;

      before(async () => {
        const bspPorts = await runMultipleInitialisedBspsNet(bspNetConfig);
        userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
        bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
        bspTwoApi = await createApiObject(`ws://127.0.0.1:${bspPorts?.bspTwoRpcPort}`);
        bspThreeApi = await createApiObject(`ws://127.0.0.1:${bspPorts?.bspThreeRpcPort}`);
      });

      after(async () => {
        await userApi.disconnect();
        await bspApi.disconnect();
        await bspTwoApi.disconnect();
        await bspThreeApi.disconnect();
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

      it("BSP is challenged and correctly submits proof", async () => {});
      it("BSP fails to submit proof and is marked as slashable", async () => {});
      it("Many BSPs challenged and correctly submit proofs", async () => {});
      it(
        "BSP submits proof, transaction gets dropped, BSP-resubmits and succeeds",
        { skip: "Dropping transactions is not implemented as testing utility yet." },
        async () => {}
      );
      it("New storage request sent by user", async () => {
        it("Only one BSP confirms it", async () => {});
        it("BSP correctly responds to challenge with new forest root", async () => {});
      });
      it("File is deleted by user", async () => {
        it("Priority challenge is included in checkpoint challenge round", async () => {});
        it("BSP that has it responds to priority challenge with proof of inclusion", async () => {});
        it("BSPs who don't have it respond non-inclusion proof", async () => {});
        it("File is deleted by BSP", async () => {});
      });
      it("BSP stops storing last file", async () => {
        it("BSP is not challenged anymore", async () => {});
      });
    }
  );
}
