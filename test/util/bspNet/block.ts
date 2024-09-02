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

export namespace BspNetBlock {
  export const seal = sealBlock;
  export const skip = skipBlocks;
  export const skipToMinChangeTime = skipBlocksToMinChangeTime;
}
