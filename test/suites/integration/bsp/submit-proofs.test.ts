import assert, { strictEqual } from "node:assert";
import {
  bspThreeKey,
  describeBspNet,
  type EnrichedBspApi,
  type FileMetadata,
  ShConsts,
  waitFor
} from "../../../util";
import { BSP_THREE_ID, BSP_TWO_ID, DUMMY_BSP_ID, NODE_INFOS } from "../../../util/bspNet/consts";

await describeBspNet(
  "BSPNet: Many BSPs Submit Proofs",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;
    let fileMetadata: FileMetadata;
    let oneBspfileMetadata: FileMetadata;
    let rootBeforeDeletion: string;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(
        launchResponse && "bspTwoRpcPort" in launchResponse && "bspThreeRpcPort" in launchResponse,
        "BSPNet failed to initialise with required ports"
      );
      fileMetadata = launchResponse.fileMetadata;
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);
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
      await userApi.block.skipTo(nextChallengeTick);

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 3,
        timeout: 10000
      });

      // Seal one more block with the pending extrinsics.
      await userApi.block.seal();

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
      await userApi.block.skipTo(bspDownDeadline);

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
      strictEqual(
        lastTickBspDownSubmittedProofAfterSlashable,
        lastTickBspDownSubmittedProof,
        "The last tick for which the BSP-Down submitted a proof should remain the same since the BSP went down"
      );
    });

    it("BSP three stops storing last file", async () => {
      // Wait for BSP-Three to catch up to the tip of the chain
      await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);

      // Build transaction for BSP-Three to stop storing the only file it has.
      const inclusionForestProof = await bspThreeApi.rpc.storagehubclient.generateForestProof(
        null,
        [fileMetadata.fileKey]
      );
      await userApi.wait.waitForAvailabilityToSendTx(bspThreeKey.address.toString());
      const blockResult = await userApi.block.seal({
        calls: [
          bspThreeApi.tx.fileSystem.bspRequestStopStoring(
            fileMetadata.fileKey,
            fileMetadata.bucketId,
            fileMetadata.location,
            fileMetadata.owner,
            fileMetadata.fingerprint,
            fileMetadata.fileSize,
            false,
            inclusionForestProof.toString()
          )
        ],
        signer: bspThreeKey
      });
      assert(blockResult.extSuccess, "Extrinsic was part of the block so its result should exist.");
      assert(
        blockResult.extSuccess === true,
        "Extrinsic to request stop storing should have been successful"
      );

      userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspRequestedToStopStoring,
        await userApi.query.system.events()
      );
    });

    it("BSP can correctly delete a file from its forest and runtime correctly updates its root", async () => {
      // Generate the inclusion proof for the file key that BSP-Three requested to stop storing.
      const inclusionForestProof = await bspThreeApi.rpc.storagehubclient.generateForestProof(
        null,
        [fileMetadata.fileKey]
      );

      // Wait enough blocks for the deletion to be allowed.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const cooldown = currentBlockNumber + minWaitForStopStoring;
      await userApi.block.skipTo(cooldown);
      await userApi.wait.waitForAvailabilityToSendTx(bspThreeKey.address.toString());

      // Confirm the request of deletion. Make sure the extrinsic doesn't fail and the root is updated correctly.
      const block = await userApi.block.seal({
        calls: [
          bspThreeApi.tx.fileSystem.bspConfirmStopStoring(
            fileMetadata.fileKey,
            inclusionForestProof.toString()
          )
        ],
        signer: bspThreeKey
      });
      // Check for the confirm stopped storing event.
      const confirmStopStoringEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmStoppedStoring"
      );
      // Wait for confirmation line in docker logs.
      await bspThreeApi.docker.waitForLog({
        containerName: "sh-bsp-three",
        searchString: "New local Forest root matches the one in the block for BSP"
      });

      // Make sure the new root was updated correctly.
      const newRoot = (await bspThreeApi.rpc.storagehubclient.getForestRoot(null)).unwrap();
      assert(userApi.events.fileSystem.BspConfirmStoppedStoring.is(confirmStopStoringEvent.event));
      const newRootInRuntime = confirmStopStoringEvent.event.data.newRoot;

      // Make sure BSP three is caught up to the tip of the chain, and finalise the block
      // to trigger the event to delete the file.
      await bspThreeApi.wait.nodeCatchUpToChainTip(userApi);
      await bspThreeApi.block.finaliseBlock(block.blockReceipt.blockHash.toString());

      // Make sure the file is no longer in the file storage.
      await waitFor({
        lambda: async () =>
          (await bspThreeApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey))
            .isFileNotFound
      });

      // Important! Keep the string conversion to avoid a recursive call that lead to a crash in javascript.
      strictEqual(
        newRoot.toString(),
        newRootInRuntime.toString(),
        "The new root should be updated correctly"
      );
    });

    it("BSP three is not challenged any more", async () => {
      const result = await userApi.call.proofsDealerApi.getNextDeadlineTick(ShConsts.BSP_THREE_ID);

      assert(result.isErr, "BSP three doesn't have files so it shouldn't have deadline");
    });

    it("New storage request sent by user, to only one BSP", async () => {
      // Pause BSP-Two and BSP-Three.
      await userApi.docker.pauseContainer("sh-bsp-two");
      await userApi.docker.pauseContainer("sh-bsp-three");

      // Send transaction to create new storage request.
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      const fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        location,
        bucketName,
        null,
        null,
        null,
        3
      );
      oneBspfileMetadata = fileMetadata;
    });

    it("Only one BSP confirms it and the MSP accepts it", async () => {
      // Wait for the MSP acceptance of the file to be in the TX pool
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
        checkTxPool: true,
        timeout: 5000
      });

      // Then wait for the BSP volunteer to be in the TX pool and seal the block
      await userApi.wait.bspVolunteer(1);

      // Finally, wait for the BSP to confirm storing the file and seal the block
      const address = userApi.createType("Address", NODE_INFOS.bsp.AddressId);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount: address });
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
      await userApi.block.skipTo(nextChallengeTick);

      const submitProofsPending = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Seal block and check that the transaction was successful.
      await userApi.block.seal();

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
      const storageRequestTtl = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            StorageRequestTtl: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asStorageRequestTtl.toNumber();
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      await userApi.block.skipTo(currentBlockNumber + storageRequestTtl, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID]
      });

      // Resume BSP-Two and BSP-Three.
      await userApi.docker.resumeContainer({
        containerName: "sh-bsp-two"
      });
      await userApi.docker.resumeContainer({
        containerName: "sh-bsp-three"
      });

      // Wait for BSPs to resync.
      await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);
      await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);

      // There shouldn't be any pending volunteer transactions.
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
        timeout: 2000,
        assertLength: 0,
        exactLength: true
      });
    });

    it("BSP-Two still correctly responds to challenges with same forest root", async () => {
      // Advance some blocks to allow the BSP to process the challenges and submit proofs.
      await userApi.block.skip(20);

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

      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();

      if (nextChallengeTick > currentBlockNumber) {
        // Advance to the next challenge tick if needed
        await userApi.block.skipTo(nextChallengeTick, {
          watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID]
        });
      }

      // There should be two pending submit proof transactions, one per active BSP.
      const submitProofsPending = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 2,
        exactLength: true
      });

      // Seal block and check that the transaction was successful.
      await userApi.block.seal();

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

    it("Non-root user cannot initiate priority challenge", async () => {
      // Attempt to call forcePriorityChallenge without sudo
      const { events, extSuccess } = await userApi.block.seal({
        calls: [
          userApi.tx.proofsDealer.priorityChallenge(
            oneBspfileMetadata.fileKey,
            true // should_remove_key = true as test suite expects the file to be deleted.
          )
        ]
      });

      // The extrinsic should have failed.
      assert.strictEqual(
        extSuccess,
        false,
        "Non-root user should not be able to call forcePriorityChallenge"
      );

      // Get the event of the extrinsic failure.
      const {
        data: { dispatchError: eventInfo }
      } = userApi.assert.fetchEvent(userApi.events.system.ExtrinsicFailed, events);

      // Ensure it failed with BadOrigin error.
      assert.strictEqual(eventInfo.isBadOrigin, true, "Error should be BadOrigin");
    });

    it("Priority challenge is initiated for file", async () => {
      // Get the root of the BSP that has the file before priority challenge.
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

      // Sudo initiates priority challenge for file removal.
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.proofsDealer.priorityChallenge(
              oneBspfileMetadata.fileKey,
              true // should_remove_key = true as test suite expects the file to be deleted.
            )
          )
        ]
      });

      // Check that the PriorityChallenge event was emitted.
      const priorityChallengeEvent = await userApi.assert.eventPresent(
        "proofsDealer",
        "NewPriorityChallenge"
      );
      const priorityChallengeEventDataBlob =
        userApi.events.proofsDealer.NewPriorityChallenge.is(priorityChallengeEvent.event) &&
        priorityChallengeEvent.event.data;
      assert(priorityChallengeEventDataBlob, "Event doesn't match Type");
      strictEqual(
        priorityChallengeEventDataBlob.keyChallenged.toString(),
        oneBspfileMetadata.fileKey,
        "The priority challenge event should contain the correct file key"
      );
      strictEqual(
        priorityChallengeEventDataBlob.shouldRemoveKey.toString(),
        "true",
        "The priority challenge event should have shouldRemoveKey set to true"
      );
      assert(
        priorityChallengeEventDataBlob.who.isNone,
        "The priority challenge should be initiated by Root origin"
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
      await userApi.block.skipTo(nextCheckpointChallengeBlock, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID]
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
        if (checkpointChallenge.key.toHuman() === oneBspfileMetadata.fileKey) {
          containsFileKey = true;
          break;
        }
      }
      assert(containsFileKey, "The file key should be included in the checkpoint challenge.");
    });

    it("BSP that has the file responds with correct proof including the file key, and BSP that doesn't have the file responds with correct proof non-including the file key", async () => {
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
      // Increment challenge periods until we get a number that is greater than the current tick.
      const currentTick = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();
      while (currentTick > dummyBspNextChallengeTick) {
        // Go one challenge period forward.
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
      // Increment challenge periods until we get a number that is greater than the current tick.
      while (currentTick > bspTwoNextChallengeTick) {
        // Go one challenge period forward.
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

      const areBspsNextChallengeBlockTheSame = firstBlockToAdvance === secondBlockToAdvance;

      // Check if firstBlockToAdvance is equal to the current block.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      if (firstBlockToAdvance !== currentBlockNumber) {
        // Advance to first next challenge block.
        await userApi.block.skipTo(firstBlockToAdvance, {
          watchForBspProofs: [DUMMY_BSP_ID, BSP_TWO_ID, BSP_THREE_ID]
        });
      }

      // Wait for BSP to generate the proof and advance one more block.
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await userApi.block.seal();

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

        return firstChallengeBlockEventDataBlob.providerId.toString() === firstBspToRespond;
      });
      assert(atLeastOneEventBelongsToFirstBsp, "No ProofAccepted event belongs to the first BSP");

      // If the first BSP is the one removing the file, assert for the event of the mutations successfully applied in the runtime.
      if (firstBspToRespond === ShConsts.DUMMY_BSP_ID) {
        const mutationsAppliedEvents = await userApi.assert.eventMany(
          "proofsDealer",
          "MutationsAppliedForProvider"
        );
        strictEqual(
          mutationsAppliedEvents.length,
          1,
          "There should be one mutations applied event"
        );

        // Check that the mutations applied event belongs to the dummy BSP.
        const mutationsAppliedEventDataBlob =
          userApi.events.proofsDealer.MutationsAppliedForProvider.is(
            mutationsAppliedEvents[0].event
          ) && mutationsAppliedEvents[0].event.data;
        assert(mutationsAppliedEventDataBlob, "Event doesn't match Type");
        strictEqual(
          mutationsAppliedEventDataBlob.providerId.toString(),
          ShConsts.DUMMY_BSP_ID,
          "The mutations applied event should belong to the dummy BSP"
        );
      }

      // If the BSPs had different next challenge blocks, advance to the second next challenge block.
      if (!areBspsNextChallengeBlockTheSame) {
        const currentBlockNumber = (
          await userApi.rpc.chain.getBlock()
        ).block.header.number.toNumber();
        if (secondBlockToAdvance !== currentBlockNumber) {
          // Advance to second next challenge block.
          await userApi.block.skipTo(secondBlockToAdvance, {
            watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID]
          });
        }

        // Wait for BSP to generate the proof and advance one more block.
        await userApi.assert.extrinsicPresent({
          module: "proofsDealer",
          method: "submitProof",
          checkTxPool: true
        });
        await userApi.block.seal();
      }

      // Check for a ProofAccepted event.
      const secondChallengeBlockEvents = await userApi.assert.eventMany(
        "proofsDealer",
        "ProofAccepted"
      );

      // Check that at least one of the `ProofAccepted` events belongs to `secondBspToRespond`.
      const atLeastOneEventBelongsToSecondBsp = secondChallengeBlockEvents.some((eventRecord) => {
        const secondChallengeBlockEventDataBlob =
          userApi.events.proofsDealer.ProofAccepted.is(eventRecord.event) && eventRecord.event.data;
        assert(secondChallengeBlockEventDataBlob, "Event doesn't match Type");

        return secondChallengeBlockEventDataBlob.providerId.toString() === secondBspToRespond;
      });
      assert(atLeastOneEventBelongsToSecondBsp, "No ProofAccepted event belongs to the second BSP");

      // If the second BSP is the one removing the file, assert for the event of the mutations successfully applied in the runtime.
      if (secondBspToRespond === ShConsts.DUMMY_BSP_ID) {
        const mutationsAppliedEvents = await userApi.assert.eventMany(
          "proofsDealer",
          "MutationsAppliedForProvider"
        );
        strictEqual(
          mutationsAppliedEvents.length,
          1,
          "There should be one mutations applied event"
        );

        // Check that the mutations applied event belongs to the dummy BSP.
        const mutationsAppliedEventDataBlob =
          userApi.events.proofsDealer.MutationsAppliedForProvider.is(
            mutationsAppliedEvents[0].event
          ) && mutationsAppliedEvents[0].event.data;
        assert(mutationsAppliedEventDataBlob, "Event doesn't match Type");
        strictEqual(
          mutationsAppliedEventDataBlob.providerId.toString(),
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

      await waitFor({
        lambda: async () => (await bspApi.rpc.storagehubclient.getForestRoot(null)).isSome
      });

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
