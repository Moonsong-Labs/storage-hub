import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  type BspNetApi,
  DUMMY_BSP_ID,
  type BspNetConfig,
  runInitialisedBspsNet,
  closeSimpleBspNet,
  sleep
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: BSP Challenge Cycle and Proof Submission", () => {
    let userApi: BspNetApi;
    let bspApi: BspNetApi;

    before(async () => {
      await runInitialisedBspsNet(bspNetConfig);
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

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("Challenge cycle initialised correctly", async () => {
      // Get the challenge period for the Provider.
      const challengePeriodResult =
        await bspApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);

      // Assert that the result is not an error.
      // This means tha the provider exists and is initialised with a challenge cycle.
      assert(challengePeriodResult.isOk);

      const challengePeriod = challengePeriodResult.asOk.toNumber();

      // Advance `challengePeriod + 1` blocks. In `challengePeriod` blocks, the node will detect
      // a new challenge seed event to which it will respond by sending a remark with event transaction.
      // That transaction will be included in the next block.
      for (let i = 0; i < challengePeriod; i++) {
        await userApi.sealBlock();
      }
      // Wait for task to execute and seal one more block.
      await sleep(500);
      const blockResult = await userApi.sealBlock();

      // Assert for the remark event as a response to the challenge seed event.
      bspApi.assertEvent("system", "Remarked", blockResult.events);
    });

    it("BSP is challenged and correctly submits proof", async () => {
      const result = await bspApi.call.storageProvidersApi.getBspInfo(DUMMY_BSP_ID);
      const bspInfo = result.asOk;
      const capacity: number = Number(bspInfo.capacity.toString());

      // TODO: Query when is the next challenge block.
      // TODO: Advance to next challenge block.
      // TODO: Build block with proof submission.
      // TODO: Check that proof submission was successful.
    });
    it("BSP fails to submit proof and is marked as slashable", async () => {
      // TODO: Stop BSP.
      // TODO: Advance to BSP deadline.
      // TODO: Check that BSP is slashable.
    });
  });
}
