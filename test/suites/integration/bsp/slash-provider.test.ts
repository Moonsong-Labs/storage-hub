import { strictEqual } from "node:assert";
import { BspNetBlock, describeBspNet, type EnrichedBspApi } from "../../../util";

describeBspNet("BSPNet: Slash Provider", ({ before, createUserApi, createBspApi, it }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("Network launches and can be queried", { skip: true }, async () => {
    const userNodePeerId = await api.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), api.shConsts.NODE_INFOS.user.expectedPeerId);

    const bspApi = await createBspApi();
    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    await bspApi.disconnect();
    strictEqual(bspNodePeerId.toString(), api.shConsts.NODE_INFOS.bsp.expectedPeerId);
  });

  it("Slash provider when SlashableProvider event processed", async () => {
    console.log("hello3");
    // Force initialise challenge cycle of Provider.
    const initialiseChallengeCycleResult = await api.sealBlock(
      api.tx.sudo.sudo(api.tx.proofsDealer.forceInitialiseChallengeCycle(api.shConsts.DUMMY_BSP_ID))
    );

    // Assert that event for challenge cycle initialisation is emitted.
    api.assertEvent(
      "proofsDealer",
      "NewChallengeCycleInitialised",
      initialiseChallengeCycleResult.events
    );

    const [_currentTick, nextChallengeDeadline1, _provider, _maybeProviderAccount] =
      api.assert.fetchEvent(
        api.events.proofsDealer.NewChallengeCycleInitialised,
        await api.query.system.events()
      );

    console.log("hello");
    //TODO: Fix me
    const nextChallengeDeadline2 = await BspNetBlock.skipToChallengePeriod(
      api,
      nextChallengeDeadline1.toNumber(),
      api.shConsts.DUMMY_BSP_ID
    );
    console.log("hello2");

    await api.assert.providerSlashed(api, api.shConsts.DUMMY_BSP_ID);

    // Check that the provider is no longer slashable.
    const slashableProvidersAfterSlash = await api.query.proofsDealer.slashableProviders(
      api.shConsts.DUMMY_BSP_ID
    );
    strictEqual(slashableProvidersAfterSlash.isNone, true);

    // Simulate 2 failed challenge periods
    await api.block.skipToChallengePeriod(nextChallengeDeadline2, api.shConsts.DUMMY_BSP_ID);

    await api.assert.providerSlashed(api, api.shConsts.DUMMY_BSP_ID);
  });
});
