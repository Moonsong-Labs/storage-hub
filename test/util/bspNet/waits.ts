import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { Address, EventRecord, H256 } from "@polkadot/types/interfaces";
import * as Assertions from "../asserts";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";
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
    strictQuantity = false,
    shouldSeal = false,
    finalizeBlock = true,
    expectedEvent,
    timeout = 10000,
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
      timeout,
      assertLength: checkQuantity,
      exactLength: strictQuantity
    });
    if (checkQuantity && strictQuantity) {
      assert(
        matches.length === checkQuantity,
        `Expected ${checkQuantity} extrinsics, but found ${matches.length} for ${module}.${method}`
      );
    } else if (checkQuantity && !strictQuantity) {
      assert(
        matches.length >= checkQuantity,
        `Expected at least ${checkQuantity} extrinsics, but found ${matches.length} for ${module}.${method}`
      );
    }

    if (shouldSeal) {
      const { events } = await sealBlock(
        api,
        undefined,
        undefined,
        undefined,
        undefined,
        finalizeBlock
      );
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
    strictQuantity: (checkQuantity ?? 0) > 0,
    shouldSeal: true,
    expectedEvent: "AcceptedBspVolunteer",
    finalizeBlock: true
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
 * @param bspAccount - Optional param to specify the BSP Account ID that may be sending submit proof extrinsics.
 * @param shouldSealBlock - Optional param to specify if the block should be sealed with the confirmation extrinsic. Defaults to true.
 * @param shouldFinalizeBlock - Optional param to specify if the block should be finalized after sealing. Defaults to true.
 * @returns A Promise that resolves when a BSP has confirmed storing a file.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForBspStored = async (
  api: ApiPromise,
  checkQuantity?: number,
  bspAccount?: Address,
  timeoutMs?: number,
  shouldSealBlock = true,
  shouldFinalizeBlock = true
) => {
  // To allow time for local file transfer to complete.
  // Default is 10s, with iterations of 100ms delay.
  const iterations = timeoutMs ? Math.ceil(timeoutMs / 100) : 100;
  const delay = 100;

  // This check is because a BSP cannot confirm storing a file in the same block in which it has to submit a proof,
  // since it can only send one root-changing transaction per block and proof submission is prioritized.
  assert(
    !(bspAccount && checkQuantity && checkQuantity > 1),
    "Invalid parameters: `waitForBspStored` cannot be used with an amount of extrinsics to wait for bigger than 1 if a BSP ID was specified."
  );

  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);

      // Check if there's a pending submit proof extrinsic from the BSP account.
      if (bspAccount) {
        const txs = await api.rpc.author.pendingExtrinsics();
        const match = txs.filter(
          (tx) => tx.method.method === "submitProof" && tx.signer.eq(bspAccount)
        );

        // If there's a submit proof extrinsic pending, advance one block to allow the BSP to submit
        // the proof and be able to confirm storing the file and continue waiting.
        if (match.length === 1) {
          await sealBlock(api, undefined, undefined, undefined, undefined, shouldFinalizeBlock);
          continue;
        }
      }

      // Check to see if the quantity of confirm storing extrinsics required are in the TX pool.
      await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true,
        timeout: 100, // Small timeout since we are already waiting between checks.
        assertLength: checkQuantity,
        exactLength: (checkQuantity ?? 0) > 0
      });

      // If there are exactly checkQuantity extrinsics (or at least one if checkQuantity is not defined), seal the block and check for the event.
      if (shouldSealBlock) {
        const { events } = await sealBlock(
          api,
          undefined,
          undefined,
          undefined,
          undefined,
          shouldFinalizeBlock
        );
        assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
      }
      break;
    } catch (error) {
      assert(
        i !== iterations,
        `Failed to confirm BSP storage after ${(i * delay) / 1000}s. Last error: ${error}`
      );
    }
  }
};

/**
 * Waits for a Provider to complete storing a file in its file storage.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for local file transfer.
 * 2. Checks for the FileFound return from the isFileInFileStorage RPC method.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for in the file storage.
 * @returns A Promise that resolves when the Provider has correctly stored a file in its file storage.
 *
 * @throws Will throw an error if the file is not complete in the file storage after a timeout.
 */
export const waitForFileStorageComplete = async (api: ApiPromise, fileKey: H256 | string) => {
  // To allow time for local file transfer to complete (20s)
  const iterations = 20;
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
        `Failed to detect file in Provider's file storage after ${(i * delay) / 1000}s`
      );
    }
  }
};

/**
 * Waits for a Provider to complete deleting a file from its file storage.
 *
 * This function performs the following steps:
 * 1. Waits for a period of time to allow for deletion.
 * 2. Checks for the FileNotFound return from the isFileInFileStorage RPC method.
 * 3. Repeats until the timeout is reached.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for in the file storage.
 * @returns A Promise that resolves when the Provider has correctly deleted a file from its file storage.
 *
 * @throws Will throw an error if the file is not deleted from the file storage after a timeout.
 */
