// import assert, { strictEqual } from "node:assert";
// import {
//   ShConsts,
//   bspThreeKey,
//   describeBspNet,
//   type EnrichedBspApi,
//   type FileMetadata
// } from "../../../util";
// import { BSP_TWO_ID, NODE_INFOS } from "../../../util/bspNet/consts";

// TODO: Skipping this test suite until new file deletion flow is implemented.
// describeBspNet(
//   "BSP: Many BSPs Submit Proofs",
//   { initialised: "multi", networkConfig: "standard" },
//   ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
//     let userApi: EnrichedBspApi;
//     let bspApi: EnrichedBspApi;
//     let bspTwoApi: EnrichedBspApi;
//     let bspThreeApi: EnrichedBspApi;
//     let fileMetadata: FileMetadata;

//     before(async () => {
//       const launchResponse = await getLaunchResponse();
//       assert(
//         launchResponse && "bspTwoRpcPort" in launchResponse && "bspThreeRpcPort" in launchResponse,
//         "BSPNet failed to initialise with required ports"
//       );
//       fileMetadata = launchResponse.fileMetadata;
//       userApi = await createUserApi();
//       bspApi = await createBspApi();
//       bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
//       bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);
//     });

//     after(async () => {
//       await bspTwoApi.disconnect();
//       await bspThreeApi.disconnect();
//     });

//     it("Network launches and can be queried", async () => {
//       const userNodePeerId = await userApi.rpc.system.localPeerId();
//       strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
//       const bspNodePeerId = await bspApi.rpc.system.localPeerId();
//       strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
//     });

//     it("Many BSPs are challenged and correctly submit proofs", async () => {
//       // Calculate the next challenge tick for the BSPs. It should be the same for all BSPs,
//       // since they all have the same file they were initialised with, and responded to it at
//       // the same time.
//       // We first get the last tick for which the BSP submitted a proof.
//       const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
//         userApi.shConsts.DUMMY_BSP_ID
//       );
//       assert(lastTickResult.isOk);
//       const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
//       // Then we get the challenge period for the BSP.
//       const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
//         userApi.shConsts.DUMMY_BSP_ID
//       );
//       assert(challengePeriodResult.isOk);
//       const challengePeriod = challengePeriodResult.asOk.toNumber();
//       // Then we calculate the next challenge tick.
//       const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;
//       // Finally, advance to the next challenge tick.
//       await userApi.block.skipTo(nextChallengeTick);

//       await userApi.assert.extrinsicPresent({
//         module: "proofsDealer",
//         method: "submitProof",
//         checkTxPool: true,
//         assertLength: 3,
//         timeout: 10000
//       });

//       // Seal one more block with the pending extrinsics.
//       await userApi.block.seal();

//       // Assert for the the event of the proof successfully submitted and verified.
//       const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
//       strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

//       // Get the new last tick for which the BSP submitted a proof.
//       // It should be the previous last tick plus one BSP period.
//       const lastTickResultAfterProof =
//         await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
//           userApi.shConsts.DUMMY_BSP_ID
//         );
//       assert(lastTickResultAfterProof.isOk);
//       const lastTickBspSubmittedProofAfterProof = lastTickResultAfterProof.asOk.toNumber();
//       strictEqual(
//         lastTickBspSubmittedProofAfterProof,
//         lastTickBspSubmittedProof + challengePeriod,
//         "The last tick for which the BSP submitted a proof should be the previous last tick plus one BSP period"
//       );

//       // Get the new deadline for the BSP.
//       // It should be the current last tick, plus one BSP period, plus the challenges tick tolerance.
//       const challengesTickTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
//       const newDeadline =
//         lastTickBspSubmittedProofAfterProof + challengePeriod + challengesTickTolerance;
//       const newDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
//         userApi.shConsts.DUMMY_BSP_ID
//       );
//       assert(newDeadlineResult.isOk);
//       const newDeadlineOnChain = newDeadlineResult.asOk.toNumber();
//       strictEqual(
//         newDeadline,
//         newDeadlineOnChain,
//         "The deadline should be the same as the one we just calculated"
//       );
//     });

//     it("BSP fails to submit proof and is marked as slashable", async () => {
//       // Get BSP-Down's deadline.
//       const bspDownDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
//         userApi.shConsts.BSP_DOWN_ID
//       );
//       assert(bspDownDeadlineResult.isOk);
//       const bspDownDeadline = bspDownDeadlineResult.asOk.toNumber();

