import assert, { strictEqual } from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type {
  CreatedBlock,
  EventRecord,
  H256,
  Hash,
  SignedBlock
} from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import * as Assertions from "../asserts";
import { isExtSuccess } from "../extrinsics";
import { alice, bob } from "../pjsKeyring";
import { sleep } from "../timer";
import * as ShConsts from "./consts";
import { waitForLog } from "./docker";
import { waitForTxInPool } from "./waits";

export interface SealedBlock {
  blockReceipt: CreatedBlock;
  txHash?: string;
  blockData?: SignedBlock;
  events?: EventRecord[];
  extSuccess?: boolean;
}

/**
 * Extends a fork in the blockchain by creating new blocks on top of a specified parent block.
 *
 * This function is used for testing chain fork scenarios. It creates
 * a specified number of new blocks, each building on top of the previous one, starting
 * from a given parent block hash.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param options - Configuration options for extending the fork:
 *   @param options.parentBlockHash - The hash of the parent block to build upon.
 *   @param options.amountToExtend - The number of blocks to add to the fork.
 *   @param options.verbose - (optional) If true, logs detailed information about the fork extension process.
 *
 * @throws Will throw an assertion error if amountToExtend is not greater than 0.
 * @returns A Promise that resolves when all blocks have been created.
 */
export const extendFork = async (
  api: ApiPromise,
  options: {
    parentBlockHash: string;
    amountToExtend: number;
    verbose?: boolean;
  }
) => {
  let parentBlockHash: string = options.parentBlockHash;
  let parentHeight = (await api.rpc.chain.getHeader(parentBlockHash)).number.toNumber();
  assert(options.amountToExtend > 0, "extendFork: amountToExtend must be greater than 0!");

  for (let i = 0; i < options.amountToExtend; i++) {
    if (options.verbose) {
      console.log(`Extending fork by 1 block. Current height: ${parentHeight}`);
      console.log(`Parent block hash: ${parentBlockHash}`);
    }
    const { blockHash } = await api.rpc.engine.createBlock(true, false, parentBlockHash);
    if (options.verbose) {
      console.log(`New block hash: ${blockHash.toHex()}`);
    }
    parentBlockHash = blockHash.toHex();
    const newBlockNumber = (await api.rpc.chain.getHeader(blockHash)).number.toNumber();
    if (options.verbose) {
      console.log(`New block number: ${newBlockNumber}`);
    }
    assert(
      newBlockNumber > parentHeight,
      "Fork is not extended! this is a bug in logic, please raise"
    );
    parentHeight = newBlockNumber;

    // TODO replace with something smarter eventually
    await waitForLog({
      containerName: "storage-hub-sh-user-1", // we can only produce blocks via the user node for now
      searchString: "💤 Idle",
      timeout: 5000
    });
  }
};

