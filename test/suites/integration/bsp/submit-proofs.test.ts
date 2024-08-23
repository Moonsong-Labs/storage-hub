import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  type BspNetApi,
  type BspNetConfig,
  closeSimpleBspNet,
  runMultipleInitialisedBspsNet,
  DUMMY_BSP_ID,
  sleep,
  assertEventMany,
  BSP_DOWN_ID,
  pauseBspContainer,
  resumeBspContainer,
  assertExtrinsicPresent
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe(`BSPNet: Many BSPs Submit Proofs (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
    let userApi: BspNetApi;
    let bspApi: BspNetApi;
    let bspTwoApi: BspNetApi;
    let bspThreeApi: BspNetApi;
    let fileData: {
      fileKey: string;
      bucketId: string;
      location: string;
      owner: string;
      fingerprint: string;
      fileSize: number;
    };

    before(async () => {
      const bspNetInfo = await runMultipleInitialisedBspsNet(bspNetConfig);
      userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
      bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      bspTwoApi = await createApiObject(`ws://127.0.0.1:${bspNetInfo?.bspTwoRpcPort}`);
      bspThreeApi = await createApiObject(`ws://127.0.0.1:${bspNetInfo?.bspThreeRpcPort}`);

      assert(bspNetInfo, "BSPNet failed to initialise");
      fileData = bspNetInfo?.fileData;
    });

    after(async () => {
      await userApi.disconnect();
      await bspApi.disconnect();
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
      await closeSimpleBspNet();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

      const bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      await bspApi.disconnect();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("Many BSPs are challenged and correctly submit proofs", async () => {
      // Calculate the next challenge tick for the BSPs. It should be the same for all BSPs,
      // since they all have the same file they were initialised with, and responded to it at
      // the same time.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks.
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.sealBlock();
      }

      // Wait for tasks to execute and for the BSPs to submit proofs.
      await sleep(500);
      // Check that there are 3 pending extrinsics from BSPs (proof submission).
      const submitProofPending = await userApi.rpc.author.pendingExtrinsics();
      strictEqual(
        submitProofPending.length,
        3,
        "There should be three pending extrinsics from BSPs (proof submission)"
      );

      // Seal one more block with the pending extrinsics.
      const blockResult = await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = assertEventMany(
        userApi,
        "proofsDealer",
        "ProofAccepted",
        blockResult.events
      );
      strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

      // Get the new last tick for which the BSP submitted a proof.
      // It should be the previous last tick plus one BSP period.
      const lastTickResultAfterProof =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(lastTickResultAfterProof.isOk);
      const lastTickBspSubmittedProofAfterProof = lastTickResultAfterProof.asOk.toNumber();
      strictEqual(
        lastTickBspSubmittedProofAfterProof,
        lastTickBspSubmittedProof + challengePeriod,
        "The last tick for which the BSP submitted a proof should be the previous last tick plus one BSP period"
      );

      // Get the new deadline for the BSP.
      // It should be the current last tick, plus one BSP period, plus the challenges tick tolerance.
      const challengesTickTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
      const newDeadline =
        lastTickBspSubmittedProofAfterProof + challengePeriod + challengesTickTolerance;
      const newDeadlineResult =
        await userApi.call.proofsDealerApi.getNextDeadlineTick(DUMMY_BSP_ID);
      assert(newDeadlineResult.isOk);
      const newDeadlineOnChain = newDeadlineResult.asOk.toNumber();
      strictEqual(
        newDeadline,
        newDeadlineOnChain,
        "The deadline should be the same as the one we just calculated"
      );
    });

    it("BSP fails to submit proof and is marked as slashable", async () => {
      // Get BSP-Down's deadline.
      const bspDownDeadlineResult =
        await userApi.call.proofsDealerApi.getNextDeadlineTick(BSP_DOWN_ID);
      assert(bspDownDeadlineResult.isOk);
      const bspDownDeadline = bspDownDeadlineResult.asOk.toNumber();

      // Get the last tick for which the BSP-Down submitted a proof before advancing to the deadline.
      const lastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(BSP_DOWN_ID);
      assert(lastTickResult.isOk);
      const lastTickBspDownSubmittedProof = lastTickResult.asOk.toNumber();

      // Advance to the deadline.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = bspDownDeadline - currentBlockNumber;
      let blockResult = null;
      for (let i = 0; i < blocksToAdvance; i++) {
        blockResult = await userApi.sealBlock();
      }

      // Expect to see a `SlashableProvider` event in the last block.
      const slashableProviderEvent = userApi.assertEvent(
        "proofsDealer",
        "SlashableProvider",
        blockResult?.events
      );
      const slashableProviderEventDataBlob =
        userApi.events.proofsDealer.SlashableProvider.is(slashableProviderEvent.event) &&
        slashableProviderEvent.event.data;
      assert(slashableProviderEventDataBlob, "Event doesn't match Type");
      strictEqual(
        slashableProviderEventDataBlob.provider.toString(),
        BSP_DOWN_ID,
        "The BSP-Down should be slashable"
      );

      // Get the last tick for which the BSP-Down submitted a proof after advancing to the deadline.
      const lastTickResultAfterSlashable =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(BSP_DOWN_ID);
      assert(lastTickResultAfterSlashable.isOk);
      const lastTickBspDownSubmittedProofAfterSlashable =
        lastTickResultAfterSlashable.asOk.toNumber();

      // The new last tick should be equal to the last tick before BSP-Down was marked as slashable plus one challenge period.
      const challengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      strictEqual(
        lastTickBspDownSubmittedProofAfterSlashable,
        lastTickBspDownSubmittedProof + challengePeriod,
        "The last tick for which the BSP-Down submitted a proof should be the last tick before BSP-Down was marked as slashable plus one challenge period"
      );
    });

    it(
      "BSP stops storing last file",
      { skip: "Not implemented yet. Needs RPC method to build proofs." },
      async () => {
        // TODO: Build inclusion forest proof for file.
        // TODO: BSP-Three sends transaction to stop storing the only file it has.
        console.log(fileData);
        // // Build transaction for BSP-Three to stop storing the only file it has.
        // const call = bspThreeApi.sealBlock(
        //   bspThreeApi.tx.fileSystem.bspStopStoring(
        //     fileData.fileKey,
        //     fileData.bucketId,
        //     fileData.location,
        //     fileData.owner,
        //     fileData.fingerprint,
        //     fileData.fileSize,
        //     false
        //   ),
        //   bspThreeKey
        // );
        await it("BSP is not challenged any more", async () => {
          // TODO: Check that BSP-Three no longer has a challenge deadline.
        });
      }
    );

    it(
      "BSP submits proof, transaction gets dropped, BSP-resubmits and succeeds",
      { skip: "Dropping transactions is not implemented as testing utility yet." },
      async () => {}
    );

    it("New storage request sent by user", async () => {
      // Pause BSP-Two and BSP-Three.
      await pauseBspContainer("sh-bsp-two");
      await pauseBspContainer("sh-bsp-three");

      // Send transaction to create new storage request.
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      await userApi.sendNewStorageRequest(source, location, bucketName);

      await it("Only one BSP confirms it", async () => {
        // Wait for the remaining BSP to volunteer.
        await sleep(500);

        const volunteerPending = await assertExtrinsicPresent(userApi, {
          module: "fileSystem",
          method: "bspVolunteer",
          checkTxPool: true
        });
        strictEqual(
          volunteerPending.length,
          1,
          "There should only be one volunteer transaction, from the remaining BSP"
        );

        await userApi.sealBlock();

        // Wait for the BSP to download the file.
        await sleep(5000);
        const confirmPending = await assertExtrinsicPresent(userApi, {
          module: "fileSystem",
          method: "bspConfirmStoring",
          checkTxPool: true
        });
        strictEqual(
          confirmPending.length,
          1,
          "There should only be one confirm transaction, from the remaining BSP"
        );

        await userApi.sealBlock();

        // Wait for the BSP to process the confirmation of the file.
        await sleep(1000);
      });

      await it("BSP correctly responds to challenge with new forest root", async () => {
        // Resume BSP-Two and BSP-Three.
        await resumeBspContainer("sh-bsp-two");
        await resumeBspContainer("sh-bsp-three");

        await userApi.sealBlock();

        // TODO: Advance to next challenge block.
        // TODO: Build block with proof submission.
        // TODO: Check that proof submission was successful.
      });
    });

    it(
      "Custom challenge is added",
      { skip: "Not implemented yet. All files have the same files." },
      async () => {
        await it("Custom challenge is included in checkpoint challenge round", async () => {
          // TODO: Send transaction for custom challenge with new file key.
          // TODO: Advance until next checkpoint challenge block.
          // TODO: Check that custom challenge was included in checkpoint challenge round.
        });

        await it("BSP that has it responds to custom challenge with proof of inclusion", async () => {
          // TODO: Advance until next challenge for BSP.
          // TODO: Build block with proof submission.
          // TODO: Check that proof submission was successful, including the custom challenge.
        });

        await it("BSPs who don't have it respond non-inclusion proof", async () => {
          // TODO: Advance until next challenge for BSP-Two and BSP-Three.
          // TODO: Build block with proof submission.
          // TODO: Check that proof submission was successful, with proof of non-inclusion.
        });
      }
    );

    it(
      "File is deleted by user",
      { skip: "Not implemented yet. All files have the same files." },
      async () => {
        // TODO: Send transaction to delete file.
        // TODO: Advance until file deletion request makes it into the priority challenge round.

        await it("Priority challenge is included in checkpoint challenge round", async () => {
          // TODO: Advance to next checkpoint challenge block.
          // TODO: Check that priority challenge was included in checkpoint challenge round.
        });

        await it("BSP that has it responds to priority challenge with proof of inclusion", async () => {
          // TODO: Advance to next challenge block.
          // TODO: Build block with proof submission.
          // TODO: Check that proof submission was successful, with proof of inclusion.
        });
        await it("File is deleted by BSP", async () => {
          // TODO: Check that file is deleted by BSP, and no longer is in the Forest.
          // TODO: Check that file is deleted by BSP, and no longer is in the File System.
        });

        await it("BSPs who don't have it respond non-inclusion proof", async () => {
          // TODO: Advance to next challenge block.
          // TODO: Build block with proof submission.
          // TODO: Check that proof submission was successful, with proof of non-inclusion.
        });
      }
    );
  });
}
