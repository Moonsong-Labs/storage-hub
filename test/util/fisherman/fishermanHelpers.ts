import type { EnrichedBspApi } from "../bspNet";

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