/**
 * Seals a block with optional extrinsics and finalizes it.
 *
 * This function creates a new block, optionally including specified extrinsics.
 * It handles the process of sending transactions, creating the block, and collecting events.
 *
 * @param api - The ApiPromise instance.
 * @param calls - Optional extrinsic(s) to include in the block.
 * @param signer - Optional signer for the extrinsics.
 * @param nonce - Optional starting nonce for the extrinsics.
 * @param parentHash - Optional parent hash to build the block on top of.
 * @param finaliseBlock - Whether to finalize the block. Defaults to true.
 * @param failOnExtrinsicNonInclusion - Whether to fail if an extrinsic is not included in the block. Defaults to true.
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
  nonce?: number,
  parentHash?: string,
  finaliseBlock = true,
  failOnExtrinsicNonInclusion = true
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
    const nonceToUse =
      nonce ?? (await api.rpc.system.accountNextIndex((signer || alice).address)).toNumber();

    // Send all transactions in sequence
    for (let i = 0; i < callArray.length; i++) {
      const call = callArray[i];
      let hash: Hash;

      if (call.isSigned) {
        hash = await call.send();
      } else {
        hash = await call.signAndSend(signer || alice, {
          nonce: nonceToUse + i
        });
      }

      // Poll for the transaction to be included in the pending extrinsics, or error out in 2 seconds
      const iterations = 20;
      const delay = 100;
      for (let i = 0; i < iterations; i++) {
        // Get the pending extrinsics
        const pendingExtrinsics = await api.rpc.author.pendingExtrinsics();

        // Check if the hash of the transaction is in the pending extrinsics
        if (pendingExtrinsics.map((ext) => ext.hash.toString()).includes(hash.toString())) {
          break;
        }

        if (i < iterations) {
          await sleep(delay);
        } else {
          if (failOnExtrinsicNonInclusion) {
            console.error(
              `Failed to find transaction ${hash.toString()} (${call.method.section.toString()}:${call.method.method.toString()}) in pending extrinsics`
            );
            throw new Error(
              `Transaction ${call.method.section.toString()}:${call.method.method.toString()} failed to be included in the block`
            );
          }
        }
      }

      results.hashes.push(hash);
    }
  }

  let blockReceipt: CreatedBlock;
  if (parentHash) {
    blockReceipt = await api.rpc.engine.createBlock(true, finaliseBlock, parentHash);
  } else {
    blockReceipt = await api.rpc.engine.createBlock(true, finaliseBlock);
  }

  const sealedResults = {
    blockReceipt,
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

    // Print any errors in the extrinsics to console for easier debugging
    for (const { event, phase } of allEvents.filter(
      ({ event }) => api.events.system.ExtrinsicFailed.is(event) && event.data
    )) {
      const errorEventDataBlob = api.events.system.ExtrinsicFailed.is(event) && event.data;
      assert(errorEventDataBlob, "Must have errorEventDataBlob since array is filtered for it");

      console.log(`Transaction failed in block ${blockHash.toString()}`);

      // Get the index of the extrinsic that failed in the block
      const extIndex = phase.isApplyExtrinsic ? phase.asApplyExtrinsic.toNumber() : -1;

      if (extIndex >= 0) {
        // Retrieve the extrinsic causing the error
        const failedExtrinsic = results.blockData?.block.extrinsics[extIndex];

        if (failedExtrinsic) {
          const { method, section } = failedExtrinsic.method;
          const args = failedExtrinsic.method.args.map((arg) => arg.toHuman());

          console.log(`Failed Extrinsic: ${section}.${method} with args ${JSON.stringify(args)}`);
        }
      }

      if (errorEventDataBlob.dispatchError.isModule) {
        const decoded = api.registry.findMetaError(errorEventDataBlob.dispatchError.asModule);
        const { docs, method, section } = decoded;
        console.log(`Error: ${section}.${method}: ${docs.join(" ")}`);
      } else {
        console.log(
          `Unable to link error to module, printing raw error message: ${errorEventDataBlob.dispatchError.toString()}`
        );
      }
    }

    for (const hash of results.hashes) {
      const extIndex = getExtIndex(hash);
      if (extIndex >= 0) {
        const extEvents = allEvents.filter(
          ({ phase }) =>
            phase.isApplyExtrinsic && Number(phase.asApplyExtrinsic.toString()) === extIndex
        );
        results.events.push(...extEvents);
        results.success.push(isExtSuccess(extEvents) ?? false);
      } else {
        console.log(
          `Extrinsic with hash ${hash.toString()} not found in block even though it was sent`
        );
      }
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
 * @param paddingMs - Optional. The time to wait between blocks in milliseconds. Defaults to 50ms.
 * @returns A Promise that resolves when all blocks have been skipped.
 */
export const skipBlocks = async (api: ApiPromise, blocksToSkip: number, paddingMs = 50) => {
  for (let i = 0; i < blocksToSkip; i++) {
    await sealBlock(api);
    await sleep(paddingMs);
  }
};

