import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";
import invariant from "tiny-invariant";

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
export const waitForBspStored = async (api: ApiPromise, checkQuantity?: number) => {
  // To allow time for local file transfer to complete (10s)
  const iterations = 100;
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
