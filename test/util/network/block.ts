import type {
  CreatedBlock,
  EventRecord,
  H256,
  Hash,
  SignedBlock
} from "@polkadot/types/interfaces";
import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { alice, bob } from "../pjsKeyring";
import { isExtSuccess } from "../extrinsics";
import { sleep } from "../timer";
import * as ShConsts from "./consts";
import assert, { strictEqual } from "node:assert";
import * as Assertions from "../asserts";
import invariant from "tiny-invariant";

export interface SealedBlock {
  blockReceipt: CreatedBlock;
  txHash?: string;
  blockData?: SignedBlock;
  events?: EventRecord[];
  extSuccess?: boolean;
}

/**
 * Seals a block with optional extrinsics and finalizes it.
 *
 * This function creates a new block, optionally including specified extrinsics.
 * It handles the process of sending transactions, creating the block, and collecting events.
 *
 * @param api - The ApiPromise instance.
 * @param calls - Optional extrinsic(s) to include in the block.
 * @param signer - Optional signer for the extrinsics.
 * @param finaliseBlock - Whether to finalize the block. Defaults to true.
 * @returns A Promise resolving to a SealedBlock object containing block details and events.
 *
 * @throws Will throw an error if the block creation fails or if extrinsics are unsuccessful.
 */
export const sealBlock = async (
  api: ApiPromise,
  calls?:
    | SubmittableExtrinsic<"promise", ISubmittableResult>
    | SubmittableExtrinsic<"promise", ISubmittableResult>[],
  signer?: KeyringPair,
  finaliseBlock = true
): Promise<SealedBlock> => {
  const initialHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  const results: {
    hashes: Hash[];
    events: EventRecord[];
    blockData?: SignedBlock;
    success: boolean[];
  } = {
    hashes: [],
    events: [],
    success: []
  };

  // Normalize to array
  const callArray = Array.isArray(calls) ? calls : calls ? [calls] : [];

  if (callArray.length > 0) {
    const nonce = await api.rpc.system.accountNextIndex((signer || alice).address);

    // Send all transactions in sequence
    for (let i = 0; i < callArray.length; i++) {
      const call = callArray[i];
      let hash: Hash;

      if (call.isSigned) {
        hash = await call.send();
      } else {
        hash = await call.signAndSend(signer || alice, { nonce: nonce.addn(i) });
      }

      results.hashes.push(hash);
    }
  }

  const sealedResults = {
    blockReceipt: await api.rpc.engine.createBlock(true, finaliseBlock),
    txHashes: results.hashes.map((hash) => hash.toString())
  };

  const blockHash = sealedResults.blockReceipt.blockHash;
  const allEvents = await (await api.at(blockHash)).query.system.events();

  if (results.hashes.length > 0) {
    const blockData = await api.rpc.chain.getBlock(blockHash);
    results.blockData = blockData;

    const getExtIndex = (txHash: Hash) => {
      return blockData.block.extrinsics.findIndex((ext) => ext.hash.toHex() === txHash.toString());
    };

    for (const hash of results.hashes) {
      const extIndex = getExtIndex(hash);
      const extEvents = allEvents.filter(
        ({ phase }) =>
          phase.isApplyExtrinsic && Number(phase.asApplyExtrinsic.toString()) === extIndex
      );
      results.events.push(...extEvents);
      results.success.push(isExtSuccess(extEvents) ?? false);
    }
  } else {
    results.events.push(...allEvents);
  }

  const extSuccess = results.success.every((success) => success);

  // Allow time for chain to settle
  for (let i = 0; i < 20; i++) {
    const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
    if (currentHeight > initialHeight) {
      break;
    }
    await sleep(50);
  }

  return Object.assign(sealedResults, {
    events: results.events,
    extSuccess: extSuccess
  }) satisfies SealedBlock;
};

/**
 * Skips a specified number of blocks in the blockchain.
 *
 * This function seals empty blocks to advance the blockchain by a given number of blocks.
 * It's useful for simulating the passage of time in the network.
 *
 * @param api - The ApiPromise instance.
 * @param blocksToSkip - The number of blocks to skip.
 * @returns A Promise that resolves when all blocks have been skipped.
 */
export const skipBlocks = async (api: ApiPromise, blocksToSkip: number) => {
  console.log(`\tSkipping ${blocksToSkip} blocks...`);
  for (let i = 0; i < blocksToSkip; i++) {
    await sealBlock(api);
    await sleep(50);
  }
};

