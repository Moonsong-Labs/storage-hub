import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import invariant from "tiny-invariant";
import {
  alice,
  bob,
  charlie,
  describeBspNet,
  type EnrichedShApi,
  ShConsts
} from "../../../../util";

describeBspNet("BSPNet: Mulitple BSP Volunteering - 1", ({ before, it, createUserApi }) => {
  let api: EnrichedShApi;

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
