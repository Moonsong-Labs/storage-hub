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
import { alice } from "../pjsKeyring";
import { isExtSuccess } from "../extrinsics";
import { sleep } from "../timer";
import type { EnrichedBspApi } from "./test-api";
import { ShConsts } from "./consts";
import assert, { strictEqual } from "node:assert";
import { Assertions } from "../asserts";

export interface SealedBlock {
  blockReceipt: CreatedBlock;
  txHash?: string;
  blockData?: SignedBlock;
  events?: EventRecord[];
  extSuccess?: boolean;
}

export const sealBlock = async (
  api: ApiPromise,
  calls?:
    | SubmittableExtrinsic<"promise", ISubmittableResult>
    | SubmittableExtrinsic<"promise", ISubmittableResult>[],
  signer?: KeyringPair
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
    blockReceipt: await api.rpc.engine.createBlock(true, true),
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

export const skipBlocks = async (api: ApiPromise, blocksToSkip: number) => {
  console.log(`\tSkipping ${blocksToSkip} blocks...`);
  for (let i = 0; i < blocksToSkip; i++) {
    await sealBlock(api);
    await sleep(50);
  }
};

export const skipBlocksToMinChangeTime: (
  api: EnrichedBspApi,
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
  // Assert that challengeTickToChallengedProviders contains an entry for the challenged provider
  const challengeTickToChallengedProviders =
    await api.query.proofsDealer.challengeTickToChallengedProviders(nextChallengeTick, provider);
  strictEqual(challengeTickToChallengedProviders.isSome, true);

  const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  const blocksToSkip = nextChallengeTick - currentHeight - 1;
  console.log("Current height: ", currentHeight);
  console.log("Next challenge tick: ", nextChallengeTick);
  console.log("Blocks to skip: ", blocksToSkip);
  console.log(`\tSkipping ${blocksToSkip} blocks to reach next challenge period...`);
  if (blocksToSkip > 0) {
    await skipBlocks(api, nextChallengeTick);
  } else {
    throw new Error("Already past next challenge period");
  }

  const oldFailedSubmissionsCount = await api.query.proofsDealer.slashableProviders(provider);
  console.log(`Block is now : ${(await api.rpc.chain.getHeader()).number.toNumber()}`);
  // Assert that the SlashableProvider event is emitted.
  const blockResult = await sealBlock(api);

  const [_provider, nextChallengeDeadline] = Assertions.fetchEvent(
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


export const advanceToBlock = async (
  api: ApiPromise,
  blockNumber: number,
  waitBetweenBlocks?: number | boolean,
  watchForBspProofs?: string[]
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
  if (blockNumber > currentBlockNumber) {
    const blocksToAdvance = blockNumber - currentBlockNumber;
    for (let i = 0; i < blocksToAdvance; i++) {
      blockResult = await sealBlock(api);
      currentBlockNumber += 1;

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
  } else {
    throw new Error(
      `Block number ${blockNumber} is lower than current block number ${currentBlockNumber}`
    );
  }

  if (blockResult) {
    return blockResult;
  }

  throw new Error("Block wasn't sealed");
};

export namespace BspNetBlock {
  export const seal = sealBlock;
  export const skip = skipBlocks;
  export const skipTo = advanceToBlock
  export const skipToMinChangeTime = skipBlocksToMinChangeTime;
  export const skipToChallengePeriod = runToNextChallengePeriodBlock;
}