export const skipBlocksToMinChangeTime: (
  api: ApiPromise,
  bspId?: `0x${string}` | H256 | Uint8Array
) => Promise<void> = async (api, bspId = ShConsts.DUMMY_BSP_ID) => {
  const lastCapacityChangeHeight = (await api.query.providers.backupStorageProviders(bspId))
    .unwrap()
    .lastCapacityChange.toNumber();
  const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
  const minChangeTime = api.consts.providers.minBlocksBetweenCapacityChanges.toNumber();
  const blocksToSkip = minChangeTime - (currentHeight - lastCapacityChangeHeight);

  if (blocksToSkip > 0) {
    console.log(
      `\tSkipping blocks to reach MinBlocksBetweenCapacityChanges height: #${minChangeTime}`
    );
    await skipBlocks(api, blocksToSkip);
  } else {
    console.log("\tNo need to skip blocks, already past MinBlocksBetweenCapacityChanges");
  }
};

export async function runToNextChallengePeriodBlock(
  api: ApiPromise,
  nextChallengeTick: number,
  provider: string
): Promise<number> {
  const tickToProvidersDeadlines = await api.query.proofsDealer.tickToProvidersDeadlines(
    nextChallengeTick,
    provider
  );
  strictEqual(tickToProvidersDeadlines.isSome, true);

  const blockNumber = await api.query.system.number();
  for (let i = blockNumber.toNumber(); i < nextChallengeTick - 1; i++) {
    await sealBlock(api);
  }

  const oldFailedSubmissionsCount = await api.query.proofsDealer.slashableProviders(provider);

  // Assert that the SlashableProvider event is emitted.
  const blockResult = await sealBlock(api);

  const [_provider, nextChallengeDeadline] = Assertions.fetchEventData(
    api.events.proofsDealer.SlashableProvider,
    blockResult.events
  );

  // Check provider is slashable for 1 additional failed submission.
  const slashableProviders = await api.query.proofsDealer.slashableProviders(provider);
  strictEqual(
    slashableProviders.unwrap().toNumber(),
    oldFailedSubmissionsCount.unwrapOrDefault().toNumber() +
      api.consts.proofsDealer.randomChallengesPerBlock.toNumber()
  );

  return nextChallengeDeadline.toNumber();
}

/**
 * Advances the blockchain to a specified block number.
 *
 * This function is crucial for testing scenarios that depend on specific blockchain states.
 * It allows for precise control over the blockchain's progression, including the ability
 * to simulate BSP proof submissions and challenge periods.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param blockNumber - The target block number to advance to.
 * @param waitBetweenBlocks - Optional. If specified:
 *                            - If a number, waits for that many milliseconds between blocks.
 *                            - If true, waits for 500ms between blocks.
 *                            - If false or undefined, doesn't wait between blocks.
 * @param watchForBspProofs - Optional. An array of BSP IDs to watch for proofs.
 *                            If specified, the function will wait for BSP proofs at appropriate intervals.
 * @param spam - Optional. If specified, the function will spam the chain with blocks.
 *                            - If true, the function will spam all blocks.
 *                            - If false or undefined, the function will not spam the chain.
 *                            - If a number, the function will spam the chain for that many blocks, and then continue with non-spammed blocks.
 *
 * @returns A Promise that resolves to a SealedBlock object representing the last sealed block.
 *
 * @throws Will throw an error if the target block number is lower than the current block number.
 *
 * @example
 * // Advance to block 100 with no waiting
 * const result = await advanceToBlock(api, 100);
 *
 * @example
 * // Advance to block 200, waiting 1000ms between blocks
 * const result = await advanceToBlock(api, 200, 1000);
 *
 * @example
 * // Advance to block 300, watching for proofs from two BSPs
 * const result = await advanceToBlock(api, 300, true, ['bsp1', 'bsp2']);
 */
