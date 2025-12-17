import type { after, afterEach, before, beforeEach, it } from "node:test";
import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { Address, Event, EventRecord } from "@polkadot/types/interfaces";
import type { Codec, IEventData, ISubmittableResult } from "@polkadot/types/types";
import type { HexString } from "@polkadot/util/types";
import type postgres from "postgres";
import type { NetworkLauncher } from "../netLaunch";
import type { BspNetTestApi } from "./test-api";

// biome-ignore lint/complexity/noBannedTypes: Good enough until we integrate ORM
export type SqlClient = postgres.Sql<{}>;

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export interface BspNetApi extends ApiPromise {
  /**
   * Runtime-aware accounts namespace.
   * Access test accounts via api.accounts.sudo, api.accounts.bspKey, etc.
   */
  accounts: {
    sudo: KeyringPair;
    bspKey: KeyringPair;
    bspDownKey: KeyringPair;
    bspTwoKey: KeyringPair;
    bspThreeKey: KeyringPair;
    mspKey: KeyringPair;
    mspDownKey: KeyringPair;
    mspTwoKey: KeyringPair;
    mspThreeKey: KeyringPair;
    shUser: KeyringPair;
  };
  /**
   * @description Creates a new bucket and submits a new storage request.
   *
   * @param source - The local path to the file to be uploaded.
   * @param location - The StorageHub "location" field of the file to be uploaded.
   * @param bucketName - The name of the bucket to be created.
   * @returns A promise that resolves to file metadata.
   */
  createBucketAndSendNewStorageRequest(
    source: string,
    location: string,
    bucketName: string,
    valuePropId?: HexString
  ): Promise<FileMetadata>;

  /**
   * Creates a new bucket.
   *
   * @param bucketName - The name of the bucket to be created.
   * @returns A promise that resolves to a new bucket event.
   */
  createBucket(bucketName: string, valuePropId?: HexString): Promise<Event>;

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

  /**
   * Pending transactions DB helpers namespace.
   * Provides convenience helpers to query/assert the pending transactions Postgres database.
   */
  pendingDb: {
    /**
     * Creates and returns a SQL client connected to the pending transactions DB.
     */
    createClient: () => SqlClient;
    /**
     * Converts an ss58 address to AccountId bytes for DB queries.
     */
    accountIdFromAddress: (address: string) => Buffer;
    /**
     * Returns the row for (accountId, nonce) if it exists.
     */
    getByNonce: (options: { sql: SqlClient; accountId: Buffer; nonce: bigint }) => Promise<any>;
    /**
     * Returns all rows for an account ordered by nonce.
     */
    getAllByAccount: (options: { sql: SqlClient; accountId: Buffer }) => Promise<any[]>;
    /**
     * Counts active-state rows for an account.
     */
    countActive: (options: { sql: SqlClient; accountId: Buffer }) => Promise<bigint>;
    /**
     * Waits until a given nonce reaches the provided state.
     */
    waitForState: (options: {
      sql: SqlClient;
      accountId: Buffer;
      nonce: bigint;
      state: string;
      timeoutMs?: number;
      pollMs?: number;
    }) => Promise<void>;
    /**
     * Asserts there are no active rows with nonce < onChainNonce.
     */
    expectClearedBelow: (options: {
      sql: SqlClient;
      accountId: Buffer;
      onChainNonce: bigint;
    }) => Promise<void>;
  };

  /**
   * Prometheus operations namespace.
   * Provides methods for querying and asserting Prometheus metrics.
   */
  prometheus: {
    /**
     * Query the Prometheus API with a PromQL query.
     */
    query: (query: string) => Promise<{
      status: string;
      data: {
        resultType: string;
        result: Array<{
          metric: Record<string, string>;
          value?: [number, string];
          values?: Array<[number, string]>;
        }>;
      };
    }>;
    /**
     * Get the current value of a metric from Prometheus.
     */
    getMetricValue: (query: string) => Promise<number>;
    /**
     * Get the targets that Prometheus is currently scraping.
     */
    getTargets: () => Promise<{
      status: string;
      data: {
        activeTargets: Array<{
          labels: Record<string, string>;
          scrapeUrl: string;
          health: string;
          lastScrape: string;
        }>;
      };
    }>;
    /**
     * Wait for Prometheus to scrape updated metrics.
     */
    waitForScrape: () => Promise<void>;
    /**
     * Default Prometheus URL for tests.
     */
    url: string;
  };
}

