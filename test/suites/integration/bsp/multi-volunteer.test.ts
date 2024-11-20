import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import invariant from "tiny-invariant";
import {
  alice,
  bob,
  bspThreeKey,
  bspThreeSeed,
  bspTwoKey,
  bspTwoSeed,
  charlie,
  describeBspNet,
  type EnrichedBspApi,
  ShConsts,
  shUser,
  sleep
} from "../../../util";
import { strictEqual } from "node:assert";
import { isDeepStrictEqual } from "node:util";

describeBspNet("BSPNet: Mulitple BSP Volunteering - 1", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  // Test below seems to be failing. sh-bsp isn't volunteering to requests even though logs claim to
  it("single BSP volunteers to multiple requests", async () => {
    const signers = [alice, bob, charlie];
    const signedExts: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];

    for (const signer of signers) {
      const bucketEvent = await api.file.newBucket("single-bsp-multi-req", signer);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

      invariant(newBucketEventDataBlob, "Event doesn't match Type");

      const fileMetadata = await api.rpc.storagehubclient.loadFileInStorage(
        "res/smile.jpg",
        "cat/smile.jpg",
        signer.address,
        newBucketEventDataBlob.bucketId
      );

      const signedExt = await api.tx.fileSystem
        .issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          "cat/smile.jpg",
          fileMetadata.fingerprint,
          fileMetadata.file_size,
          ShConsts.DUMMY_MSP_ID,
          [ShConsts.NODE_INFOS.user.expectedPeerId]
        )
        .signAsync(signer);

      signedExts.push(signedExt);
    }

    await api.sealBlock(signedExts);

    await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true,
      assertLength: 3,
      timeout: 5000
    });
  });
});

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
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });

    await api.docker.onboardBsp({
      bspSigner: bspThreeKey,
      name: "sh-bsp-three",
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    const bucketEvent = await api.file.newBucket("multi-bsp-single-req");
    const newBucketEventDataBlob =
      api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

    invariant(newBucketEventDataBlob, "Event doesn't match Type");

    const fileMetadata = await api.rpc.storagehubclient.loadFileInStorage(
      "res/adolphus.jpg",
      "cat/adolphus.jpg",
      shUser.address,
      newBucketEventDataBlob.bucketId
    );

    const signedExt = await api.tx.fileSystem
      .issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        "cat/adolphus.jpg",
        fileMetadata.fingerprint,
        fileMetadata.file_size,
        ShConsts.DUMMY_MSP_ID,
        [ShConsts.NODE_INFOS.user.expectedPeerId]
      )
      .signAsync(shUser);

    await api.sealBlock(signedExt);

    // Waits for all three BSPs to volunteer
    await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true,
      assertLength: 3,
      timeout: 5000
    });
    await api.sealBlock();

    // Wait for a bsp to confirm storage, and check that the other BSPs failed the race
    await api.wait.bspStored();
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

describeBspNet("BSPNet: Mulitple BSP Volunteering - 3", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("multiple BSPs volunteer to multiple requests", async () => {
    // Replicate to 3 BSPs, 1 block to maxthreshold (i.e. instant acceptance)
    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(3, 1)));

    await api.docker.onboardBsp({
      bspSigner: bspTwoKey,
      name: "sh-bsp-two",
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });

    await api.docker.onboardBsp({
      bspSigner: bspThreeKey,
      name: "sh-bsp-three",
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    const signers = [alice, bob, charlie];
    const signedExts: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];

    for (const signer of signers) {
      const bucketEvent = await api.file.newBucket("multi-bsp-multi-req", signer);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

      invariant(newBucketEventDataBlob, "Event doesn't match Type");

      const fileMetadata = await api.rpc.storagehubclient.loadFileInStorage(
        "res/cloud.jpg",
        "cat/cloud.jpg",
        signer.address,
        newBucketEventDataBlob.bucketId
      );

      const signedExt = await api.tx.fileSystem
        .issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          "cat/cloud.jpg",
          fileMetadata.fingerprint,
          fileMetadata.file_size,
          ShConsts.DUMMY_MSP_ID,
          [ShConsts.NODE_INFOS.user.expectedPeerId]
        )
        .signAsync(signer);

      signedExts.push(signedExt);
    }

    await api.sealBlock(signedExts);

    // Allow time for BSP to react
    await sleep(5000);
    const matchedExts = await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true
    });

    strictEqual(matchedExts.length, 9, "Expected 9 bspVolunteer extrinsics from three BSPs");
  });
});
