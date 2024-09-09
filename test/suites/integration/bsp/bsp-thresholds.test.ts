import assert from "node:assert";
import { addBsp, bspTwoKey, bspTwoSeed, describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";

describeBspNet("BSPNet: BSP Volunteering Thresholds", { initialised: false, keepAlive: false }, ({ before, it, createUserApi, beforeEach }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  beforeEach(async () => {
    const replicationTarget = 1
    const blocksToMax = 1
    const maxThreshold = ShConsts.U32_MAX

    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.fileSystem.setGlobalParameters(replicationTarget, maxThreshold, blocksToMax)
      )
    )

  })

  it("Reputation increased on successful storage", { skip: "Not Implemented" }, async () => {
    const repBefore = (await api.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID)).unwrap().reputationWeight.toBigInt()
    await api.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "buckethead");
    await api.wait.bspVolunteer()
    await api.wait.bspStored()

    const repAfter = (await api.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID)).unwrap().reputationWeight.toBigInt()

    assert(repAfter > repBefore, "Reputation should increase after successful storage request fufilled")
    console.log(`Reputation increased from ${repBefore} to ${repAfter}`)
  });


  it("zero reputation can still volunteer and be accepted", { skip: "Not Implemented, requires forceOnboardBSP with arbitrary starting rep" }, async () => {
      // Create a new BSP and onboard with no reputation
      // Set global params to small numbers
      // Create a new storage request
      // Verify that it still eventually is able to volunteer and store against the other BSPs
  });

  // Not sure if this is good test? the times are very dependent on threshold created
  it("BSP two eventually volunteers after threshold curve is met", async () => {
    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(2, ShConsts.U32_MAX, 10)))

    await addBsp(api, bspTwoKey, {
      name: "sh-bsp-two",
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });

    await api.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "buckethead"); // T0
    await api.sealBlock() // T1
    await api.sealBlock() // T2
    await api.wait.bspVolunteer() // T3
    await api.wait.bspStored() // T4

    await api.sealBlock() // T5
    await api.wait.bspVolunteer() // T6
    await api.wait.bspStored() // T7
  })
  // bsp with reputation is prioritised


  // threhold globals can be changed 

  // Threshold req relaxes over blocks elapsed


});
