import { ApiPromise, WsProvider } from "@polkadot/api";
import type { BspNetApi } from "./types";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { sealBlock, sendFileSendRpc } from "./helpers";

export const createApiObject = async (uri: string): Promise<BspNetApi> => {
  const baseApi = await ApiPromise.create({
    provider: new WsProvider(uri),
    noInitWarn: true,
  });

  const extendedApi = Object.assign(baseApi, {
    sealBlock: async (
      call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
      signer?: KeyringPair,
    ) => sealBlock(baseApi, call, signer),
    
    sendFile: async (
      localPath: string,
      remotePath: string,
      addressId: string,
    ) => sendFileSendRpc(baseApi, localPath, remotePath, addressId),
  });

  return extendedApi;
};