import type { ApiPromise } from ".";

export const createBlock = async (api: ApiPromise) => {
  // TODO: Add Events and extrinsics to the block returned by this function

  return await api.rpc.engine.createBlock(true, true);
};
