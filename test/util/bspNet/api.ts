import "@storagehub/types-bundle";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord, H256 } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import { types as BundledTypes } from "@storagehub/types-bundle";
import { assertEventPresent } from "../asserts";
import {
  createBucket,
  createBucketAndSendNewStorageRequest,
  sendNewStorageRequest
} from "./fileHelpers";
import type { BspNetApi } from "./types";
import { advanceToBlock, sealBlock } from "./block";
import type { HexString } from "@polkadot/util/types";

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
      advanceToBlock(baseApi, {
        ...options,
        blockNumber
      }),

    sendNewStorageRequest: async (
      source: string,
      location: string,
      bucketId: H256,
      owner?: KeyringPair,
      mspId?: HexString
    ) => sendNewStorageRequest(baseApi, source, location, bucketId, owner, mspId),

    createBucketAndSendNewStorageRequest: async (
      source: string,
      location: string,
      bucketName: string,
      valuePropId?: HexString
    ) => createBucketAndSendNewStorageRequest(baseApi, source, location, bucketName, valuePropId),

    createBucket: async (bucketName: string, valuePropId?: HexString) =>
      createBucket(baseApi, bucketName, valuePropId),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
