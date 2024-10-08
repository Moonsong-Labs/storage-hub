import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import { strictEqual } from "node:assert";
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
  sleep
} from "../../../../util";

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
