import type { EnrichedBspApi } from "../bspNet";
import { sleep } from "../timer";

/**
 * Helper function to wait for delete_file extrinsic in transaction pool
 * @param api - The API instance to use
 * @param _fileKey - The file key (currently unused but kept for future filtering)
 * @param expectedCount - Number of expected delete_file extrinsics (default: 1)
 * @param timeout - Timeout in milliseconds (default: 10000)
 * @returns Promise<boolean> - True if expected number of extrinsics found, false if timeout
 */
export async function waitForDeleteFileExtrinsic(
  api: EnrichedBspApi,
  expectedCount = 1,
  timeout = 10000
): Promise<boolean> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const pendingTxs = await api.rpc.author.pendingExtrinsics();
      const deleteFileTxs = pendingTxs.filter(
        (tx) => tx.method.method === "deleteFile" && tx.method.section === "fileSystem"
      );

      if (deleteFileTxs.length >= expectedCount) {
        return true;
      }
    } catch (error) {
      console.warn("Error checking pending extrinsics:", error);
    }

    await sleep(500);
  }

  return false;
}

/**
 * Helper function to wait for delete_file_for_incomplete_storage_request extrinsic in transaction pool
 * @param api - The API instance to use
 * @param expectedCount - Number of expected delete_file_for_incomplete_storage_request extrinsics (default: 1)
 * @param timeout - Timeout in milliseconds (default: 10000)
 * @returns Promise<boolean> - True if expected number of extrinsics found, false if timeout
 */
export async function waitForDeleteFileForIncompleteStorageRequestExtrinsic(
  api: EnrichedBspApi,
  expectedCount = 1,
  timeout = 10000
): Promise<boolean> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const pendingTxs = await api.rpc.author.pendingExtrinsics();
      const deleteFileTxs = pendingTxs.filter(
        (tx) =>
          tx.method.method === "deleteFileForIncompleteStorageRequest" &&
          tx.method.section === "fileSystem"
      );

      if (deleteFileTxs.length >= expectedCount) {
        return true;
      }
    } catch (error) {
      console.warn("Error checking pending extrinsics:", error);
    }

    await sleep(500);
  }

  return false;
}

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
