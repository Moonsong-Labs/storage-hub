import { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedShApi } from "../../../util";

describeBspNet("BSPNet: Slash Provider", ({ before, createUserApi, createBspApi, it }) => {
  let userApi: EnrichedShApi;
  let bspApi: EnrichedShApi;

  before(async () => {
    userApi = await createUserApi();
    bspApi = await createBspApi();
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await userApi.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    strictEqual(bspNodePeerId.toString(), bspApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
  });

  it("Slash provider when SlashableProvider event processed", async () => {
    // Force initialise challenge cycle of Provider.
    const initialiseChallengeCycleResult = await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.proofsDealer.forceInitialiseChallengeCycle(bspApi.shConsts.DUMMY_BSP_ID)
      )
    );

    // Assert that event for challenge cycle initialisation is emitted.
    userApi.assertEvent(
      "proofsDealer",
      "NewChallengeCycleInitialised",
      initialiseChallengeCycleResult.events
    );

    const [_currentTick, nextChallengeDeadline1, _provider, _maybeProviderAccount] =
      userApi.assert.fetchEventData(
        userApi.events.proofsDealer.NewChallengeCycleInitialised,
        await userApi.query.system.events()
      );

    const nextChallengeDeadline2 = await userApi.block.skipToChallengePeriod(
      nextChallengeDeadline1.toNumber(),
      bspApi.shConsts.DUMMY_BSP_ID
    );

    // Wait for provider to be slashed.
    await userApi.assert.providerSlashed(bspApi.shConsts.DUMMY_BSP_ID);

    // Check that the provider is no longer slashable.
    const slashableProvidersAfterSlash = await userApi.query.proofsDealer.slashableProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    strictEqual(slashableProvidersAfterSlash.isNone, true);

    // Simulate 2 failed challenge periods
    await userApi.block.skipToChallengePeriod(
      nextChallengeDeadline2,
      userApi.shConsts.DUMMY_BSP_ID
    );

    // Wait for provider to be slashed.
    await userApi.assert.providerSlashed(userApi.shConsts.DUMMY_BSP_ID);
  });
});