export const skipBlocksUntilBspCanChangeCapacity: (
  api: ApiPromise,
  bspId?: `0x${string}` | H256 | Uint8Array
) => Promise<void> = async (api, bspId = ShConsts.DUMMY_BSP_ID, verbose = false) => {
  const queryEarliestChangeCapacityBlockResult =
    await api.call.storageProvidersApi.queryEarliestChangeCapacityBlock(bspId);
  assert(
    queryEarliestChangeCapacityBlockResult.isOk,
    "Failed to query earliest change capacity block"
  );
  const blockToAdvanceTo = queryEarliestChangeCapacityBlockResult.asOk.toNumber();
  const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  if (blockToAdvanceTo > currentHeight) {
    verbose &&
      console.log(
        `\tSkipping to block #${blockToAdvanceTo} to go beyond MinBlocksBetweenCapacityChanges`
      );
    await advanceToBlock(api, {
      blockNumber: blockToAdvanceTo - 1,
      verbose: false,
      watchForBspProofs: [bspId.toString()]
    });
  } else {
    verbose &&
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

  const {
    data: { nextChallengeDeadline }
  } = Assertions.fetchEvent(api.events.proofsDealer.SlashableProvider, blockResult.events);

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
  options: {
    blockNumber: number;
    waitBetweenBlocks?: number | boolean;
    watchForBspProofs?: string[];
    finalised?: boolean;
    spam?: number | boolean;
    verbose?: boolean;
  }
): Promise<SealedBlock> => {
  const currentBlock = await api.rpc.chain.getBlock();
  let currentBlockNumber = currentBlock.block.header.number.toNumber();

  let blockResult = null;

  assert(
    options.blockNumber > currentBlockNumber,
    `Block number ${options.blockNumber} is lower than current block number ${currentBlockNumber}`
  );
  const blocksToAdvance = options.blockNumber - currentBlockNumber;

  let blocksToSpam = 0;
  if (options.spam) {
    if (typeof options.spam === "number") {
      blocksToSpam = options.spam;
    } else {
      blocksToSpam = blocksToAdvance;
    }
  }

  // Get the maximum block weight for normal class.
  // This is used to check if the block weight is reaching that maximum.
  const maxNormalBlockWeight = api.consts.system.blockWeights.perClass.normal.maxTotal.unwrap();

  for (let i = 0; i < blocksToAdvance; i++) {
    // Only for spamming!
    if (options.spam && i < blocksToSpam) {
      if (options.verbose) {
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

    blockResult = await sealBlock(api, [], undefined, undefined, undefined, options.finalised);
    currentBlockNumber += 1;

    const blockWeight = await api.query.system.blockWeight();
    const blockWeightNormal = blockWeight.normal;

    if (options.spam && i < blocksToSpam && options.verbose) {
      console.log(`Normal block weight for block ${i + 1}: ${blockWeightNormal}`);

      const currentTick = (await api.call.proofsDealerApi.getCurrentTick()).toNumber();
      console.log(`Current tick: ${currentTick}`);
    }

    // If watching for BSP proofs, we need to know if this block is a challenge block for any of the BSPs.
    if (options.watchForBspProofs) {
      let txsToWaitFor = 0;
      for (const bspId of options.watchForBspProofs) {
        // Get the next challenge tick.
        const nextChallengeTickResult =
          await api.call.proofsDealerApi.getNextTickToSubmitProofFor(bspId);

        if (nextChallengeTickResult.isErr) {
          options.verbose && console.log(`Failed to get next challenge tick for BSP ${bspId}`);
          continue;
        }

        const nextChallengeTick = nextChallengeTickResult.asOk.toNumber();
        if (currentBlockNumber === nextChallengeTick) {
          txsToWaitFor++;
        }
      }

      // Wait for all corresponding BSPs to have submitted their proofs.
      await waitForTxInPool(api, {
        module: "proofsDealer",
        method: "submitProof",
        checkQuantity: txsToWaitFor,
        strictQuantity: false
      });
    }

    if (options.waitBetweenBlocks) {
      if (typeof options.waitBetweenBlocks === "number") {
        await sleep(options.waitBetweenBlocks);
      } else {
        await sleep(500);
      }
    }
  }

  assert(blockResult, "Block wasn't sealed");

  return blockResult;
};

/**
 * Finalises a block (and therefore all of its predecessors) in the blockchain.
 *
 * @param api - The ApiPromise instance.
 * @param hashToFinalise - The hash of the block to finalise.
 * @returns A Promise that resolves when the chain reorganization is complete.
 */
export async function finaliseBlock(api: ApiPromise, hashToFinalise: string): Promise<void> {
  await api.rpc.engine.finalizeBlock(hashToFinalise);
}

/**
 * Performs a chain reorganisation by creating a finalised block on top of the parent block.
 *
 * This function is used to simulate network forks and test the system's ability to handle
 * chain reorganizations. It's a critical tool for ensuring the robustness of the BSP network
 * in face of potential consensus issues.
 *
 * @param api - The ApiPromise instance.
 * @throws Will throw an error if the head block is already finalised.
 * @returns A Promise that resolves when the chain reorganization is complete.
 */
export async function reOrgWithFinality(api: ApiPromise): Promise<void> {
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
 * Performs a chain reorganisation by creating a longer forked chain.
 * If no parent starting block is provided, the chain will start the fork from the last
 * finalised block.
 *
 * !!! WARNING !!!
 * The number of blocks this function can create for the alternative fork is limited by the
 * "unincluded segment capacity" parameter, set in the `ConsensusHook` config type of the
 * `cumulus-pallet-parachain-system`. If you try to build more blocks than this limit to
 * achieve the reorg, the node will panic when building the block.
 *
 * This function is used to simulate network forks and test the system's ability to handle
 * chain reorganizations. It's a critical tool for ensuring the robustness of the BSP network
 * in face of potential consensus issues.
 *
 * @param api - The ApiPromise instance.
 * @param startingBlock - Optional. The hash of the starting block to create the fork from.
 * @throws Will throw an error if the last finalised block is greater than the starting block
 *         or if the starting block is the same or higher than the current block.
 * @returns A Promise that resolves when the chain reorganization is complete.
 */
export async function reOrgWithLongerChain(
  api: ApiPromise,
  startingBlockHash?: string
): Promise<void> {
  const blockHash = startingBlockHash ?? (await api.rpc.chain.getFinalizedHead());
  const startingBlock = await api.rpc.chain.getHeader(blockHash);
  const startingBlockNumber = startingBlock.number.toNumber();

  const finalisedHash = await api.rpc.chain.getFinalizedHead();
  const finalisedBlock = await api.rpc.chain.getHeader(finalisedHash);
  const finalisedBlockNumber = finalisedBlock.number.toNumber();

  const currentBlock = await api.rpc.chain.getHeader();
  const currentBlockNumber = currentBlock.number.toNumber();

  if (finalisedBlockNumber > startingBlockNumber) {
    throw new Error(
      `Last finalised block #${finalisedBlockNumber} is greater than starting block #${startingBlockNumber}. So a fork cannot start from it.`
    );
  }

  if (startingBlockNumber === currentBlockNumber) {
    throw new Error(
      `Starting block #${startingBlockNumber} is the same as the current block #${currentBlockNumber}. So a fork cannot start from it.`
    );
  }

  if (startingBlockNumber > currentBlockNumber) {
    throw new Error(
      `Starting block #${startingBlockNumber} is higher than the current block #${currentBlockNumber}. So a fork cannot start from it.`
    );
  }

  const parentHash = blockHash;
  await extendFork(api, {
    parentBlockHash: parentHash.toString(),
    amountToExtend: currentBlockNumber - startingBlockNumber + 1,
    verbose: false
  });
}
