import assert from "node:assert";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
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
  ShConsts
} from "../../../util";

describeBspNet("BSPNet: Mulitple BSP Volunteering - 3", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("multiple BSPs volunteer to multiple requests", async () => {
    // Replicate to 3 BSPs, 1 block to maxthreshold (i.e. instant acceptance)
    const maxReplicationTargetRuntimeParameter = {
      RuntimeConfig: {
        MaxReplicationTarget: [null, 3]
      }
    };
    const tickRangeToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 1]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter))
      ]
    });
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)
        )
      ]
    });

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

    const signers = [alice, bob, charlie];
    const signedExts: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];

    for (const signer of signers) {
      const bucketEvent = await api.file.newBucket("multi-bsp-multi-req", signer);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
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
          [ShConsts.NODE_INFOS.user.expectedPeerId],
          {
            LowSecurity: null
          }
        )
        .signAsync(signer);

      signedExts.push(signedExt);
    }

    await api.block.seal({ calls: signedExts });

    await api.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true,
      assertLength: 9,
      timeout: 10000
    });
  });
});