export const waitForFileDeletionFromFileStorageComplete = async (
  api: ApiPromise,
  fileKey: H256 | string
) => {
  // To allow time for deletion to complete (20s)
  const iterations = 20;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const fileStorageResult = await api.rpc.storagehubclient.isFileInFileStorage(fileKey);
      assert(fileStorageResult.isFileNotFound, "File still in file storage");
      break;
    } catch {
      assert(
        i !== iterations,
        `Failed to detect file deletion from Provider's file storage after ${(i * delay) / 1000}s`
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
 * Waits for an MSP to complete deleting a file from a bucket in its forest.
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The file key to check for deletion the forest storage.
 * @param bucketId - The bucket ID to check for deletion the forest storage.
 * @returns A Promise that resolves when the MSP has correctly deleted the file from its bucket forest storage.
 */
export const waitForMspBucketFileDeletionComplete = async (
  api: ApiPromise,
  fileKey: H256 | string,
  bucketId: H256 | string
) => {
  // To allow time for file deletion to complete (10s)
  const iterations = 20;
  const delay = 500;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const fileDeletionResult = await api.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
      assert(fileDeletionResult.isFalse, "File still in forest storage");
      break;
    } catch {
      assert(i !== iterations, `Failed to detect MSP file deletion after ${(i * delay) / 1000}s`);
    }
  }
};

/**
 * Waits for a MSP to complete deleting a bucket from its forest storage.
 *
 * This function performs the following steps:
 * 1. Waits for a period of time to allow the MSP to delete the bucket from its forest storage.
 * 2. Checks for the `None` return from the getForestRoot RPC method.
 *
 * @param api - The ApiPromise instance to interact with the RPC.
 * @param fileKey - The bucket ID to check for deletion the forest storage.
 * @returns A Promise that resolves when the MSP has correctly deleted a bucket from its forest storage.
 *
 * @throws Will throw an error if the bucket is still in the forest storage after a timeout.
 */
export const waitForMspBucketDeletionComplete = async (
  api: ApiPromise,
  bucketId: H256 | string
) => {
  // To allow time for bucket deletion to complete (20s)
  const iterations = 20;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const bucketDeletionResult = await api.rpc.storagehubclient.getForestRoot(bucketId);
      assert(bucketDeletionResult.isNone, "Bucket still in forest storage");
      break;
    } catch {
      assert(i !== iterations, `Failed to detect MSP bucket deletion after ${(i * delay) / 1000}s`);
    }
  }
};

/**
 * Waits for a BSP to catch up to the top of the chain.
 *
 * This function performs the following steps:
 * 1. Waits for a longer period to allow for the BSP to catch up.
 * 2. Checks for the best block to make sure it matches the chain tip.
 * 3. Attempts to trigger gossip between the nodes by building a few additional blocks
 *    after some time has passed.
 *
 * @param nodeSyncedApi - The ApiPromise that is already synced to the top of the chain.
 * @param nodeBehindApi - The ApiPromise instance that is behind the chain tip.
 * @returns A Promise that resolves when a node has correctly catched up to the top of the chain.
 *
 * @throws Will throw an error if the node doesn't catch up after a timeout.
 */
export const waitForNodeToCatchUpToChainTip = async (
  nodeSyncedApi: ApiPromise,
  nodeBehindApi: ApiPromise
) => {
  // To allow time for node to catch up to the tip of the chain (40s)
  // We wait for 10s for the two nodes to sync to the same block, and if by that time they
  // haven't caught up, we build a block to trigger gossip between the nodes.
  // This is because in some edge cases, the latest block from the synced node might not have
  // been gossiped to the "behind" node, before it went into syncing mode.
  const blockBuildingIterations = 4;
  const iterations = 100;
  const delay = 100;
  for (let i = 0; i < blockBuildingIterations + 1; i++) {
    try {
      for (let j = 0; j < iterations + 1; j++) {
        try {
          await sleep(delay);
          const syncedBestBlock = await nodeSyncedApi.rpc.chain.getHeader();
          const nodeBehindBestBlock = await nodeBehindApi.rpc.chain.getHeader();

          assert(
            syncedBestBlock.hash.toString() === nodeBehindBestBlock.hash.toString(),
            "Node did not catch up to the chain tip"
          );
          break;
        } catch {
          assert(j !== iterations, `Failed to detect node catch up after ${(j * delay) / 1000}s`);
        }
      }
    } catch {
      assert(
        i !== blockBuildingIterations,
        `Failed to detect node catch up after ${(i * iterations * delay) / 1000}s`
      );
      // If they're still not in sync, build a block to trigger gossip between the nodes.
      await nodeSyncedApi.rpc.engine.createBlock(true, true);
    }
  }
};

export const waitForBlockImported = async (api: ApiPromise, blockHash: string) => {
  // To allow time for node to catch up to the tip of the chain (10s)
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
 * @param timeoutMs - Optional param to specify the timeout in milliseconds.
 * @returns A Promise that resolves when a MSP has sent a response to storage requests.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForMspResponseWithoutSealing = async (
  api: ApiPromise,
  checkQuantity?: number,
  timeoutMs = 10000
) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "mspRespondStorageRequestsMultipleBuckets",
    checkQuantity,
    timeout: timeoutMs
  });
};