/**
 * Represents information about a network toxicity.
 * This interface is used to describe a Toxic "debuff" that can be applied to a running toxiproxy.
 *
 * @interface
 * @property {("latency"|"down"|"bandwidth"|"slow_close"|"timeout"|"reset_peer"|"slicer"|"limit_data")} type - The type of network toxic.
 * @property {string} name - The name of the network toxic.
 * @property {("upstream"|"downstream")} stream - The link direction of the network toxic.
 * @property {number} toxicity - The probability of the toxic being applied to a link (defaults to 1.0, 100%)
 * @property {Object} attributes - A map of toxic-specific attributes
 */
export interface ToxicInfo {
  type:
    | "latency"
    | "down"
    | "bandwidth"
    | "slow_close"
    | "timeout"
    | "reset_peer"
    | "slicer"
    | "limit_data";
  name: string;
  stream: "upstream" | "downstream";
  toxicity: number;
  attributes: {
    [key: string]: string | number | undefined;
  };
}

/**
 * Represents the metadata of a file.
 *
 * @interface
 * @property {string} fileKey - The file key of the stored file.
 * @property {string} bucketId - The bucket ID registered of the file.
 * @property {string} location - The remote location of the file.
 * @property {string} owner - The owner of the file.
 * @property {string} fingerprint - The generated fingerprint of the file.
 * @property {number} fileSize - The size of the file in bytes.
 */
export interface FileMetadata {
  /**The file key of the stored file. */
  fileKey: string;
  /**The bucket ID registered of the file. */
  bucketId: string;
  /**The remote location of the file. */
  location: string;
  /**The owner of the file. */
  owner: string;
  /**The generated fingerprint of the file. */
  fingerprint: string;
  /**The size of the file in bytes. */
  fileSize: number;
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

  /**
   * If true, runs launched userNode has attached indexer service enabled.
   */
  indexer?: boolean;

  /**
   * Optional parameter to set the indexer mode when indexer is enabled.
   * 'full' - indexes all events (default)
   * 'lite' - indexes only essential events as defined in LITE_MODE_EVENTS.md
   * 'fishing' - indexes only events related to fishing (fisherman service)
   */
  indexerMode?: "full" | "lite" | "fishing";

  /**
   * If true, runs fisherman service.
   */
  fisherman?: boolean;

  /**
   * Optional parameter to run the backend service.
   * Requires indexer to be enabled.
   */
  backend?: boolean;

  /**
   * If true, runs indexer as standalone service instead of embedded in user node (fullnet only).
   */
  standaloneIndexer?: boolean;

  /**
   * Maximum number of incomplete storage requests to process during initial sync.
   * Must be at least 1.
   */
  fishermanIncompleteSyncMax?: number;

  /**
   * Page size for incomplete storage request pagination.
   * Must be at least 1.
   */
  fishermanIncompleteSyncPageSize?: number;

  /**
   * Optional parameter to set the Rust log level for all nodes.
   * Defaults to 'info' if not specified.
   */
  logLevel?: string;

  /**
   * If true, runs Prometheus server for metrics collection.
   */
  telemetry?: boolean;
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
   * @return The result of creating the network, which may include network details or initialization data.
   */
  getLaunchResponse: () => ReturnType<typeof NetworkLauncher.create>;
};

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
   * Creates and returns a connected API instance for the first MSP node.
   * @returns A promise that resolves to an enriched api instance for MSP operations.
   */
  createMsp1Api: () => ReturnType<typeof BspNetTestApi.create> | undefined;

  /**
   * Creates and returns a connected API instance for the second MSP node.
   * @returns A promise that resolves to an enriched api instance for MSP operations.
   */
  createMsp2Api: () => ReturnType<typeof BspNetTestApi.create> | undefined;

  /**
   * Creates and returns a connected API instance for the fisherman node.
   * Only available when fisherman is enabled in test options.
   * @returns A promise that resolves to an enriched api instance for fisherman operations.
   */
  createFishermanApi?: () => ReturnType<typeof BspNetTestApi.create>;

  /**
   * Creates and returns a connected API instance for the indexer node.
   * Only available when standalone indexer is enabled in test options.
   * @returns A promise that resolves to an enriched api instance for indexer operations.
   */
  createIndexerApi?: () => ReturnType<typeof BspNetTestApi.create>;

  /**
   * Creates and returns a connected API instance for a BSP node.
   * @returns A promise that resolves to  an enriched api instance for BSP operations.
   */
  createApi: (
    endpoint: `ws://${string}` | `wss://${string}`
  ) => ReturnType<typeof BspNetTestApi.create>;

  /**
   * Creates and returns a sql client connected to the local postgres database.
   * @returns A sql client instance for interacting with the indexer db.
   */
  createSqlClient: () => SqlClient;

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
   * @returns The result of creating the network, which may include network details or initialization data.
   */
  getLaunchResponse: () => ReturnType<typeof NetworkLauncher.create>;
};

