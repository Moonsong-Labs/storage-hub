import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";
import invariant from "tiny-invariant";
import type { H256 } from "@polkadot/types/interfaces";

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
  const iterations = 100;
  const delay = 100;

  // To allow node time to react on chain events
  for (let i = 0; i < iterations; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true
      });
      if (checkQuantity) {
        invariant(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspVolunteer`
        );
      }
      break;
    } catch {
      invariant(
        i < iterations - 1,
        `Failed to detect BSP volunteer extrinsic in txPool after ${(i * delay) / 1000}s`
      );
    }
  }

  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "AcceptedBspVolunteer", events);
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
  const iterations = 41;
  const delay = 50;

  // To allow node time to react on chain events
  for (let i = 0; i < iterations; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true
      });
      if (checkQuantity) {
        invariant(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspVolunteer`
        );
      }
      break;
    } catch {
      invariant(
        i < iterations - 1,
        `Failed to detect BSP volunteer extrinsic in txPool after ${(i * delay) / 1000}s`
      );
    }
  }
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
export const waitForBspStored = async (api: ApiPromise, checkQuantity?: number) => {
  // To allow time for local file transfer to complete (5s)
  const iterations = 50;
  const delay = 100;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true
      });
      if (checkQuantity) {
        invariant(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspVolunteer`
        );
      }
      const { events } = await sealBlock(api);
      assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
      break;
    } catch {
      invariant(
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
  // To allow time for local file transfer to complete (5s)
  const iterations = 50;
  const delay = 100;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true
      });
      if (checkQuantity) {
        invariant(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspVolunteer`
        );
      }
      break;
    } catch {
      invariant(
        i !== iterations,
        `Failed to detect BSP storage confirmation extrinsic in txPool after ${(i * delay) / 1000}s`
      );
    }
  }
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
      invariant(fileStorageResult.isFileFound, "File not found in file storage");
      break;
    } catch {
      invariant(
        i !== iterations,
        `Failed to detect BSP storage confirmation extrinsic in txPool after ${(i * delay) / 1000}s`
      );
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
  const iterations = 10;
  const delay = 1000;
  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const syncedBestBlock = await syncedApi.rpc.chain.getHeader();
      const bspBehindBestBlock = await bspBehindApi.rpc.chain.getHeader();
      invariant(
        syncedBestBlock.hash.toString() === bspBehindBestBlock.hash.toString(),
        "BSP did not catch up to the chain tip"
      );
      break;
    } catch {
      invariant(i !== iterations, `Failed to detect BSP catch up after ${(i * delay) / 1000}s`);
    }
  }
};
