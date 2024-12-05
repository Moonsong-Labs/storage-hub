import { strictEqual } from "node:assert";
import { isDeepStrictEqual } from "node:util";
import {
  bspThreeKey,
  bspThreeSeed,
  bspTwoKey,
  bspTwoSeed,
  describeBspNet,
  type EnrichedBspApi,
  ShConsts
} from "../../../util";

describeBspNet("BSPNet: Mulitple BSP Volunteering - 2", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("multiple BSPs race to volunteer for single file", async () => {
    // Replicate to 1 BSPs, 1 block to maxthreshold (i.e. instant acceptance)
    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(1, 1)));

    await api.docker.onboardBsp({
      bspSigner: bspTwoKey,
      name: "sh-bsp-two",
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"],
      waitForIdle: true
    });

    await api.docker.onboardBsp({
      bspSigner: bspThreeKey,
      name: "sh-bsp-three",
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"],
      waitForIdle: true
    });

    await api.file.createBucketAndSendNewStorageRequest(
      "res/adolphus.jpg",
      "cat/adolphus.jpg",
      "multi-bsp-single-req"
    );

    // Waits for all three BSPs to volunteer
    await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true,
      assertLength: 3,
      timeout: 15000
    });
    await api.sealBlock();

    await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspConfirmStoring",
      checkTxPool: true,
      assertLength: 3,
      timeout: 15000
    });
    await api.sealBlock();

    const matchedEvents = await api.assert.eventMany("system", "ExtrinsicFailed");
    strictEqual(matchedEvents.length, 2, "Expected 2 ExtrinsicFailed events from the losing BSPs");

    const matched = matchedEvents
      .map(({ event }) => {
        return api.events.system.ExtrinsicFailed.is(event) && event.data.dispatchError.toJSON();
      })
      .map((json) => isDeepStrictEqual(json, { index: "41", error: "0x01000000" }));

    strictEqual(
      matched.length,
      2,
      "ExtrinsicFailed events should be FileSystemPallet :: StorageRequestNotFound"
    );
  });
});
