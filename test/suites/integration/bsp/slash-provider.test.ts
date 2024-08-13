import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  type BspNetApi,
  cleardownTest,
  DUMMY_BSP_ID,
  fetchEventData,
  runSimpleBspNet
} from "../../../util";
import { sleep } from "@zombienet/utils";

describe("BSPNet: Slash Provider", () => {
  let api: BspNetApi;

  before(async () => {
    await runSimpleBspNet({ noisy: false, rocksdb: false });
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

    const [_currentTick, nextChallengeDeadline1, _provider, _maybeProviderAccount] = fetchEventData(
      api.events.proofsDealer.NewChallengeCycleInitialised,
      await api.query.system.events()
    );

    const nextChallengeDeadline2 = await runToNextChallengePeriodBlock(
      api,
      nextChallengeDeadline1.toNumber(),
      DUMMY_BSP_ID
    );

    await checkProviderWasSlashed(api, DUMMY_BSP_ID);

    // Check that the provider is no longer slashable.
    const slashableProvidersAfterSlash =
      await api.query.proofsDealer.slashableProviders(DUMMY_BSP_ID);
    strictEqual(slashableProvidersAfterSlash.isNone, true);

    // Simulate 2 failed challenge periods
    await runToNextChallengePeriodBlock(api, nextChallengeDeadline2, DUMMY_BSP_ID);

    await checkProviderWasSlashed(api, DUMMY_BSP_ID);
  });
});

/**
 * Wait some time before sealing a block and checking if the provider was slashed.
 * @param api
 * @param providerId
 */
async function checkProviderWasSlashed(api: BspNetApi, providerId: string) {
  // Wait for provider to be slashed.
  await sleep(500);
  await api.sealBlock();

  const [provider, _amountSlashed] = fetchEventData(
    api.events.providers.Slashed,
    await api.query.system.events()
  );

  strictEqual(provider.toString(), providerId);
}

/**
 * Seal blocks until the next challenge period block.
 *
 * It will verify that the SlashableProvider event is emitted and check if the provider is slashable with an additional failed challenge deadline.
 * @param api
 * @param nextChallengeTick
 * @param provider
 */
async function runToNextChallengePeriodBlock(
  api: BspNetApi,
  nextChallengeTick: number,
  provider: string
): Promise<number> {
  // Assert that challengeTickToChallengedProviders contains an entry for the challenged provider
  const challengeTickToChallengedProviders =
    await api.query.proofsDealer.challengeTickToChallengedProviders(nextChallengeTick, provider);
  strictEqual(challengeTickToChallengedProviders.isSome, true);

  const blockNumber = await api.query.system.number();
  for (let i = blockNumber.toNumber(); i < nextChallengeTick - 1; i++) {
    await api.sealBlock();
  }

  const oldFailedSubmissionsCount = await api.query.proofsDealer.slashableProviders(provider);

  // Assert that the SlashableProvider event is emitted.
  const blockResult = await api.sealBlock();

  const [_provider, nextChallengeDeadline] = fetchEventData(
    api.events.proofsDealer.SlashableProvider,
    blockResult.events
  );

  // Check provider is slashable for 1 additional failed submission.
  const slashableProviders = await api.query.proofsDealer.slashableProviders(provider);
  strictEqual(
    slashableProviders.unwrap().toNumber(),
    oldFailedSubmissionsCount.unwrapOrDefault().toNumber() + 1
  );

  return nextChallengeDeadline.toNumber();
}
