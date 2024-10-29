import type { H256 } from "@polkadot/types/interfaces";
import type { BspNetTestApi } from "../network";
import type { BspNetConfig } from "../bspNet";
import type { after, afterEach, before, beforeEach, it } from "node:test";
import type { launchFullNetwork } from "./helpers";

/**
 * Context object provided to test suites for interacting with the BSP network.
 * Contains utility functions and configuration for setting up and manipulating the test environment.
 */
export type FullNetContext = {
  /**
   * Test runner's wrapped 'it' function for defining individual test cases.
   */
  it: typeof it;

  /**
   * Creates and returns a connected API instance for a user node.
   * @returns A promise that resolves to an enriched api instance for user operations.
   */
  createUserApi: () => ReturnType<typeof BspNetTestApi.create>;

  /**
   * Creates and returns a connected API instance for a BSP node.
   * @returns A promise that resolves to an enriched api instance for BSP operations.
   */
  createBspApi: () => ReturnType<typeof BspNetTestApi.create>;

  /**
   * Creates and returns a connected API instance for a MSP node.
   * @returns A promise that resolves to an enriched api instance for MSP operations.
   */
  createMspApi: () => ReturnType<typeof BspNetTestApi.create> | undefined;

  /**
   * Creates and returns a connected API instance for a BSP node.
   * @returns A promise that resolves to  an enriched api instance for BSP operations.
   */
  createApi: (
    endpoint: `ws://${string}` | `wss://${string}`
  ) => ReturnType<typeof BspNetTestApi.create>;

  /**
   * The current configuration of the BSP network for this test run.
   */
  bspNetConfig: BspNetConfig;

  /**
   * Before hook for test setup operations.
   */
  before: typeof before;

  /**
   * After hook for test cleanup operations.
   */
  after: typeof after;

  beforeEach: typeof beforeEach;

  afterEach: typeof afterEach;

  /**
   * Retrieves the response from launching the network.
   * @returns The result of the launchFullNetwork function, which may include network details or initialization data (for multiInitialised network only).
   */
  getLaunchResponse: () => ReturnType<typeof launchFullNetwork>;
};

/**
 * Represents the initial state of the network after initialisation.
 */
export type Initialised = {
  /** A list of bucket IDs created during network initialisation */
  bucketIds: H256[];
};
