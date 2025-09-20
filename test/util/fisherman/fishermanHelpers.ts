import type { EnrichedBspApi } from "../bspNet";
import { waitFor } from "../bspNet";

/**
 * Helper function to wait for fisherman to process an event
 * @param api - The API instance to use
 * @param searchPattern - The log pattern to search for
 * @param timeout - Timeout in milliseconds (default: 30000)
 * @returns Promise<boolean> - True if pattern found, false if timeout
 */
export async function waitForFishermanProcessing(
  api: EnrichedBspApi,
  searchPattern: string,
  timeout = 30000
): Promise<boolean> {
  try {
    await api.docker.waitForLog({
      searchString: searchPattern,
      containerName: "storage-hub-sh-fisherman-1",
      timeout
    });
    return true;
  } catch (error) {
    console.warn(`Failed to find fisherman log pattern "${searchPattern}": ${error}`);
    return false;
  }
}

/**
 * Ensures the fisherman node is ready and synced with latest block from collator (userApi)
 * @param userApi - The user API instance for docker commands
 * @param fishermanApi - The fisherman API instance to check block height
 * @param maxInitialBlock - Maximum acceptable initial block number (default: 5)
 * @returns Promise<void>
 */
export async function waitForFishermanReady(
  userApi: EnrichedBspApi,
  fishermanApi: EnrichedBspApi
): Promise<void> {
  // Wait for the fisherman service to be fully initialized
  await userApi.docker.waitForLog({
    searchString: "🎣 Fisherman service started",
    containerName: "storage-hub-sh-fisherman-1",
    timeout: 30000
  });

  // Wait for fisherman node to report idle state
  await userApi.docker.waitForLog({
    searchString: "💤 Idle",
    containerName: "storage-hub-sh-fisherman-1",
    timeout: 30000
  });

  const syncCurrentBlock = await userApi.rpc.chain.getBlock();
  const syncBlockNumber = syncCurrentBlock.block.header.number.toNumber();

  // Verify fisherman is at the correct block height
  await waitFor({
    lambda: async () => {
      const currentBlock = await fishermanApi.rpc.chain.getBlock();
      const blockNumber = currentBlock.block.header.number.toNumber();
      return blockNumber === syncBlockNumber;
    },
    iterations: 30,
    delay: 1000
  });
}
