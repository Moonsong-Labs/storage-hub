import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { Codec, IEventData, ISubmittableResult } from "@polkadot/types/types";
import type { EventRecord, Event } from "@polkadot/types/interfaces";
import type { after, afterEach, before, beforeEach, it } from "node:test";
import type { launchNetwork } from "./testrunner";
import type { BspNetTestApi } from "../network/test-api";
import type { SealedBlock } from "../network/block";
import type { FileMetadata, ToxicInfo } from "../network";

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export interface BspNetApi extends ApiPromise {
  /**
   * Seals a block optionally with a given extrinsic and signer.
   *
   * @param call - The extrinsic to be included in the block.
   * @param signer - The keyring pair used to sign the block.
   * @returns A promise that resolves to a sealed block.
   */
  sealBlock(
    calls?:
      | SubmittableExtrinsic<"promise", ISubmittableResult>
      | SubmittableExtrinsic<"promise", ISubmittableResult>[],
    signer?: KeyringPair
  ): Promise<SealedBlock>;

  /**
   * @description Advances the block number to the given block number.
   *
   * @param blockNumber - The block number to advance to.
   * @param waitBetweenBlocks - Whether to wait between blocks. Defaults to false. Can also be set to a number to wait that many milliseconds between blocks.
   * @returns A promise that resolves when the block number is advanced.
   */
  advanceToBlock: (
    blockNumber: number,
    options?: {
      waitBetweenBlocks?: number | boolean;
      waitForBspProofs?: string[];
    }
  ) => Promise<SealedBlock>;

  /**
   * @description Creates a new bucket and submits a new storage request.
   *
   * @param source - The local path to the file to be uploaded.
   * @param location - The StorageHub "location" field of the file to be uploaded.
   * @param bucketName - The name of the bucket to be created.
   * @returns A promise that resolves to file metadata.
   */
  sendNewStorageRequest(
    source: string,
    location: string,
    bucketName: string
  ): Promise<FileMetadata>;

  /**
   * Creates a new bucket.
   *
   * @param bucketName - The name of the bucket to be created.
   * @returns A promise that resolves to a new bucket event.
   */
  createBucket(bucketName: string): Promise<Event>;

  /**
   * Asserts that a specific event occurred in a list of events.
   *
   * @param module - The module where the event originated.
   * @param method - The method that triggered the event.
   * @param events - The list of event records to search through.
   * @returns An object containing the event and its data.
   */
  assertEvent(
    module: string,
    method: string,
    events?: EventRecord[]
  ): { event: Event; data: Codec[] & IEventData };
}

/**
 * Configuration options for the BSP network.
 * These settings determine the behavior and characteristics of the network during tests.
 */
export type BspNetConfig = {
  /**
   * If true, simulates a noisy network environment with added latency and bandwidth limitations.
   * Useful for testing network resilience and performance under suboptimal conditions.
   */
  noisy: boolean;

  /**
   * If true, uses RocksDB as the storage backend instead of the default in-memory database.
   */
  rocksdb: boolean;

  /**
   * Optional parameter to set the storage capacity of the BSP.
   * Measured in bytes.
   */
  capacity?: bigint;

  /**
   * Optional parameter to set the timeout interval for submit extrinsic retries.
   */
  extrinsicRetryTimeout?: number;

  /**
   * Optional parameter to set the weight of the BSP.
   * Measured in bytes.
   */
  bspStartingWeight?: bigint;

  /**
   * Optional parameter to define what toxics to apply to the network.
   * Only applies when `noisy` is set to true.
   */
  toxics?: ToxicInfo[];
};

/**
 * Context object provided to test suites for interacting with the BSP network.
 * Contains utility functions and configuration for setting up and manipulating the test environment.
 */
export type BspNetContext = {
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
   * @returns The result of the launchNetwork function, which may include network details or initialization data (for multiInitialised network only).
   */
  getLaunchResponse: () => ReturnType<typeof launchNetwork>;
};

/**
 * Network configuration options for BspNet tests.
 */
export type NetworkConfig =
  /** Uses default configuration with a single BSP and no network noise */
  | "standard"
  /** Runs tests with multiple configurations, including both RocksDB and MemoryDB */
  | "all"
  /** Simulates a noisy network environment with added latency and bandwidth limitations */
  | "noisy"
  /** Custom network configuration */
  | BspNetConfig[];

/**
 * Options for configuring BspNet test runs.
 * These options allow fine-tuning of test behavior and network configuration.
 */
export type TestOptions = {
  /** If true, keeps the network alive after tests complete */
  keepAlive?: boolean;
  /** If true, skips the test suite */
  skip?: boolean;
  /** If true, runs only this test suite */
  only?: boolean;
  /** Sets a custom timeout for the test suite */
  timeout?: number;
  /** Specifies the network configuration to use */
  networkConfig?: NetworkConfig;
  /**
   * Determines the initial state of the network:
   * - false: Network starts with MSP & BSP already enrolled
   * - true: Network starts with MSP & BSP already enrolled and sample file already stored
   * - "multi": Runs tests with both initialised and non-initialised network configurations
   */
  initialised?: boolean | "multi";
  /** Set a custom capacity for the BSP */
  capacity?: bigint;
  /** Set a custom BSP weight */
  bspStartingWeight?: bigint;
  /** Custom toxics to apply to the network */
  toxics?: ToxicInfo[];
  /** Set a custom timeout interval for submit extrinsic retries */
  extrinsicRetryTimeout?: number;
};

/**
 * Represents the configuration and metadata for an initialised multi-BSP network.
 * This type is used to store information about additional BSPs in the network and
 * the initial file data stored in the network.
 *
 * @property {number} bspTwoRpcPort - The RPC port number for the second BSP node.
 * @property {number} bspThreeRpcPort - The RPC port number for the third BSP node.
 * @property {FileMetadata} fileData - Metadata of the initial file stored in the network.
 */
export type InitialisedMultiBspNetwork = {
  /**
   * The RPC port number for the second BSP node.
   */
  bspTwoRpcPort: number;
  /**
   * The RPC port number for the third BSP node.
   */
  bspThreeRpcPort: number;
  /**
   * @see FileMetadata for details on the file metadata structure.
   */
  fileData: FileMetadata;
};


