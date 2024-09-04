import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./block";

/**
 * Waits for a BSP to volunteer for a storage request.
 *
 * This function performs the following steps:
 * 1. Waits for a short period to allow the node to react.
 * 2. Checks for the presence of a 'bspVolunteer' extrinsic in the transaction pool.
 * 3. Seals a block and verifies the presence of an 'AcceptedBspVolunteer' event.
 *
 * @param api - The ApiPromise instance to interact with the blockchain.
 * @returns A Promise that resolves when a BSP has volunteered and been accepted.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 *
 * @todo Implement polling instead of fixed sleep.
 */
export const waitForBspVolunteer = async (api: ApiPromise) => {
  // To allow node to react
  // TODO poll
  await sleep(500);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspVolunteer",
    checkTxPool: true
  });
  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "AcceptedBspVolunteer", events);
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
 * @returns A Promise that resolves when a BSP has confirmed storing a file.
 *
 * @throws Will throw an error if the expected extrinsic or event is not found.
 *
 * @todo Implement polling instead of fixed sleep.
 */
export const waitForBspStored = async (api: ApiPromise) => {
  // To allow for local file transfer
  // TODO poll
  await sleep(5000);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspConfirmStoring",
    checkTxPool: true
  });
  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
};

/**
 * Namespace containing wait functions for BSP-related events.
 *
 * This namespace provides a convenient interface to access various wait functions
 * related to BSP operations. It's designed to be used as part of the enhanced BSP API,
 * offering a cohesive set of tools for managing asynchronous operations in testing scenarios.
 */
export namespace Waits {
  /**  * @see {@link waitForBspVolunteer} for waiting on BSP volunteer events */
  export const bspVolunteer = waitForBspVolunteer;
  /** * @see {@link waitForBspStored} for waiting on BSP storage confirmation events */
  export const bspStored = waitForBspStored;
}
