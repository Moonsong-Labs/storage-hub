import assert, { strictEqual } from "node:assert";
import {
  describeBspNet,
  shUser,
  sleep,
  type EnrichedBspApi,
  type FileMetadata,
  ShConsts,
  bspThreeKey
} from "../../../util";
import { BSP_THREE_ID, BSP_TWO_ID, DUMMY_BSP_ID, NODE_INFOS } from "../../../util/bspNet/consts";

describeBspNet(
  "BSP: Many BSPs Submit Proofs",
  { initialised: "multi", networkConfig: "standard", only: true },
  ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;
    let fileData: FileMetadata;
    let oneBspFileData: FileMetadata;
    let rootBeforeDeletion: string;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(launchResponse, "BSPNet failed to initialise");
      fileData = launchResponse.fileData;
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspThreeRpcPort}`);
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("Many BSPs are challenged and correctly submit proofs", async () => {
      // Calculate the next challenge tick for the BSPs. It should be the same for all BSPs,
      // since they all have the same file they were initialised with, and responded to it at
      // the same time.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;
      // Finally, advance to the next challenge tick.
      await userApi.advanceToBlock(nextChallengeTick);

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 3,
        timeout: 10000
      });

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
      strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

      // Get the new last tick for which the BSP submitted a proof.
      // It should be the previous last tick plus one BSP period.
      const lastTickResultAfterProof =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.DUMMY_BSP_ID
        );
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
      const newDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
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
      const bspDownDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.BSP_DOWN_ID
      );
      assert(bspDownDeadlineResult.isOk);
      const bspDownDeadline = bspDownDeadlineResult.asOk.toNumber();

      // Get the last tick for which the BSP-Down submitted a proof before advancing to the deadline.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.BSP_DOWN_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspDownSubmittedProof = lastTickResult.asOk.toNumber();
      // Finally, advance to the next challenge tick.
      await userApi.advanceToBlock(bspDownDeadline);

      // Expect to see a `SlashableProvider` event in the last block.
      const slashableProviderEvent = await userApi.assert.eventPresent(
        "proofsDealer",
        "SlashableProvider"
      );
      const slashableProviderEventDataBlob =
        userApi.events.proofsDealer.SlashableProvider.is(slashableProviderEvent.event) &&
        slashableProviderEvent.event.data;
      assert(slashableProviderEventDataBlob, "Event doesn't match Type");
      strictEqual(
        slashableProviderEventDataBlob.provider.toString(),
        userApi.shConsts.BSP_DOWN_ID,
        "The BSP-Down should be slashable"
      );

      // Get the last tick for which the BSP-Down submitted a proof after advancing to the deadline.
      const lastTickResultAfterSlashable =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.BSP_DOWN_ID
        );
      assert(lastTickResultAfterSlashable.isOk);
      const lastTickBspDownSubmittedProofAfterSlashable =
        lastTickResultAfterSlashable.asOk.toNumber();

      // The new last tick should be equal to the last tick before BSP-Down was marked as slashable plus one challenge period.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      strictEqual(
        lastTickBspDownSubmittedProofAfterSlashable,
        lastTickBspDownSubmittedProof + challengePeriod,
        "The last tick for which the BSP-Down submitted a proof should be the last tick before BSP-Down was marked as slashable plus one challenge period"
      );
    });

    it("BSP stops storing last file", async () => {
      const inclusionForestProof = await bspThreeApi.rpc.storagehubclient.generateForestProof(
        null,
        [fileData.fileKey]
      );
      // Build transaction for BSP-Three to stop storing the only file it has.
      await userApi.sealBlock(
        bspThreeApi.tx.fileSystem.bspRequestStopStoring(
          fileData.fileKey,
          fileData.bucketId,
          fileData.location,
          fileData.owner,
          fileData.fingerprint,
          fileData.fileSize,
          false,
          inclusionForestProof.toString()
        ),
        bspThreeKey
      );

      userApi.assert.fetchEventData(
        userApi.events.fileSystem.BspRequestedToStopStoring,
        await userApi.query.system.events()
      );
    });

    it(
      "BSP can correctly delete a file from its forest and runtime correctly updates its root",
      { skip: "Not implemented yet. Needs RPC method to build proofs." },
      async () => {
        // TODO: Setup a BSP that has two files which lie under the same NibbledBranch in the forest.
        // TODO: Generate the proof to delete one of the files.
        /* let inclusionForestProof = bspThreeApi.rpc.storagehubclient.buildForestRoot(fileData.fileKey); */
        // TODO: Request the deletion of the file:
        /* const fileDeletionRequestResult = bspThreeApi.sealBlock(bspThreeApi.tx.fileSystem.bspRequestStopStoring(
            fileData.fileKey,
            fileData.bucketId,
            fileData.location,
            fileData.owner,
            fileData.fingerprint,
            fileData.fileSize,
            false,
            inclusion_forest_proof: ForestProof<T>,
        ); */
        // Wait enough blocks for the deletion to be allowed.
        /* const currentBlock = await bspThreeApi.rpc.chain.getBlock();
		const currentBlockNumber = currentBlock.block.header.number.toNumber();
		const cooldown = currentBlockNumber + bspThreeApi.consts.fileSystem.minWaitForStopStoring.toNumber();
		await bspThreeApi.advanceToBlock(cooldown); */
        // TODO: Confirm the request of deletion. Make sure the extrinsic doesn't fail and the root is updated correctly.
        /*  const fileDeletionConfirmResult = bspThreeApi.sealBlock(bspThreeApi.tx.fileSystem.bspConfirmStopStoring(
				fileData.fileKey,
				inclusionForestProof,
			)); 
			// Check for the confirm stopped storing event.
      		let confirmStopStoringEvent = bspThreeApi.assert.eventPresent(
        		"fileSystem",
       			"BspConfirmStoppedStoring",
        		fileDeletionConfirmResult.events
      		);
			// Make sure the new root was updated correctly.
			bspThreeApi.rpc.storagehubclient.deleteFile(fileData.fileKey); // Not sure if this is the correct way to do it.
			const newRoot = bspThreeApi.rpc.storagehubclient.getForestRoot();
			const newRootInRuntime = confirmStopStoringEvent.event.data.newRoot;
			assert(newRoot === newRootInRuntime, "The new root should be updated correctly");
		*/
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
      await userApi.docker.pauseBspContainer("sh-bsp-two");
      await userApi.docker.pauseBspContainer("sh-bsp-three");

      // Send transaction to create new storage request.
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      const fileData = await userApi.file.newStorageRequest(source, location, bucketName);
      oneBspFileData = fileData;
    });

    it("Only one BSP confirms it", async () => {
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored(1, NODE_INFOS.bsp.AddressId );
    });

    it("BSP correctly responds to challenge with new forest root", async () => {
      // Advance to two challenge periods ahead for first BSP.
      // This is because in the odd case that we're exactly on the next challenge tick right now,
      // there is a race condition chance where the BSP will send the submit proof extrinsic in the
      // next block, since the Forest write lock is released as a consequence of the confirm storing
      // extrinsic. So we advance two challenge periods ahead to be sure.

      // First we get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        ShConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate two challenge ticks ahead.
      const nextChallengeTick = lastTickBspSubmittedProof + 2 * challengePeriod;
      // Finally, advance two challenge ticks ahead.
      await userApi.advanceToBlock(nextChallengeTick);

      // Wait for BSP to submit proof.
      await sleep(1000);

      // There should be at least one pending submit proof transaction.
      const submitProofsPending = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      assert(submitProofsPending.length > 0);

      // Seal block and check that the transaction was successful.
      await userApi.sealBlock();

      // Assert for the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
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
        waitForBspProofs: [ShConsts.DUMMY_BSP_ID]
      });

      // Resume BSP-Two and BSP-Three.
      await userApi.docker.resumeBspContainer({ containerName: "sh-bsp-two" });
      await userApi.docker.resumeBspContainer({ containerName: "sh-bsp-three" });

      // Wait for BSPs to resync.
      await sleep(1000);

      // There shouldn't be any pending volunteer transactions.
      await assert.rejects(
        async () => {
          await userApi.assert.extrinsicPresent({
            module: "fileSystem",
            method: "bspVolunteer",
            checkTxPool: true
          });
        },
        /No matching extrinsic found for fileSystem\.bspVolunteer/,
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
      const submitProofsPending = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      assert(submitProofsPending.length > 0);

      // Seal block and check that the transaction was successful.
      await userApi.sealBlock();

      // Assert for the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
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
      // Get the root of the BSP that has the file before deletion.
      const bspMetadata = await userApi.query.providers.backupStorageProviders(
        ShConsts.DUMMY_BSP_ID
      );
      assert(bspMetadata, "BSP metadata should exist");
      assert(bspMetadata.isSome, "BSP metadata should be Some");
      const bspMetadataBlob = bspMetadata.unwrap();
      rootBeforeDeletion = bspMetadataBlob.root.toHex();
      // Make sure it matches the one of the actual merkle forest.
      const actualRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(
        rootBeforeDeletion,
        actualRoot.toHex(),
        "The root of the BSP should match the actual merkle forest root."
      );

      // User sends file deletion request.
      await userApi.sealBlock(
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
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequest");

      // Advance until the deletion request expires so that it can be processed.
      const deletionRequestTtl = Number(userApi.consts.fileSystem.pendingFileDeletionRequestTtl);
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      await userApi.advanceToBlock(currentBlockNumber + deletionRequestTtl, {
        waitForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID]
      });

      // Check for a file deletion request event.
      await userApi.assert.eventPresent("fileSystem", "PriorityChallengeForFileDeletionQueued");
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
      await userApi.advanceToBlock(nextCheckpointChallengeBlock, {
        waitForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID]
      });

      // Check that the event for the priority challenge is emitted.
      const newCheckpointChallengesEvent = await userApi.assert.eventPresent(
        "proofsDealer",
        "NewCheckpointChallenge"
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
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID);
      assert(dummyBspLastTickResult.isOk);
      const lastTickBspSubmittedProof = dummyBspLastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const dummyBspChallengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.DUMMY_BSP_ID
      );
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
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.BSP_TWO_ID);
      assert(bspTwoLastTickResult.isOk);
      const bspTwoLastTickBspTwoSubmittedProof = bspTwoLastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const bspTwoChallengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.BSP_TWO_ID
      );
      assert(bspTwoChallengePeriodResult.isOk);
      const bspTwoChallengePeriod = bspTwoChallengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      let bspTwoNextChallengeTick = bspTwoLastTickBspTwoSubmittedProof + bspTwoChallengePeriod;
      // If it is exactly equal to the current tick, we take the next challenge tick.
      if (bspTwoNextChallengeTick === currentTick) {
        bspTwoNextChallengeTick += bspTwoChallengePeriod;
      }

      const firstBspToRespond =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? ShConsts.DUMMY_BSP_ID
          : ShConsts.BSP_TWO_ID;
      const secondBspToRespond =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? ShConsts.BSP_TWO_ID
          : ShConsts.DUMMY_BSP_ID;
      const firstBlockToAdvance =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? dummyBspNextChallengeTick
          : bspTwoNextChallengeTick;
      const secondBlockToAdvance =
        dummyBspNextChallengeTick < bspTwoNextChallengeTick
          ? bspTwoNextChallengeTick
          : dummyBspNextChallengeTick;

      // Advance to first next challenge block.
      await userApi.advanceToBlock(firstBlockToAdvance, {
        waitForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
      });

      // Wait for BSP to generate the proof and advance one more block.
      await sleep(500);
      await userApi.sealBlock();

      // Check for a ProofAccepted event.
      const firstChallengeBlockEvents = await userApi.assert.eventMany(
        "proofsDealer",
        "ProofAccepted"
      );

      // Check that at least one of the `ProofAccepted` events belongs to `firstBspToRespond`.
      const atLeastOneEventBelongsToFirstBsp = firstChallengeBlockEvents.some((eventRecord) => {
        const firstChallengeBlockEventDataBlob =
          userApi.events.proofsDealer.ProofAccepted.is(eventRecord.event) && eventRecord.event.data;
        assert(firstChallengeBlockEventDataBlob, "Event doesn't match Type");

        return firstChallengeBlockEventDataBlob.provider.toString() === firstBspToRespond;
      });
      assert(atLeastOneEventBelongsToFirstBsp, "No ProofAccepted event belongs to the first BSP");

      // If the first BSP is the one removing the file, assert for the event of the mutations successfully applied in the runtime.
      if (firstBspToRespond === ShConsts.DUMMY_BSP_ID) {
        const mutationsAppliedEvents = await userApi.assert.eventMany(
          "proofsDealer",
          "MutationsApplied"
        );
        strictEqual(
          mutationsAppliedEvents.length,
          1,
          "There should be one mutations applied event"
        );

        // Check that the mutations applied event belongs to the dummy BSP.
        const mutationsAppliedEventDataBlob =
          userApi.events.proofsDealer.MutationsApplied.is(mutationsAppliedEvents[0].event) &&
          mutationsAppliedEvents[0].event.data;
        assert(mutationsAppliedEventDataBlob, "Event doesn't match Type");
        strictEqual(
          mutationsAppliedEventDataBlob.provider.toString(),
          ShConsts.DUMMY_BSP_ID,
          "The mutations applied event should belong to the dummy BSP"
        );
      }

      // Advance to second next challenge block.
      await userApi.advanceToBlock(secondBlockToAdvance, {
        waitForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID]
      });

      // Wait for BSP to generate the proof and advance one more block.
      await sleep(500);
      const secondChallengeBlockResult = await userApi.sealBlock();

      // Check for a ProofAccepted event.
      const secondChallengeBlockEvents = await userApi.assert.eventMany(
        "proofsDealer",
        "ProofAccepted",
        secondChallengeBlockResult.events
      );

      // Check that at least one of the `ProofAccepted` events belongs to `secondBspToRespond`.
      const atLeastOneEventBelongsToSecondBsp = secondChallengeBlockEvents.some((eventRecord) => {
        const secondChallengeBlockEventDataBlob =
          userApi.events.proofsDealer.ProofAccepted.is(eventRecord.event) && eventRecord.event.data;
        assert(secondChallengeBlockEventDataBlob, "Event doesn't match Type");

        return secondChallengeBlockEventDataBlob.provider.toString() === secondBspToRespond;
      });
      assert(atLeastOneEventBelongsToSecondBsp, "No ProofAccepted event belongs to the second BSP");

      // If the second BSP is the one removing the file, assert for the event of the mutations successfully applied in the runtime.
      if (secondBspToRespond === ShConsts.DUMMY_BSP_ID) {
        const mutationsAppliedEvents = await userApi.assert.eventMany(
          "proofsDealer",
          "MutationsApplied"
        );
        strictEqual(
          mutationsAppliedEvents.length,
          1,
          "There should be one mutations applied event"
        );

        // Check that the mutations applied event belongs to the dummy BSP.
        const mutationsAppliedEventDataBlob =
          userApi.events.proofsDealer.MutationsApplied.is(mutationsAppliedEvents[0].event) &&
          mutationsAppliedEvents[0].event.data;
        assert(mutationsAppliedEventDataBlob, "Event doesn't match Type");
        strictEqual(
          mutationsAppliedEventDataBlob.provider.toString(),
          ShConsts.DUMMY_BSP_ID,
          "The mutations applied event should belong to the dummy BSP"
        );
      }
    });

    it("File is removed from Forest by BSP", async () => {
      // Make sure the root was updated in the runtime
      const bspMetadataAfterDeletion = await userApi.query.providers.backupStorageProviders(
        ShConsts.DUMMY_BSP_ID
      );
      assert(bspMetadataAfterDeletion, "BSP metadata should exist");
      assert(bspMetadataAfterDeletion.isSome, "BSP metadata should be Some");
      const bspMetadataAfterDeletionBlob = bspMetadataAfterDeletion.unwrap();
      assert(
        bspMetadataAfterDeletionBlob.root.toHex() !== rootBeforeDeletion,
        "The root should have been updated on chain"
      );

      // Wait for BSP to update his local forest root.
      await sleep(500);
      // Check that the runtime root matches the forest root of the BSP.
      const forestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(
        bspMetadataAfterDeletionBlob.root.toString(),
        forestRoot.toString(),
        "The runtime root should match the forest root of the BSP"
      );
    });

    it(
      "File mutation is finalised and BSP removes it from File Storage",
      { skip: "Not implemented yet." },
      async () => {
        // TODO: Finalise block with mutations.
        // TODO: Check that file is removed from File Storage. Need a RPC method for this.
      }
    );
  }
);