export const advanceToBlock = async (
  api: ApiPromise,
  blockNumber: number,
  waitBetweenBlocks?: number | boolean,
  watchForBspProofs?: string[],
  spam?: number | boolean,
  verbose?: boolean
): Promise<SealedBlock> => {
  // If watching for BSP proofs, we need to know the blocks at which they are challenged.
  const challengeBlockNumbers: { nextChallengeBlock: number; challengePeriod: number }[] = [];
  if (watchForBspProofs) {
    for (const bspId of watchForBspProofs) {
      // First we get the last tick for which the BSP submitted a proof.
      const lastTickResult =
        await api.call.proofsDealerApi.getLastTickProviderSubmittedProof(bspId);
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await api.call.proofsDealerApi.getChallengePeriod(bspId);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      challengeBlockNumbers.push({
        nextChallengeBlock: nextChallengeTick,
        challengePeriod
      });
    }
  }

  const currentBlock = await api.rpc.chain.getBlock();
  let currentBlockNumber = currentBlock.block.header.number.toNumber();

  let blockResult = null;

  invariant(
    blockNumber > currentBlockNumber,
    `Block number ${blockNumber} is lower than current block number ${currentBlockNumber}`
  );
  const blocksToAdvance = blockNumber - currentBlockNumber;

  let blocksToSpam = 0;
  if (spam) {
    if (typeof spam === "number") {
      blocksToSpam = spam;
    } else {
      blocksToSpam = blocksToAdvance;
    }
  }

  // Get the maximum block weight for normal class.
  // This is used to check if the block weight is reaching that maximum.
  const maxNormalBlockWeight = api.consts.system.blockWeights.perClass.normal.maxTotal.unwrap();

  for (let i = 0; i < blocksToAdvance; i++) {
    if (spam && i < blocksToSpam) {
      if (verbose) {
        console.log(`Spamming block ${i + 1} of ${blocksToSpam}`);
      }
      // The nonce of the spamming transactions should be incremented by 1 for each transaction.
      let nonce = (await api.rpc.system.accountNextIndex(alice.address)).toNumber();

      // We don't consider the proof size of the weight because we're spamming with transfers from
      // and to the same account. So the proof size is the same, regardless of the number of transfers.
      let accumulatedTransferWeightRefTime = 0;
      while (
        accumulatedTransferWeightRefTime + ShConsts.TRANSFER_WEIGHT_REF_TIME <=
        maxNormalBlockWeight.refTime.toNumber()
      ) {
        // We use transfers instead of remarks because with remarks the tx pool gets filled up before
        // we reach the maximum block weight.
        await api.tx.balances.transferAllowDeath(bob.address, 1).signAndSend(alice, { nonce });

        accumulatedTransferWeightRefTime += ShConsts.TRANSFER_WEIGHT_REF_TIME;
        nonce += 1;
      }
    }

    blockResult = await sealBlock(api);
    currentBlockNumber += 1;

    const blockWeight = await api.query.system.blockWeight();
    const blockWeightNormal = blockWeight.normal;

    if (spam && i < blocksToSpam && verbose) {
      console.log(`Normal block weight for block ${i + 1}: ${blockWeightNormal}`);

      const currentTick = (await api.call.proofsDealerApi.getCurrentTick()).toNumber();
      console.log(`Current tick: ${currentTick}`);
    }

    // Check if we need to wait for BSP proofs.
    if (watchForBspProofs) {
      for (const challengeBlockNumber of challengeBlockNumbers) {
        if (currentBlockNumber === challengeBlockNumber.nextChallengeBlock) {
          // Wait for the BSP to process the proof.
          await sleep(500);

          // Update next challenge block.
          challengeBlockNumbers[0].nextChallengeBlock += challengeBlockNumber.challengePeriod;
          break;
        }
      }
    }

    if (waitBetweenBlocks) {
      if (typeof waitBetweenBlocks === "number") {
        await sleep(waitBetweenBlocks);
      } else {
        await sleep(500);
      }
    }
  }

  invariant(blockResult, "Block wasn't sealed");

  return blockResult;
};

/**
 * Performs a chain reorganization by creating a finalized block on top of the parent block.
 *
 * This function is used to simulate network forks and test the system's ability to handle
 * chain reorganizations. It's a critical tool for ensuring the robustness of the BSP network
 * in face of potential consensus issues.
 *
 * @param api - The ApiPromise instance.
 * @throws Will throw an error if the head block is already finalized.
 * @returns A Promise that resolves when the chain reorganization is complete.
 */
export async function reOrgBlocks(api: ApiPromise): Promise<void> {
  const currentBlockHeader = await api.rpc.chain.getHeader();
  const finalisedHash = await api.rpc.chain.getFinalizedHead();

  if (currentBlockHeader.hash.eq(finalisedHash)) {
    console.error(`Head block #${currentBlockHeader.number.toString()} is already finalised`);
    console.error(
      "Tip ℹ️: You can create unfinalised blocks in sealBlock() by passing finaliseBlock = false"
    );
    throw "Cannot reorg a finalised block";
  }
  await api.rpc.engine.createBlock(true, true, finalisedHash);
}

/**
 * Options for creating a block in the chain.
 */
export type SealBlockOptions = {
  /**
   * Optional extrinsic(s) to include in the sealed block.
   * Can be a single extrinsic or an array of extrinsics.
   */
  calls?:
    | SubmittableExtrinsic<"promise", ISubmittableResult>
    | SubmittableExtrinsic<"promise", ISubmittableResult>[];

  /**
   * Optional signer for the extrinsics.
   * If not provided, a default signer (usually 'alice') will be used.
   */
  signer?: KeyringPair;

  /**
   * Whether to finalize the block after sealing.
   * Defaults to true if not specified.
   */
  finaliseBlock?: boolean;
};