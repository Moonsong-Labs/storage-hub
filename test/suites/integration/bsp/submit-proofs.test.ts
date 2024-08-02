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
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe(`BSPNet: Many BSPs Submit Proofs (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
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

    it("Many BSPs are challenged and correctly submit proofs", async () => {
      // TODO: Query when is the next challenge block.
      // TODO: Advance to next challenge block.
      // TODO: Check that BSPs have pending transaction of proof submission.
      // TODO: Build block with proof submissions.
      // TODO: Check that BSPs' proof submissions were successful.
      // TODO: Check that BSPs' challenge cycles were correctly pushed forward.
    });
    it("BSP fails to submit proof and is marked as slashable", async () => {
      // TODO: Advance to BSP-Down deadline.
      // TODO: Check that BSP-Down is slashable.
      // TODO: Check that BSP-Down's challenge cycle was correctly pushed forward.
    });
    it(
      "BSP submits proof, transaction gets dropped, BSP-resubmits and succeeds",
      { skip: "Dropping transactions is not implemented as testing utility yet." },
      async () => {}
    );
    it("New storage request sent by user", async () => {
      // TODO: Stop BSP-Two and BSP-Three.
      // TODO: Send transaction to create new storage request.
      it("Only one BSP confirms it", async () => {
        // TODO: Check that BSP volunteers alone.
        // TODO: Check that BSP confirms storage request.
      });
      it("BSP correctly responds to challenge with new forest root", async () => {
        // TODO: Advance to next challenge block.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful.
      });
    });
    it("Custom challenge is added", async () => {
      it("Custom challenge is included in checkpoint challenge round", async () => {
        // TODO: Send transaction for custom challenge with new file key.
        // TODO: Advance until next checkpoint challenge block.
        // TODO: Check that custom challenge was included in checkpoint challenge round.
      });
      it("BSP that has it responds to custom challenge with proof of inclusion", async () => {
        // TODO: Advance until next challenge for BSP.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful, including the custom challenge.
      });
      it("BSPs who don't have it respond non-inclusion proof", async () => {
        // TODO: Advance until next challenge for BSP-Two and BSP-Three.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful, with proof of non-inclusion.
      });
    });
    it("File is deleted by user", async () => {
      // TODO: Send transaction to delete file.
      // TODO: Advance until file deletion request makes it into the priority challenge round.
      it("Priority challenge is included in checkpoint challenge round", async () => {
        // TODO: Advance to next checkpoint challenge block.
        // TODO: Check that priority challenge was included in checkpoint challenge round.
      });
      it("BSP that has it responds to priority challenge with proof of inclusion", async () => {
        // TODO: Advance to next challenge block.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful, with proof of inclusion.
      });
      it("File is deleted by BSP", async () => {
        // TODO: Check that file is deleted by BSP, and no longer is in the Forest.
        // TODO: Check that file is deleted by BSP, and no longer is in the File System.
      });
      it("BSPs who don't have it respond non-inclusion proof", async () => {
        // TODO: Advance to next challenge block.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful, with proof of non-inclusion.
      });
    });
    it("BSP stops storing last file", async () => {
      // TODO: BSP-Three sends transaction to stop storing the only file it has.
      it("BSP is not challenged any more", async () => {
        // TODO: Check that BSP-Three no longer has a challenge deadline.
      });
    });
  });
}
