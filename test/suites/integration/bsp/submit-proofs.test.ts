import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import {
  assertEventMany,
  assertExtrinsicPresent,
  BSP_DOWN_ID,
  createApiObject,
  describeBspNet,
  DUMMY_BSP_ID,
  NODE_INFOS,
  pauseBspContainer,
  resumeBspContainer,
  BSP_TWO_ID,
  assertEventPresent,
  shUser,
  BSP_THREE_ID,
  sleep,
  type BspNetApi
} from "../../../util";

describeBspNet(
  "Many BSPs Submit Proofs",
  { initialised: "multi", networkConfig: "standard", only: true },
  ({ before, createUserApi, after, it, getLaunchResponse }) => {
    let userApi: BspNetApi;
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
    let oneBspFileData: {
      fileKey: string;
      bucketId: string;
      location: string;
      owner: string;
      fingerprint: string;
      fileSize: number;
    };

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(launchResponse, "BSPNet failed to initialise");
      fileData = launchResponse.fileData;
      userApi = await createUserApi();
      bspTwoApi = await createApiObject(`ws://127.0.0.1:${launchResponse?.bspTwoRpcPort}`);
      bspThreeApi = await createApiObject(`ws://127.0.0.1:${launchResponse?.bspThreeRpcPort}`);
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
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
      // Finally, advance to the next challenge tick.
      await userApi.advanceToBlock(nextChallengeTick);

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

      // Assert for the event of the proof successfully submitted and verified.
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
      // Finally, advance to the next challenge tick.
      const blockResult = await userApi.advanceToBlock(bspDownDeadline);

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
      }
    );

    it("BSP is not challenged any more", { skip: "Not implemented yet." }, async () => {
      // TODO: Check that BSP-Three no longer has a challenge deadline.
    });

    it(
      "BSP submits proof, transaction gets dropped, BSP-resubmits and succeeds",
      { skip: "Dropping transactions is not implemented as testing utility yet." },
      async () => {}
    );

    it("New storage request sent by user, to only one BSP", async () => {
      // Pause BSP-Two and BSP-Three.
      await pauseBspContainer("sh-bsp-two");
      await pauseBspContainer("sh-bsp-three");

      // Send transaction to create new storage request.
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      const fileData = await userApi.sendNewStorageRequest(source, location, bucketName);
      oneBspFileData = fileData;
    });

    it("Only one BSP confirms it", async () => {
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

    it("BSP correctly responds to challenge with new forest root", async () => {
      // Advance to two challenge periods ahead for first BSP.
      // This is because in the odd case that we're exactly on the next challenge tick right now,
      // there is a race condition chance where the BSP will send the submit proof extrinsic in the
      // next block, since the Forest write lock is released as a consequence of the confirm storing
      // extrinsic. So we advance two challenge periods ahead to be sure.

      // First we get the last tick for which the BSP submitted a proof.
      const lastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate two challenge ticks ahead.
      const nextChallengeTick = lastTickBspSubmittedProof + 2 * challengePeriod;
      // Finally, advance two challenge ticks ahead.
      await userApi.advanceToBlock(nextChallengeTick);

      // Wait for BSP to submit proof.
      await sleep(1000);

      // There should be at least one pending submit proof transaction.
      const submitProofsPending = await assertExtrinsicPresent(userApi, {
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      assert(submitProofsPending.length > 0);

      // Seal block and check that the transaction was successful.
      const blockResult = await userApi.sealBlock();

      // Assert for the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = assertEventMany(
        userApi,
        "proofsDealer",
        "ProofAccepted",
        blockResult.events
      );
      strictEqual(
        proofAcceptedEvents.length,
        submitProofsPending.length,
        "All pending submit proof transactions should have been successful"
      );
    });

    it("Resume BSPs, and they shouldn't volunteer for the expired storage request", async () => {
      // Advance a number of blocks up to when the storage request times out for sure.
      const storageRequestTtl = Number(userApi.consts.fileSystem.storageRequestTtl);
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      await userApi.advanceToBlock(currentBlockNumber + storageRequestTtl, {
        waitForBspProofs: [DUMMY_BSP_ID]
      });

      // Resume BSP-Two and BSP-Three.
      await resumeBspContainer({ containerName: "sh-bsp-two" });
      await resumeBspContainer({ containerName: "sh-bsp-three" });

      // Wait for BSPs to resync.
      await sleep(3000);

      // There shouldn't be any pending volunteer transactions.
      await assert.rejects(
        async () => {
          await assertExtrinsicPresent(userApi, {
            module: "fileSystem",
            method: "bspVolunteer",
            checkTxPool: true
          });
        },
        (err: Error) => {
          const firstThreeWords = err.message.split(" ").slice(0, 3).join(" ");
          return firstThreeWords === "No extrinsics matching";
        },
        "There should be no pending volunteer transactions"
      );
    });

    it("BSP-Two still correctly responds to challenges with same forest root", async () => {
      // Advance some blocks to allow the BSP to process the challenges and submit proofs.
      for (let i = 0; i < 20; i++) {
        await userApi.sealBlock();
        await sleep(500);
      }

      // Advance to next challenge tick for BSP-Two.
      // First we get the last tick for which the BSP submitted a proof.
      const lastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(BSP_TWO_ID);
      assert(lastTickResult.isOk);
      const lastTickBspTwoSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(BSP_TWO_ID);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspTwoSubmittedProof + challengePeriod;
      // Finally, advance to the next challenge tick.
      await userApi.advanceToBlock(nextChallengeTick);

      // Wait for tasks to execute and for the BSPs to submit proofs.
      await sleep(500);

      // There should be at least one pending submit proof transaction.
      const submitProofsPending = await assertExtrinsicPresent(userApi, {
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      assert(submitProofsPending.length > 0);

      // Seal block and check that the transaction was successful.
      const blockResult = await userApi.sealBlock();

      // Assert for the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = assertEventMany(
        userApi,
        "proofsDealer",
        "ProofAccepted",
        blockResult.events
      );
      strictEqual(
        proofAcceptedEvents.length,
        submitProofsPending.length,
        "All pending submit proof transactions should have been successful"
      );
    });

    it(
      "Custom challenge is added",
      { skip: "Not implemented yet. All BSPs have the same files." },
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

    it("File is deleted by user", async () => {
      // User sends file deletion request.
      const deleteFileExtrinsicResult = await userApi.sealBlock(
        userApi.tx.fileSystem.deleteFile(
          oneBspFileData.bucketId,
          oneBspFileData.fileKey,
          oneBspFileData.location,
          oneBspFileData.fileSize,
          oneBspFileData.fingerprint,
          null
        ),
        shUser
      );

      // Check for a file deletion request event.
      assertEventPresent(
        userApi,
        "fileSystem",
        "FileDeletionRequest",
        deleteFileExtrinsicResult.events
      );

      // Advance until the deletion request expires so that it can be processed.
      const deletionRequestTtl = Number(userApi.consts.fileSystem.pendingFileDeletionRequestTtl);
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const deletionRequestEnqueuedResult = await userApi.advanceToBlock(
        currentBlockNumber + deletionRequestTtl,
        {
          waitForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
        }
      );

      // Check for a file deletion request event.
      assertEventPresent(
        userApi,
        "fileSystem",
        "PriorityChallengeForFileDeletionQueued",
        deletionRequestEnqueuedResult.events
      );
    });

    it("Priority challenge is included in checkpoint challenge round", async () => {
      // Advance to next checkpoint challenge block.
      const checkpointChallengePeriod = Number(
        userApi.consts.proofsDealer.checkpointChallengePeriod
      );
      const lastCheckpointChallengeTick = Number(
        await userApi.call.proofsDealerApi.getLastCheckpointChallengeTick()
      );
      const nextCheckpointChallengeBlock = lastCheckpointChallengeTick + checkpointChallengePeriod;
      const checkpointChallengeBlockResult = await userApi.advanceToBlock(
        nextCheckpointChallengeBlock,
        {
          waitForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
        }
      );

      // Check that the event for the priority challenge is emitted.
      const newCheckpointChallengesEvent = assertEventPresent(
        userApi,
        "proofsDealer",
        "NewCheckpointChallenge",
        checkpointChallengeBlockResult.events
      );

      // Check that the file key is in the included checkpoint challenges.
      const newCheckpointChallengesEventDataBlob =
        userApi.events.proofsDealer.NewCheckpointChallenge.is(newCheckpointChallengesEvent.event) &&
        newCheckpointChallengesEvent.event.data;
      assert(newCheckpointChallengesEventDataBlob, "Event doesn't match Type");
      let containsFileKey = false;
      for (const checkpointChallenge of newCheckpointChallengesEventDataBlob.challenges) {
        if (checkpointChallenge[0].toHuman() === oneBspFileData.fileKey) {
          containsFileKey = true;
          break;
        }
      }
      assert(containsFileKey, "The file key should be included in the checkpoint challenge.");
    });

    it("BSP that has the file responds with correct proof including the file key, and BSP that doesn't have the file responds with correct proof non-including the file key", async () => {
      // Check who has a challenge tick coming up first: the BSP that has the file or BSP-Two who doesn't have it.
      // Whoever has the challenge tick first, we check that they submitted a proof successfully first.
      const currentTick = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();

      // Calculate next challenge tick for the BSP that has the file.
      // We first get the last tick for which the BSP submitted a proof.
      const dummyBspLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(dummyBspLastTickResult.isOk);
      const lastTickBspSubmittedProof = dummyBspLastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const dummyBspChallengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
      assert(dummyBspChallengePeriodResult.isOk);
      const dummyBspChallengePeriod = dummyBspChallengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      let dummyBspNextChallengeTick = lastTickBspSubmittedProof + dummyBspChallengePeriod;
      // If it is exactly equal to the current tick, we take the next challenge tick.
      if (dummyBspNextChallengeTick === currentTick) {
        dummyBspNextChallengeTick += dummyBspChallengePeriod;
      }

      // Calculate next challenge tick for BSP-Two.
      // We first get the last tick for which the BSP submitted a proof.
      const bspTwoLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(BSP_TWO_ID);
      assert(bspTwoLastTickResult.isOk);
      const bspTwoLastTickBspTwoSubmittedProof = bspTwoLastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const bspTwoChallengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(BSP_TWO_ID);
      assert(bspTwoChallengePeriodResult.isOk);
      const bspTwoChallengePeriod = bspTwoChallengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      let bspTwoNextChallengeTick = bspTwoLastTickBspTwoSubmittedProof + bspTwoChallengePeriod;
      // If it is exactly equal to the current tick, we take the next challenge tick.
      if (bspTwoNextChallengeTick === currentTick) {
        bspTwoNextChallengeTick += bspTwoChallengePeriod;
      }

      const firstBspToRespond =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick ? DUMMY_BSP_ID : BSP_TWO_ID;
      const secondBspToRespond =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick ? BSP_TWO_ID : DUMMY_BSP_ID;
      const firstBlockToAdvance =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? dummyBspNextChallengeTick
          : bspTwoNextChallengeTick;
      const secondBlockToAdvance =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? bspTwoNextChallengeTick + bspTwoChallengePeriod
          : dummyBspNextChallengeTick + dummyBspChallengePeriod;

      // Advance to first next challenge block.
      await userApi.advanceToBlock(firstBlockToAdvance, {
        waitForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
      });

      // Wait for BSP to generate the proof and advance one more block.
      await sleep(500);
      const firstChallengeBlockResult = await userApi.sealBlock();

      // Check for a ProofAccepted event.
      const firstChallengeBlockEvents = assertEventPresent(
        userApi,
        "proofsDealer",
        "ProofAccepted",
        firstChallengeBlockResult.events
      );
      const firstChallengeBlockEventDataBlob =
        userApi.events.proofsDealer.ProofAccepted.is(firstChallengeBlockEvents.event) &&
        firstChallengeBlockEvents.event.data;
      assert(firstChallengeBlockEventDataBlob, "Event doesn't match Type");
      strictEqual(
        firstChallengeBlockEventDataBlob.provider.toString(),
        firstBspToRespond,
        "The BSP should be the one who submitted the proof."
      );

      // Advance to second next challenge block.
      await userApi.advanceToBlock(secondBlockToAdvance, {
        waitForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
      });

      // Wait for BSP to generate the proof and advance one more block.
      await sleep(500);
      const secondChallengeBlockResult = await userApi.sealBlock();

      // Check for a ProofAccepted event.
      const secondChallengeBlockEvents = assertEventPresent(
        userApi,
        "proofsDealer",
        "ProofAccepted",
        secondChallengeBlockResult.events
      );
      const secondChallengeBlockEventDataBlob =
        userApi.events.proofsDealer.ProofAccepted.is(secondChallengeBlockEvents.event) &&
        secondChallengeBlockEvents.event.data;
      assert(secondChallengeBlockEventDataBlob, "Event doesn't match Type");
      strictEqual(
        secondChallengeBlockEventDataBlob.provider.toString(),
        secondBspToRespond,
        "The BSP should be the one who submitted the proof."
      );
    });

    it("File is removed from Forest by BSP", { skip: "Not implemented yet." }, async () => {
      // TODO: Check that file is deleted by BSP, and no longer is in the Forest.
    });

    it(
      "File mutation is finalised and BSP removes it from File Storage",
      { skip: "Not implemented yet." },
      async () => {
        // TODO: Finalise block with mutations.
        // TODO: Check that file is removed from File Storage.
      }
    );
  }
);
