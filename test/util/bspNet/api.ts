import { ApiPromise, WsProvider } from "@polkadot/api";
import type { BspNetApi } from "./types";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { createBucket, getForestRoot, sealBlock, sendLoadFileRpc } from "./helpers";
import { assertEventPresent } from "../asserts";
import type { EventRecord, H256 } from "@polkadot/types/interfaces";

//TODO: Maybe make this a resource?
export const createApiObject = async (uri: string): Promise<BspNetApi> => {
  const baseApi = await ApiPromise.create({
    provider: new WsProvider(uri),
    noInitWarn: true
  });

  return Object.assign(baseApi, {
    sealBlock: async (
      call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
      signer?: KeyringPair
    ) => sealBlock(baseApi, call, signer),

    loadFile: async (localPath: string, remotePath: string, addressId: string, bucket: H256) =>
      sendLoadFileRpc(baseApi, localPath, remotePath, addressId, bucket),

    getForestRoot: async () => getForestRoot(baseApi),

    createBucket: async (bucketName: string) => createBucket(baseApi, bucketName),

    assertEvent: (module: string, method: string, events?: EventRecord[]) =>
      assertEventPresent(baseApi, module, method, events)
  });
};