/**
 * Represents the initial state of the network after initialisation.
 */
export type Initialised = {
  /** The metadata of the initial file stored in the network */
  fileMetadata: FileMetadata;
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
  /**
   * Generate and upload a large file (in GB) for performance testing.
   * File stored in docker/tmp/ and cleaned up after test.
   */
  big_file?: number;
  /** Set a custom capacity for the BSP */
  capacity?: bigint;
  /** Set a custom BSP weight */
  bspStartingWeight?: bigint;
  /** Custom toxics to apply to the network */
  toxics?: ToxicInfo[];
  /** Set a custom timeout interval for submit extrinsic retries, in seconds */
  extrinsicRetryTimeout?: number;
  /** If true, runs launched userNode has attached indexer service enabled. */
  indexer?: boolean;
  /**
   * Optional parameter to set the indexer mode when indexer is enabled.
   * 'full' - indexes all events (default)
   * 'lite' - indexes only essential events as defined in LITE_MODE_EVENTS.md
   */
  indexerMode?: "full" | "lite" | "fishing";
  /** If true, runs indexer as standalone service instead of embedded in user node (fullnet only) */
  standaloneIndexer?: boolean;
  /** If true, runs fisherman service */
  fisherman?: boolean;
  /** If true, runs backend service */
  backend?: boolean;
  /** If true, enable Pending Transactions Postgres DB for MSP 1 during tests (fullnet only) */
  pendingTxDb?: boolean;
  /**
   * Set the runtime type to use
   * 'parachain' - Polkadot parachain runtime (default)
   * 'solochain' - Solochain EVM runtime
   */
  runtimeType?: "parachain" | "solochain";
  /**
   * Maximum number of incomplete storage requests to process during initial sync.
   * Must be at least 1.
   */
  fishermanIncompleteSyncMax?: number;
  /**
   * Page size for incomplete storage request pagination.
   * Must be at least 1.
   */
  fishermanIncompleteSyncPageSize?: number;
  /**
   * Optional parameter to set the Rust log level for all nodes.
   * Defaults to 'info' if not specified.
   */
  logLevel?: string;
  /** If true, runs Prometheus server for metrics collection */
  telemetry?: boolean;
};

/**
 * Represents the configuration and metadata for an initialised multi-BSP network.
 * This type is used to store information about additional BSPs in the network and
 * the initial file data stored in the network.
 *
 * @property {number} bspTwoRpcPort - The RPC port number for the second BSP node.
 * @property {number} bspThreeRpcPort - The RPC port number for the third BSP node.
 * @property {FileMetadata} fileMetadata - Metadata of the initial file stored in the network.
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
  fileMetadata: FileMetadata;
};

/**
 * Options for creating a block in the chain.
 */
export type SealBlockOptions = {
  /**
   * Optional extrinsic(s) to include in the sealed block.
   * Can be a single extrinsic or an array of extrinsics.
   */
  calls?:
    | SubmittableExtrinsic<"promise", ISubmittableResult>
    | SubmittableExtrinsic<"promise", ISubmittableResult>[];

  /**
   * Optional signer for the extrinsics.
   * If not provided, a default signer (usually 'alice') will be used.
   */
  signer?: KeyringPair;

  /**
   * Optional nonce for the extrinsics.
   * If not provided, the next nonce will be used.
   */
  nonce?: number;

  /**
   * Optional parent hash for the block.
   * If not provided, the current block hash will be used.
   */
  parentHash?: string;

  /**
   * Whether to finalize the block after sealing.
   * Defaults to true if not specified.
   */
  finaliseBlock?: boolean;

  /**
   * Whether to fail the block if extrinsic is not included.
   * Defaults to true if not specified.
   */
  failOnExtrinsicNonInclusion?: boolean;
};

/**
 * Options for the BSP Stored waiting utility function
 */
export type BspStoredOptions = {
  /**
   * The number of expected extrinsics.
   */
  expectedExts?: number;

  /**
   * The BSP Account ID that may be sending submit proof extrinsics.
   */
  bspAccount?: Address;

  /**
   * The timeout in milliseconds for the wait.
   */
  timeoutMs?: number;

  /**
   * Whether to seal a block after waiting for the transaction.
   * Defaults to true if not specified.
   */
  sealBlock?: boolean;

  /**
   * Whether to finalize the block after sealing.
   * Defaults to true if not specified.
   */
  finalizeBlock?: boolean;
};
