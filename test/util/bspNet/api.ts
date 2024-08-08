import "@storagehub/api-augment";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { BspNetApi } from "./types";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { createBucket, sealBlock } from "./helpers";
import { assertEventPresent } from "../asserts";
import type { EventRecord } from "@polkadot/types/interfaces";
import * as definitions from "../../node_modules/@storagehub/api-augment/src/interfaces/definitions";

//TODO: Maybe make this a resource?
export const createApiObject = async (uri: string): Promise<BspNetApi> => {
  const types = Object.values(definitions).reduce((res, { types }) => ({ ...res, ...types }), {});
  const rpcMethods = Object.entries(definitions).reduce(
    (res: Record<string, any>, [key, { rpc }]) => {
      if (rpc) {
        res[key] = rpc;
      }
      return res;
    },
    {}
  );
  const runtime = Object.entries(definitions).reduce(
    (res: Record<string, any>, [, { runtime }]) => {
      if (runtime) {
        Object.assign(res, runtime);
      }
      return res;
    },
    {}
  );

  const baseApi = await ApiPromise.create({
    provider: new WsProvider(uri),
    noInitWarn: true,
    types,
    rpc: rpcMethods,
    runtime
  });

  return Object.assign(baseApi, {
    sealBlock: async (
      call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
      signer?: KeyringPair
    ) => sealBlock(baseApi, call, signer),

    createBucket: async (bucketName: string) => createBucket(baseApi, bucketName),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
