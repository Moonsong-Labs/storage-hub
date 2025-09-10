import { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedBspApi } from "../../../util";

await describeBspNet("BSPNet: Slash Provider", ({ before, createUserApi, createBspApi, it }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

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
    const initialiseChallengeCycleResult = await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(
          userApi.tx.proofsDealer.forceInitialiseChallengeCycle(bspApi.shConsts.DUMMY_BSP_ID)
        )
      ]
    });

    // Assert that event for challenge cycle initialisation is emitted.
    await userApi.assert.eventPresent(
      "proofsDealer",
      "NewChallengeCycleInitialised",
      initialiseChallengeCycleResult.events
    );

    const {
      data: { nextChallengeDeadline: nextChallengeDeadline1 }
    } = userApi.assert.fetchEvent(
      userApi.events.proofsDealer.NewChallengeCycleInitialised,
      initialiseChallengeCycleResult.events
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
