import "@storagehub/types-bundle";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import { types as BundledTypes } from "@storagehub/types-bundle";
import { assertEventPresent } from "../asserts";
import { createBucket, sendNewStorageRequest } from "./fileHelpers";
import type { BspNetApi } from "./types";
import { advanceToBlock, sealBlock } from "./block";

/**
 * DEPRECATED: Use BspNetTestApi.create() instead
 *
 */
export const createApiObject = async (
  uri: `ws://${string}` | `wss://${string}`
): Promise<BspNetApi> => {
  const baseApi = await ApiPromise.create({
    provider: new WsProvider(uri),
    noInitWarn: true,
    throwOnConnect: false,
    throwOnUnknown: false,
    typesBundle: BundledTypes
  });

  return Object.assign(baseApi, {
    sealBlock: async (
      calls?:
        | SubmittableExtrinsic<"promise", ISubmittableResult>
        | SubmittableExtrinsic<"promise", ISubmittableResult>[],
      signer?: KeyringPair
    ) => sealBlock(baseApi, calls, signer),

    advanceToBlock: async (
      blockNumber: number,
      options?: {
        waitBetweenBlocks?: number | boolean;
        waitForBspProofs?: string[];
      }
    ) =>
      advanceToBlock(baseApi, blockNumber, options?.waitBetweenBlocks, options?.waitForBspProofs),

    sendNewStorageRequest: async (source: string, location: string, bucketName: string) =>
      sendNewStorageRequest(baseApi, source, location, bucketName),

    createBucket: async (bucketName: string) => createBucket(baseApi, bucketName),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