//       // Get the last tick for which the BSP-Down submitted a proof before advancing to the deadline.
//       const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
//         userApi.shConsts.BSP_DOWN_ID
//       );
//       assert(lastTickResult.isOk);
//       const lastTickBspDownSubmittedProof = lastTickResult.asOk.toNumber();
//       // Finally, advance to the next challenge tick.
//       await userApi.block.skipTo(bspDownDeadline);

//       // Expect to see a `SlashableProvider` event in the last block.
//       const slashableProviderEvent = await userApi.assert.eventPresent(
//         "proofsDealer",
//         "SlashableProvider"
//       );
//       const slashableProviderEventDataBlob =
//         userApi.events.proofsDealer.SlashableProvider.is(slashableProviderEvent.event) &&
//         slashableProviderEvent.event.data;
//       assert(slashableProviderEventDataBlob, "Event doesn't match Type");
//       strictEqual(
//         slashableProviderEventDataBlob.provider.toString(),
//         userApi.shConsts.BSP_DOWN_ID,
//         "The BSP-Down should be slashable"
//       );

//       // Get the last tick for which the BSP-Down submitted a proof after advancing to the deadline.
//       const lastTickResultAfterSlashable =
//         await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
//           userApi.shConsts.BSP_DOWN_ID
//         );
//       assert(lastTickResultAfterSlashable.isOk);
//       const lastTickBspDownSubmittedProofAfterSlashable =
//         lastTickResultAfterSlashable.asOk.toNumber();

//       // The new last tick should be equal to the last tick before BSP-Down was marked as slashable plus one challenge period.
//       const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
//         userApi.shConsts.DUMMY_BSP_ID
//       );
//       assert(challengePeriodResult.isOk);
//       strictEqual(
//         lastTickBspDownSubmittedProofAfterSlashable,
//         lastTickBspDownSubmittedProof,
//         "The last tick for which the BSP-Down submitted a proof should remain the same since the BSP went down"
//       );
//     });

//     it("BSP three stops storing last file", async () => {
//       // Wait for BSP-Three to catch up to the tip of the chain
//       await userApi.wait.bspCatchUpToChainTip(bspThreeApi);

//       // Build transaction for BSP-Three to stop storing the only file it has.
//       const inclusionForestProof = await bspThreeApi.rpc.storagehubclient.generateForestProof(
//         null,
//         [fileMetadata.fileKey]
//       );
//       await userApi.wait.waitForAvailabilityToSendTx(bspThreeKey.address.toString());
//       const blockResult = await userApi.block.seal({
//         calls: [
//           bspThreeApi.tx.fileSystem.bspRequestStopStoring(
//             fileMetadata.fileKey,
//             fileMetadata.bucketId,
//             fileMetadata.location,
//             fileMetadata.owner,
//             fileMetadata.fingerprint,
//             fileMetadata.fileSize,
//             false,
//             inclusionForestProof.toString()
//           )
//         ],
//         signer: bspThreeKey
//       });
//       assert(blockResult.extSuccess, "Extrinsic was part of the block so its result should exist.");
//       assert(
//         blockResult.extSuccess === true,
//         "Extrinsic to request stop storing should have been successful"
//       );

//       userApi.assert.fetchEvent(
//         userApi.events.fileSystem.BspRequestedToStopStoring,
//         await userApi.query.system.events()
//       );
//     });

//     it("BSP can correctly delete a file from its forest and runtime correctly updates its root", async () => {
//       // Generate the inclusion proof for the file key that BSP-Three requested to stop storing.
//       const inclusionForestProof = await bspThreeApi.rpc.storagehubclient.generateForestProof(
//         null,
//         [fileMetadata.fileKey]
//       );

//       // Wait enough blocks for the deletion to be allowed.
//       const currentBlock = await userApi.rpc.chain.getBlock();
//       const currentBlockNumber = currentBlock.block.header.number.toNumber();
//       const minWaitForStopStoring = (
//         await userApi.query.parameters.parameters({
//           RuntimeConfig: {
//             MinWaitForStopStoring: null
//           }
//         })
//       )
//         .unwrap()
//         .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
//       const cooldown = currentBlockNumber + minWaitForStopStoring;
//       await userApi.block.skipTo(cooldown);
//       await userApi.wait.waitForAvailabilityToSendTx(bspThreeKey.address.toString());

