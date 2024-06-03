import type { ApiPromise } from ".";

export const createBlock = async (api: ApiPromise) => {
  // TODO: This is gross but placeholder until we fix manual sealing in SH
  const maxAttempts = 20;

  for (let i = 0; i < maxAttempts; i++) {
    try {
      return await api.rpc.engine.createBlock(true, true);
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      console.log("Block creation failed, retrying...");
    }
  }
  throw new Error(`Block creation failed after ${maxAttempts} attempts`);
};
