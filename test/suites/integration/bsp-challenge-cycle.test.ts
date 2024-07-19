import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  runBspNet,
  type BspNetApi,
  cleardownTest,
  DUMMY_BSP_ID,
  type BspNetConfig
} from "../../util";
import { sleep } from "@zombienet/utils";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: BSP Challenge Cycle", () => {
    let api: BspNetApi;

    before(async () => {
      await runBspNet(bspNetConfig);
      api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
    });

    after(async () => {
      await cleardownTest(api);
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await api.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

      const bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      await bspApi.disconnect();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("Challenge cycle initialised correctly by root", async () => {
      // Force initialise challenge cycle of Provider.
      const initialiseChallengeCycleResult = await api.sealBlock(
        api.tx.sudo.sudo(api.tx.proofsDealer.forceInitialiseChallengeCycle(DUMMY_BSP_ID))
      );

      // Assert that event for challenge cycle initialisation is emitted.
      api.assertEvent(
        "proofsDealer",
        "NewChallengeCycleInitialised",
        initialiseChallengeCycleResult.events
      );

      // Get the challenge period for the Provider.
      // For now, we are abusing the fact that all BSPs have the same (minimum) challenge period.
      // But...
      // TODO: Make this get the challenge period either from a runtime API or an RPC call.
      const spMinDeposit = api.consts.providers.spMinDeposit;
      const stakeToChallengePeriod = api.consts.proofsDealer.stakeToChallengePeriod;
      const challengePeriod = spMinDeposit.toNumber() / stakeToChallengePeriod.toNumber();

      // Advance `challengePeriod + 1` blocks. In `challengePeriod` blocks, the node will detect
      // a new challenge seed event to which it will respond by sending a remark with event transaction.
      // That transaction will be included in the next block.
      for (let i = 0; i < challengePeriod; i++) {
        await api.sealBlock();
      }
      // Wait for task to execute and seal one more block.
      await sleep(500);
      const blockResult = await api.sealBlock();

      // Assert for the remark event as a response to the challenge seed event.
      api.assertEvent("system", "Remarked", blockResult.events);
    });
  });
}
