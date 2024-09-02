import "@storagehub/types-bundle";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { BspNetApi } from "./types";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { createBucket, sealBlock, sendNewStorageRequest } from "./helpers";
import { assertEventPresent } from "../asserts";
import type { EventRecord } from "@polkadot/types/interfaces";
import { types as BundledTypes } from "@storagehub/types-bundle";

//TODO: Maybe make this a resource?
export const createApiObject = async (uri: string): Promise<BspNetApi> => {
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

    sendNewStorageRequest: async (source: string, location: string, bucketName: string) =>
      sendNewStorageRequest(baseApi, source, location, bucketName),

    createBucket: async (bucketName: string) => createBucket(baseApi, bucketName),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
