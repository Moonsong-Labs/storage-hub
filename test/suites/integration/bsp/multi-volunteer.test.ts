import assert from "node:assert";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import { alice, bob, charlie, describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";

describeBspNet("BSPNet: Mulitple BSP Volunteering - 1", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  // Test below seems to be failing. sh-bsp isn't volunteering to requests even though logs claim to
  it("single BSP volunteers to multiple requests", async () => {
    // 1 block to maxthreshold (i.e. instant acceptance)
    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(null, 1)));

    const signers = [alice, bob, charlie];
    const signedExts: SubmittableExtrinsic<"promise", ISubmittableResult>[] = [];

    for (const signer of signers) {
      const bucketEvent = await api.file.newBucket("single-bsp-multi-req", signer);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
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
          [ShConsts.NODE_INFOS.user.expectedPeerId],
          null
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
