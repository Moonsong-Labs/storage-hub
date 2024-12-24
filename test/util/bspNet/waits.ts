import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";
import assert from "node:assert";
import type { Address, H256 } from "@polkadot/types/interfaces";
import type { WaitForTxOptions } from "./test-api";

/**
 * Generic function to wait for a transaction in the pool.
 *
 * If the expected amount of extrinsics is 0, this function will return immediately.
 */
export const waitForTxInPool = async (api: ApiPromise, options: WaitForTxOptions) => {
  const {
    module,
    method,
    checkQuantity,
    shouldSeal = false,
    expectedEvent,
    timeout = 1000,
    verbose = false
  } = options;
  // Handle the case where the expected amount of extrinsics is 0
  if (checkQuantity === 0) {
    // If the expected amount is 0, we can return immediately
    verbose &&
      console.log(
        `Expected 0 extrinsics for ${module}.${method}. Skipping wait for extrinsic in txPool.`
      );
    return;
  }

  // To allow node time to react on chain events
  try {
    const matches = await assertExtrinsicPresent(api, {
      module,
      method,
      checkTxPool: true,
      timeout
    });
    if (checkQuantity) {
      assert(
        matches.length === checkQuantity,
        `Expected ${checkQuantity} extrinsics, but found ${matches.length} for ${module}.${method}`
      );
    }

    if (shouldSeal) {
      const { events } = await sealBlock(api);
      if (expectedEvent) {
        assertEventPresent(api, module, expectedEvent, events);
      }
    }
  } catch (e) {
    throw new Error(`Failed to detect ${module}.${method} extrinsic in txPool. Error: ${e}`);
  }
};

/**
 * Waits for a BSP to volunteer for a storage request.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'bspVolunteer' extrinsic in the transaction pool.
 * 3. Seals a block and verifies the presence of an 'AcceptedBspVolunteer' event.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a BSP has volunteered and been accepted.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForBspVolunteer = async (api: ApiPromise, checkQuantity?: number) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "bspVolunteer",
    checkQuantity,
    shouldSeal: true,
    expectedEvent: "AcceptedBspVolunteer"
  });
};

/**
 * Waits for a BSP to send to the tx pool the extrinsic to volunteer for a storage request.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'bspVolunteer' extrinsic in the transaction pool.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a BSP has volunteered and been accepted.
 *
 * @throws Will throw an error if the expected extrinsic is not found.
 */
export const waitForBspVolunteerWithoutSealing = async (
  api: ApiPromise,
  checkQuantity?: number
) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "bspVolunteer",
    checkQuantity
  });
};

/**
 * Waits for a BSP to confirm storing a file.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for local file transfer.
 * 2. Checks for the presence of a 'bspConfirmStoring' extrinsic in the transaction pool.
 * 3. Seals a block and verifies the presence of a 'BspConfirmedStoring' event.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a BSP has confirmed storing a file.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForBspStored = async (
  api: ApiPromise,
  checkQuantity?: number,
  bspAccount?: Address
) => {
  // To allow time for local file transfer to complete (10s)
  const iterations = 100;
  const delay = 200;

  // We do this because a BSP cannot call `bspConfirmStoring` in the same block in which it has to submit a proof, since it can only send one root-changing transaction per block and proof submission is prioritized.
  assert(
    !(bspAccount && checkQuantity && checkQuantity > 1),
    "Invalid parameters: `waitForBspStored` cannot be used with an amount of extrinsics to wait for bigger than 1 if a BSP ID was specified."
  );

  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);

      // check if we have a submitProof extrinsic
      if (bspAccount) {
        const txs = await api.rpc.author.pendingExtrinsics();
        const match = txs.filter(
          (tx) => tx.method.method === "submitProof" && tx.signer.eq(bspAccount)
        );

        // If we have a submit proof event at the same time we are trying to confirm storage
        // we need to advance one block because the two event cannot happen at the same time
        if (match.length === 1) {
          await sealBlock(api);
        }
      }

      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true,
        timeout: 300
      });
      if (checkQuantity) {
        assert(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspConfirmStoring`
        );
      }
      const { events } = await sealBlock(api);
      assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
      break;
    } catch {
      assert(
        i !== iterations,
        `Failed to detect BSP storage confirmation extrinsic in txPool after ${(i * delay) / 1000}s`
      );
    }
  }
};

/**
 * Waits for a BSP to send to the tx pool the extrinsic to confirm storing a file.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for local file transfer.
 * 2. Checks for the presence of a 'bspConfirmStoring' extrinsic in the transaction pool.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a BSP has submitted to the tx pool the extrinsic to confirm storing a file.
 *
 * @throws Will throw an error if the expected extrinsic is not found.
 */
export const waitForBspStoredWithoutSealing = async (api: ApiPromise, checkQuantity?: number) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "bspConfirmStoring",
    checkQuantity,
    timeout: 10000
  });
};

/**
 * Waits for a BSP to complete storing a file in its file storage.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for local file transfer.
 * 2. Checks for the FileFound return from the isFileInFileStorage RPC method.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for in the file storage.
 * @returns A Promise that resolves when a BSP has correctly stored a file in its file storage.
 *
 * @throws Will throw an error if the file is not complete in the file storage after a timeout.
 */
