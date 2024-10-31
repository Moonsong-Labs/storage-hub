import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";
import invariant from "tiny-invariant";
import type { H256 } from "@polkadot/types/interfaces";
import type { Hash } from "crypto";

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
        checkTxPool: true,
        timeout: 100
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
  const iterations = 100;
  const delay = 100;

  // To allow node time to react on chain events
  for (let i = 0; i < iterations; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
        timeout: 100
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
export const waitForBspStored = async (api: ApiPromise, checkQuantity?: number, bspId?: string) => { // TODO: add bsp ID to check if is not confirming storing
  // To allow time for local file transfer to complete (10s)
  const iterations = 100;
  const delay = 100;

  if ( bspId && checkQuantity && checkQuantity > 1 ) {
    // We do this because `bspConfirmStoring` cannot happened in the same block in which a BSP submit a proof.
    throw new Error("Cannot use `waitForBspStored` with a bspId and an amount of extrinsec bigger than 1.")
  }

  for (let i = 0; i < iterations + 1; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true,
        timeout: 100
      });
      if (checkQuantity) {
        invariant(
          matches.length === checkQuantity,
          `Expected ${checkQuantity} extrinsics, but found ${matches.length} for fileSystem.bspConfirmStoring`
        );
      }
      const { events } = await sealBlock(api);
      assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
      break;
    } catch {

      console.log(bspId);

      if ( bspId ) {
        try {
          // In cases the bsp is submitting a proof at the same time is trying to confirmStoring
          const matches = await assertExtrinsicPresent(api, {
            module: "proofsDealer",
            method: "submitProof",
            checkTxPool: true,
            timeout: 100
          });

          console.log(matches)

        } catch {
          continue;
        }
      }

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
        checkTxPool: true,
        timeout: 100
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
      invariant(fileDeletionResult.isFalse, "File still in forest storage");
      break;
    } catch {
      invariant(
        i !== iterations,
        `Failed to detect BSP file deletion after ${(i * delay) / 1000}s`
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

/**
 * Waits for a MSP to respond to storage requests.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'mspRespondStorageRequestsMultipleBuckets' extrinsic in the transaction pool.
 * 3. Seals a block and verifies the presence of an 'MspRespondedToStorageRequests' event.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @param checkQuantity - Optional param to specify the number of expected extrinsics.
 * @returns A Promise that resolves when a MSP has sent a response to storage requests.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 */
export const waitForMspResponse = async (api: ApiPromise, checkQuantity?: number) => {
  const iterations = 41;
  const delay = 50;

  // To allow node time to react on chain events
  for (let i = 0; i < iterations; i++) {
    try {
      await sleep(delay);
      const matches = await assertExtrinsicPresent(api, {
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
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
  const mspRespondEvent = assertEventPresent(
    api,
    "fileSystem",
    "MspRespondedToStorageRequests",
    events
  );

  const mspRespondDataBlob =
    api.events.fileSystem.MspRespondedToStorageRequests.is(mspRespondEvent.event) &&
    mspRespondEvent.event.data;

  if (!mspRespondDataBlob) {
    throw new Error("Event doesn't match Type");
  }

  const responses = mspRespondDataBlob.results.responses;

  return responses;
};