//       // Confirm the request of deletion. Make sure the extrinsic doesn't fail and the root is updated correctly.
//       await userApi.block.seal({
//         calls: [
//           bspThreeApi.tx.fileSystem.bspConfirmStopStoring(
//             fileMetadata.fileKey,
//             inclusionForestProof.toString()
//           )
//         ],
//         signer: bspThreeKey
//       });
//       // Check for the confirm stopped storing event.
//       const confirmStopStoringEvent = await userApi.assert.eventPresent(
//         "fileSystem",
//         "BspConfirmStoppedStoring"
//       );
//       // Wait for confirmation line in docker logs.
//       await bspThreeApi.docker.waitForLog({
//         containerName: "sh-bsp-three",
//         searchString: "New local Forest root matches the one in the block for BSP"
//       });

//       // Make sure the new root was updated correctly.
//       const newRoot = (await bspThreeApi.rpc.storagehubclient.getForestRoot(null)).unwrap();
//       assert(userApi.events.fileSystem.BspConfirmStoppedStoring.is(confirmStopStoringEvent.event));
//       const newRootInRuntime = confirmStopStoringEvent.event.data.newRoot;

//       // Important! Keep the string conversion to avoid a recursive call that lead to a crash in javascript.
//       strictEqual(
//         newRoot.toString(),
//         newRootInRuntime.toString(),
//         "The new root should be updated correctly"
//       );
//     });

//     it("BSP three is not challenged any more", async () => {
//       const result = await userApi.call.proofsDealerApi.getNextDeadlineTick(ShConsts.BSP_THREE_ID);

//       assert(result.isErr, "BSP three doesn't have files so it shouldn't have deadline");
//     });

//     it("New storage request sent by user, to only one BSP", async () => {
//       // Pause BSP-Two and BSP-Three.
//       await userApi.docker.pauseContainer("sh-bsp-two");
//       await userApi.docker.pauseContainer("sh-bsp-three");
//     });

//     it("Only one BSP confirms it and the MSP accepts it", async () => {
//       // Wait for the MSP acceptance of the file to be in the TX pool
//       await userApi.assert.extrinsicPresent({
//         module: "fileSystem",
//         method: "mspRespondStorageRequestsMultipleBuckets",
//         checkTxPool: true,
//         timeout: 5000
//       });

//       // Then wait for the BSP volunteer to be in the TX pool and seal the block
//       await userApi.wait.bspVolunteer(1);

//       // Finally, wait for the BSP to confirm storing the file and seal the block
//       const address = userApi.createType("Address", NODE_INFOS.bsp.AddressId);
//       await userApi.wait.bspStored({ expectedExts: 1, bspAccount: address });
//     });

//     it("BSP correctly responds to challenge with new forest root", async () => {
//       // Advance to two challenge periods ahead for first BSP.
//       // This is because in the odd case that we're exactly on the next challenge tick right now,
//       // there is a race condition chance where the BSP will send the submit proof extrinsic in the
//       // next block, since the Forest write lock is released as a consequence of the confirm storing
//       // extrinsic. So we advance two challenge periods ahead to be sure.

//       // First we get the last tick for which the BSP submitted a proof.
//       const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
//         ShConsts.DUMMY_BSP_ID
//       );
//       assert(lastTickResult.isOk);
//       const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
//       // Then we get the challenge period for the BSP.
//       const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
//         ShConsts.DUMMY_BSP_ID
//       );
//       assert(challengePeriodResult.isOk);
//       const challengePeriod = challengePeriodResult.asOk.toNumber();
//       // Then we calculate two challenge ticks ahead.
//       const nextChallengeTick = lastTickBspSubmittedProof + 2 * challengePeriod;
//       // Finally, advance two challenge ticks ahead.
//       await userApi.block.skipTo(nextChallengeTick);

//       const submitProofsPending = await userApi.assert.extrinsicPresent({
//         module: "proofsDealer",
//         method: "submitProof",
//         checkTxPool: true
//       });

//       // Seal block and check that the transaction was successful.
//       await userApi.block.seal();

//       // Assert for the event of the proof successfully submitted and verified.
//       const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
//       strictEqual(
//         proofAcceptedEvents.length,
//         submitProofsPending.length,
//         "All pending submit proof transactions should have been successful"
//       );
//     });