export const waitForBspFileStorageComplete = async (api: ApiPromise, fileKey: H256 | string) => {
  // To allow time for local file transfer to complete (10s)
  const iterations = 10;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const fileStorageResult = await api.rpc.storagehubclient.isFileInFileStorage(fileKey);
      assert(fileStorageResult.isFileFound, "File not found in file storage");
      break;
    } catch {
      assert(
        i !== iterations,
        `Failed to detect BSP file in file storage after ${(i * delay) / 1000}s`
      );
    }
  }
};

/**
 * Waits for a BSP to complete deleting a file from its forest storage.
 *
 * This function performs the following steps:
 * 1. Waits for a period of time to allow the BSP to delete the file from its forest storage.
 * 2. Checks for the `false` return from the isFileInForest RPC method.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for deletion the forest storage.
 * @returns A Promise that resolves when a BSP has correctly deleted a file from its forest storage.
 *
 * @throws Will throw an error if the file is still in the forest storage after a timeout.
 */
export const waitForBspFileDeletionComplete = async (api: ApiPromise, fileKey: H256 | string) => {
  // To allow time for file deletion to complete (10s)
  const iterations = 20;
  const delay = 500;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const fileDeletionResult = await api.rpc.storagehubclient.isFileInForest(null, fileKey);
      assert(fileDeletionResult.isFalse, "File still in forest storage");
      break;
    } catch {
      assert(i !== iterations, `Failed to detect BSP file deletion after ${(i * delay) / 1000}s`);
    }
  }
};

/**
 * Waits for a BSP to catch up to the top of the chain.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for the BSP to catch up.
 * 2. Checks for the best block to make sure it matches the chain tip.
 *
 * @param syncedApi - The ApiPromise that is already synced to the top of the chain.
 * @param bspBehindApi - The ApiPromise instance that is behind the chain tip.
 * @returns A Promise that resolves when a BSP has correctly catched up to the top of the chain.
 *
 * @throws Will throw an error if the BSP doesn't catch up after a timeout.
 */
export const waitForBspToCatchUpToChainTip = async (
  syncedApi: ApiPromise,
  bspBehindApi: ApiPromise
) => {
  // To allow time for BSP to catch up to the tip of the chain (10s)
  const iterations = 100;
  const delay = 100;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const syncedBestBlock = await syncedApi.rpc.chain.getHeader();
      const bspBehindBestBlock = await bspBehindApi.rpc.chain.getHeader();
      assert(
        syncedBestBlock.hash.toString() === bspBehindBestBlock.hash.toString(),
        "BSP did not catch up to the chain tip"
      );
      break;
    } catch {
      assert(i !== iterations, `Failed to detect BSP catch up after ${(i * delay) / 1000}s`);
    }
  }
};

export const waitForBlockImported = async (api: ApiPromise, blockHash: string) => {
  // To allow time for BSP to catch up to the tip of the chain (10s)
  const iterations = 100;
  const delay = 100;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const block = await api.rpc.chain.getBlock(blockHash);
      assert(block.block.header.number.toNumber() > 0, "Block not imported");
      break;
    } catch {
      assert(i !== iterations, `Failed to detect block imported after ${(i * delay) / 1000}s`);
    }
  }
};

// TODO: Maybe we should refactor these to a different file under `mspNet` or something along those lines
/**
 * Waits for a MSP to respond to storage requests.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'mspRespondStorageRequestsMultipleBuckets' extrinsic in the transaction pool.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a MSP has sent a response to storage requests.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForMspResponseWithoutSealing = async (api: ApiPromise, checkQuantity?: number) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "mspRespondStorageRequestsMultipleBuckets",
    checkQuantity,
    timeout: 10000
  });
};

/**
 * Options for the `waitFor` function.
 * @param lambda - The condition to wait for.
 * @param iterations - The number of iterations to wait for the condition to be true.
 * @param delay - The delay between iterations.
 */
export interface WaitForOptions {
  lambda: () => Promise<boolean>;
  iterations?: number;
  delay?: number;
}

/**
 * Waits for an arbitrary condition to be true. It keeps polling the condition until it is true or
 * a timeout is reached.
 */
export const waitFor = async (options: WaitForOptions) => {
  const { lambda, iterations = 100, delay = 100 } = options;

  for (let i = 0; i < iterations; i++) {
    try {
      await sleep(delay);
      const result = await lambda();
      if (result) {
        return;
      }
    } catch (e: unknown) {
      if (i === iterations - 1) {
        const errorMessage = e instanceof Error ? e.message : String(e);
        throw new Error(`Failed after ${(iterations * delay) / 1000}s: ${errorMessage}`);
      }
    }
  }
  throw new Error(`Failed after ${(iterations * delay) / 1000}s`);
};

/**
 * Waits for a MSP to complete storing a file in its file storage.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for local file transfer.
 * 2. Checks for the FileFound return from the isFileInFileStorage RPC method.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for in the file storage.
 * @returns A Promise that resolves when the MSP has correctly stored a file in its file storage.
 *
 * @throws Will throw an error if the file is not complete in the file storage after a timeout.
 */
export const waitForMspFileStorageComplete = async (api: ApiPromise, fileKey: H256 | string) => {
  // To allow time for local file transfer to complete (10s)
  const iterations = 10;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const fileStorageResult = await api.rpc.storagehubclient.isFileInFileStorage(fileKey);
      assert(fileStorageResult.isFileFound, "File not found in file storage");
      break;
    } catch {
      assert(
        i !== iterations,
        `Failed to detect MSP file in file storage after ${(i * delay) / 1000}s`
      );
    }
  }
};
