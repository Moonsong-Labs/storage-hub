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
  fetchEventData,
} from "../../../util";
import { sleep } from "@zombienet/utils";

describe("BSPNet: BSP Challenge Cycle", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet({ noisy: false, rocksdb: false });
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

  it("Slash provider when SlashableProvider event processed", async () => {
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

    const [_currentTick, nextChallengeDeadline, _provider, _maybeProviderAccount] = fetchEventData(
      api.events.proofsDealer.NewChallengeCycleInitialised,
      await api.query.system.events()
    );

    // Assert that challengeTickToChallengedProviders contains an entry for the challenged provider
    const challengeTickToChallengedProviders = await api.query.proofsDealer.challengeTickToChallengedProviders(nextChallengeDeadline, DUMMY_BSP_ID);
    strictEqual(challengeTickToChallengedProviders.isSome, true);

    const blockNumber = await api.query.system.number();

    // Advance `challengePeriod + 1` blocks. In `challengePeriod` blocks.
    for (let i = blockNumber.toNumber(); i < nextChallengeDeadline.toNumber(); i++) {
      await api.sealBlock();
    }

    // Wait for task to execute and seal one more block.
    await sleep(500);
    const blockResult = await api.sealBlock();

    // Assert that the SlashableProvider event is emitted.
    api.assertEvent("proofsDealer", "SlashableProvider", blockResult.events);
  });
});