//     it("Resume BSPs, and they shouldn't volunteer for the expired storage request", async () => {
//       // Advance a number of blocks up to when the storage request times out for sure.
//       const storageRequestTtl = (
//         await userApi.query.parameters.parameters({
//           RuntimeConfig: {
//             StorageRequestTtl: null
//           }
//         })
//       )
//         .unwrap()
//         .asRuntimeConfig.asStorageRequestTtl.toNumber();
//       const currentBlock = await userApi.rpc.chain.getBlock();
//       const currentBlockNumber = currentBlock.block.header.number.toNumber();
//       await userApi.block.skipTo(currentBlockNumber + storageRequestTtl, {
//         watchForBspProofs: [ShConsts.DUMMY_BSP_ID]
//       });

//       // Resume BSP-Two and BSP-Three.
//       await userApi.docker.resumeContainer({
//         containerName: "sh-bsp-two"
//       });
//       await userApi.docker.resumeContainer({
//         containerName: "sh-bsp-three"
//       });

//       // Wait for BSPs to resync.
//       await userApi.wait.bspCatchUpToChainTip(bspTwoApi);
//       await userApi.wait.bspCatchUpToChainTip(bspThreeApi);

//       // There shouldn't be any pending volunteer transactions.
//       await assert.rejects(
//         async () => {
//           await userApi.assert.extrinsicPresent({
//             module: "fileSystem",
//             method: "bspVolunteer",
//             checkTxPool: true,
//             timeout: 2000
//           });
//         },
//         /No matching extrinsic found for fileSystem\.bspVolunteer/,
//         "There should be no pending volunteer transactions"
//       );
//     });

//     it("BSP-Two still correctly responds to challenges with same forest root", async () => {
//       // Advance some blocks to allow the BSP to process the challenges and submit proofs.
//       await userApi.block.skip(20);

//       // Advance to next challenge tick for BSP-Two.
//       // First we get the last tick for which the BSP submitted a proof.
//       const lastTickResult =
//         await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(BSP_TWO_ID);
//       assert(lastTickResult.isOk);
//       const lastTickBspTwoSubmittedProof = lastTickResult.asOk.toNumber();
//       // Then we get the challenge period for the BSP.
//       const challengePeriodResult =
//         await userApi.call.proofsDealerApi.getChallengePeriod(BSP_TWO_ID);
//       assert(challengePeriodResult.isOk);
//       const challengePeriod = challengePeriodResult.asOk.toNumber();
//       // Then we calculate the next challenge tick.
//       const nextChallengeTick = lastTickBspTwoSubmittedProof + challengePeriod;

//       const currentBlock = await userApi.rpc.chain.getBlock();
//       const currentBlockNumber = currentBlock.block.header.number.toNumber();

//       if (nextChallengeTick > currentBlockNumber) {
//         // Advance to the next challenge tick if needed
//         await userApi.block.skipTo(nextChallengeTick, {
//           watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID]
//         });
//       }

//       // There should be two pending submit proof transactions, one per active BSP.
//       const submitProofsPending = await userApi.assert.extrinsicPresent({
//         module: "proofsDealer",
//         method: "submitProof",
//         checkTxPool: true,
//         assertLength: 2,
//         exactLength: true
//       });

//       // Seal block and check that the transaction was successful.
//       await userApi.block.seal();

//       // Assert for the event of the proof successfully submitted and verified.
//       const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");

//       strictEqual(
//         proofAcceptedEvents.length,
//         submitProofsPending.length,
//         "All pending submit proof transactions should have been successful"
//       );
//     });

//     it(
//       "Custom challenge is added",
//       { skip: "Not implemented yet. All BSPs have the same files." },
//       async () => {
//         await it("Custom challenge is included in checkpoint challenge round", async () => {
//           // TODO: Send transaction for custom challenge with new file key.
//           // TODO: Advance until next checkpoint challenge block.
//           // TODO: Check that custom challenge was included in checkpoint challenge round.
//         });

//         await it("BSP that has it responds to custom challenge with proof of inclusion", async () => {
//           // TODO: Advance until next challenge for BSP.
//           // TODO: Build block with proof submission.
//           // TODO: Check that proof submission was successful, including the custom challenge.
//         });

//         await it("BSPs who don't have it respond non-inclusion proof", async () => {
//           // TODO: Advance until next challenge for BSP-Two and BSP-Three.
//           // TODO: Build block with proof submission.
//           // TODO: Check that proof submission was successful, with proof of non-inclusion.
//         });
//       }
//     );

//     it(
//       "File mutation is finalised and BSP removes it from File Storage",
//       { skip: "Not implemented yet." },
//       async () => {
//         // TODO: Finalise block with mutations.
//         // TODO: Check that file is removed from File Storage. Need a RPC method for this.
//       }
//     );
//   }
// );