/**
 * Waits for a MSP to submit a proof for a pending file deletion request.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'pendingFileDeletionRequestSubmitProof' extrinsic in the transaction pool.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a MSP has submitted a proof for a pending file deletion request.
 *
 * @throws Will throw an error if the expected extrinsic is not found.
 */
export const waitForMspPendingFileDeletionRequestSubmitProof = async (
  api: ApiPromise,
  checkQuantity?: number
) => {
  await waitForTxInPool(api, {
    module: "fileSystem",
    method: "pendingFileDeletionRequestSubmitProof",
    checkQuantity,
    timeout: 10000
  });
};

/**
 * Waits for a block where the given address has no pending extrinsics.
 *
 * This can be used to wait for a block where it is safe to send a transaction signed by the given address,
 * without risking it clashing with another transaction with the same nonce already in the pool. For example,
 * BSP nodes are often sending transactions, so if you want to send a transaction using one of the BSP keys,
 * you should wait for the BSP to have no pending extrinsics before sending the transaction.
 *
 * IMPORTANT: As long as the address keeps having pending extrinsics, this function will keep waiting and building
 * blocks to include such transactions.
 *
 * @param api - The ApiPromise instance.
 * @param address - The address of the account to wait for.
 */
export const waitForAvailabilityToSendTx = async (
  api: ApiPromise,
  address: string,
  iterations = 100,
  delay = 500
) => {
  let isTxFromAddressPresent = false;
  let its = iterations;
  do {
    await sleep(delay);

    // Check if the address has pending extrinsics
    const result = await api.rpc.author.pendingExtrinsics();
    isTxFromAddressPresent = result.some((tx) => tx.signer.toString() === address);
    if (isTxFromAddressPresent) {
      // Build a block with the transactions from the address
      await sealBlock(api);
    }
  } while (isTxFromAddressPresent && its-- > 0);

  if (isTxFromAddressPresent) {
    // If the address still has pending extrinsics after the maximum number of iterations, throw an error
    throw new Error(`Failed after ${iterations} iterations and ${(iterations * delay) / 1000}s`);
  }
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
  const { lambda, iterations = 200, delay = 100 } = options;

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

export const waitForStorageRequestNotOnChain = async (api: ApiPromise, fileKey: H256 | string) => {
  // 10 iterations at 1 second per iteration = 10 seconds wait time
  const iterations = 10;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      // Try to get the storage request from the chain
      const result = await api.query.fileSystem.storageRequests(fileKey);

      // If the storage request wasn't found, it has been fulfilled/expired/rejected.
      if (result.isNone) {
        return;
      }

      // If it has been found, seal a new block and wait for the next iteration to check if
      // it has been fulfilled/expired/rejected.
      await sealBlock(api);
    } catch {
      assert(
        i !== iterations,
        `Detected storage request in on-chain storage after ${(i * delay) / 1000}s`
      );
    }
  }
};

export const waitForStorageRequestFulfilled = async (api: ApiPromise, fileKey: H256 | string) => {
  // 10 iterations at 1 second per iteration = 10 seconds wait time
  const iterations = 10;
  const delay = 1000;

  // First check that the storage request exists in storage, since otherwise the StorageRequestFulfilled event
  // will never be emitted.
  const storageRequest = await api.query.fileSystem.storageRequests(fileKey);
  assert(
    storageRequest.isSome,
    "Storage request not found in storage but `waitForStorageRequestFulfilled` was called"
  );

  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      // Check in the events of the last block to see if any StorageRequestFulfilled event were emitted and get them.
      const previous_block_events = (await api.query.system.events()) as EventRecord[];
      const storageRequestFulfilledEvents = Assertions.assertEventMany(
        api,
        "fileSystem",
        "StorageRequestFulfilled",
        previous_block_events
      );

      // Check if any of the events are for the file key we are waiting for.
      const storageRequestFulfilledEvent = storageRequestFulfilledEvents.find((event) => {
        const storageRequestFulfilledEventData =
          api.events.fileSystem.StorageRequestFulfilled.is(event.event) && event.event.data;
        assert(
          storageRequestFulfilledEventData,
          "Event doesn't match type but eventMany should have filtered it out"
        );
        return storageRequestFulfilledEventData.fileKey.toString() === fileKey.toString();
      });

      // If the event was found, check to make sure the storage request is not on-chain and return.
      if (storageRequestFulfilledEvent) {
        await waitForStorageRequestNotOnChain(api, fileKey);
        return;
      }

      // If the event was not found, seal a new block and wait for the next iteration to check if
      // it has been emitted.
      await sealBlock(api);
    } catch {
      assert(
        i !== iterations,
        `Storage request has not been fulfilled after ${(i * delay) / 1000}s`
      );
    }
  }
};
