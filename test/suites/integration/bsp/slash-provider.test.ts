import assert, { strictEqual } from "node:assert";
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

    // Loop slashing until capacity is 0
    let nextChallengeDeadline = nextChallengeDeadline2;
    let capacity = await userApi.call.storageProvidersApi.queryStorageProviderCapacity(
      bspApi.shConsts.DUMMY_BSP_ID
    );

    while (capacity.toNumber() > 0) {
      // Skip to next challenge period to trigger SlashableProvider event
      nextChallengeDeadline = await userApi.block.skipToChallengePeriod(
        nextChallengeDeadline,
        bspApi.shConsts.DUMMY_BSP_ID
      );

      // Wait for provider to be slashed
      await userApi.assert.providerSlashed(bspApi.shConsts.DUMMY_BSP_ID);

      // Query updated capacity
      capacity = await userApi.call.storageProvidersApi.queryStorageProviderCapacity(
        bspApi.shConsts.DUMMY_BSP_ID
      );
    }

    // Skip to next challenge period - this will emit SlashableProvider event from runtime
    // but the client should NOT submit a slash extrinsic since capacity is already 0
    await userApi.block.skipToChallengePeriod(nextChallengeDeadline, bspApi.shConsts.DUMMY_BSP_ID);

    try {
      await userApi.assert.providerSlashed(bspApi.shConsts.DUMMY_BSP_ID);
      assert.fail("Provider should not be slashed when capacity is 0");
    } catch (e) {
      // Expected error
    }
  });
});
