import assert from "node:assert";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { alice, bob, charlie, describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";

await describeBspNet("BSPNet: Mulitple BSP Volunteering - 1", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  // Test below seems to be failing. sh-bsp isn't volunteering to requests even though logs claim to
  it("single BSP volunteers to multiple requests", async () => {
    // 1 block to maxthreshold (i.e. instant acceptance)
    const tickToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 1]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter))
      ]
    });

    const signers = [alice, bob, charlie];
    const signedExts: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];

    for (const signer of signers) {
      const bucketEvent = await api.file.newBucket("single-bsp-multi-req", signer);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const ownerHex = u8aToHex(decodeAddress(signer.address)).slice(2);
      const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
        "res/smile.jpg",
        "cat/smile.jpg",
        ownerHex,
        newBucketEventDataBlob.bucketId
      );

      const signedExt = await api.tx.fileSystem
        .issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          "cat/smile.jpg",
          fileMetadata.fingerprint,
          fileMetadata.file_size,
          ShConsts.DUMMY_MSP_ID,
          [ShConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
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
      assertLength: 3,
      timeout: 5000
    });
  });
});
