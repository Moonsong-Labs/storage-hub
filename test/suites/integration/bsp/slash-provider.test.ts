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
  fetchEventData
} from "../../../util";
import { sleep } from "@zombienet/utils";

describe("BSPNet: Slash Provider", () => {
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

    await runToNextChallengePeriodBlock(api, nextChallengeDeadline.toNumber());

    let numBlocksPassed = 0;

    await checkProviderWasSlashed(api, DUMMY_BSP_ID);

    numBlocksPassed += 1;

    // Check that the provider is no longer slashable.
    const slashableProvidersAfterSlash =
      await api.query.proofsDealer.slashableProviders(DUMMY_BSP_ID);
    strictEqual(slashableProvidersAfterSlash.isNone, true);

    await runToNextChallengePeriodBlock(
      api,
      (await getNextChallengeDeadlineAfterFirst(api)) - numBlocksPassed
    );
    await runToNextChallengePeriodBlock(api, await getNextChallengeDeadlineAfterFirst(api));

    await checkProviderWasSlashed(api, DUMMY_BSP_ID);
  });
});

/**
 * Wait some time before sealing a block and checking if the provider was slashed.
 * @param api
 * @param providerId
 */
async function checkProviderWasSlashed(api: BspNetApi, providerId: string) {
  // Weight for provider to be slashed.
  await sleep(1000);
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
 * @param nextChallengeDeadline
 */
async function runToNextChallengePeriodBlock(api: BspNetApi, nextChallengeDeadline: number) {
  // Assert that challengeTickToChallengedProviders contains an entry for the challenged provider
  const challengeTickToChallengedProviders =
    await api.query.proofsDealer.challengeTickToChallengedProviders(
      nextChallengeDeadline,
      DUMMY_BSP_ID
    );
  strictEqual(challengeTickToChallengedProviders.isSome, true);

  const blockNumber = await api.query.system.number();
  for (let i = blockNumber.toNumber(); i < nextChallengeDeadline - 1; i++) {
    await api.sealBlock();
  }

  const oldFailedSubmissionsCount = await api.query.proofsDealer.slashableProviders(DUMMY_BSP_ID);

  // Assert that the SlashableProvider event is emitted.
  const blockResult = await api.sealBlock();
  api.assertEvent("proofsDealer", "SlashableProvider", blockResult.events);

  // Check provider is slashable for 1 failed challenge deadline.
  const slashableProviders = await api.query.proofsDealer.slashableProviders(DUMMY_BSP_ID);
  strictEqual(
    slashableProviders.unwrap().toNumber(),
    oldFailedSubmissionsCount.unwrapOrDefault().toNumber() + 1
  );
}

/**
 * Get the next challenge deadline after the first challenge deadline (taking into account the challenge tick tolerance).
 * @param api
 */
async function getNextChallengeDeadlineAfterFirst(api: BspNetApi): Promise<number> {
  const currentTicker = await api.query.proofsDealer.challengesTicker();

  const spMinDeposit = api.consts.providers.spMinDeposit;
  const stakeToChallengePeriod = api.consts.proofsDealer.stakeToChallengePeriod;

  const challengePeriod = spMinDeposit.toNumber() / stakeToChallengePeriod.toNumber();

  return currentTicker.toNumber() + challengePeriod;
}
