import "@storagehub/types-bundle";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord, H256 } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { HexString } from "@polkadot/util/types";
import { types as BundledTypes } from "@storagehub/types-bundle";
import { assertEventPresent } from "../asserts";
import {
  alith,
  ethBspDownKey,
  ethBspKey,
  ethBspThreeKey,
  ethBspTwoKey,
  ethMspDownKey,
  ethMspKey,
  ethMspThreeKey,
  ethMspTwoKey,
  ethShUser
} from "../evmNet/keyring";
import {
  alice,
  bspDownKey,
  bspKey,
  bspThreeKey,
  bspTwoKey,
  mspDownKey,
  mspKey,
  mspThreeKey,
  mspTwoKey,
  shUser
} from "../pjsKeyring";
import { advanceToBlock, sealBlock } from "./block";
import {
  createBucket,
  createBucketAndSendNewStorageRequest,
  sendNewStorageRequest
} from "./fileHelpers";
import type { BspNetApi } from "./types";

/**
 * DEPRECATED: Use BspNetTestApi.create() instead
 *
 */
export const createApiObject = async (
  uri: `ws://${string}` | `wss://${string}`,
  runtimeType: "parachain" | "solochain" = "parachain"
): Promise<BspNetApi> => {
  const baseApi = await ApiPromise.create({
    provider: new WsProvider(uri),
    noInitWarn: true,
    throwOnConnect: false,
    throwOnUnknown: false,
    typesBundle: BundledTypes
  });

  const accounts =
    runtimeType === "solochain"
      ? {
          sudo: alith,
          bspKey: ethBspKey,
          bspDownKey: ethBspDownKey,
          bspTwoKey: ethBspTwoKey,
          bspThreeKey: ethBspThreeKey,
          mspKey: ethMspKey,
          mspDownKey: ethMspDownKey,
          mspTwoKey: ethMspTwoKey,
          mspThreeKey: ethMspThreeKey,
          shUser: ethShUser
        }
      : {
          sudo: alice,
          bspKey,
          bspDownKey,
          bspTwoKey,
          bspThreeKey,
          mspKey,
          mspDownKey,
          mspTwoKey,
          mspThreeKey,
          shUser
        };

  return Object.assign(baseApi, {
    accounts,
    sealBlock: async (
      calls?:
        | SubmittableExtrinsic<"promise", ISubmittableResult>
        | SubmittableExtrinsic<"promise", ISubmittableResult>[],
      signer?: KeyringPair
    ) => sealBlock(baseApi, calls, signer ?? accounts.sudo),

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
    ) =>
      sendNewStorageRequest(baseApi, source, location, bucketId, owner ?? accounts.shUser, mspId),

    createBucketAndSendNewStorageRequest: async (
      source: string,
      location: string,
      bucketName: string,
      valuePropId?: HexString
    ) =>
      createBucketAndSendNewStorageRequest(
        baseApi,
        source,
        location,
        bucketName,
        accounts.shUser,
        valuePropId
      ),

    createBucket: async (bucketName: string, valuePropId?: HexString) =>
      createBucket(baseApi, bucketName, accounts.shUser, valuePropId),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
