import "@storagehub/api-augment";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { HexString } from "@polkadot/util/types";
import { types as BundledTypes } from "@storagehub/types-bundle";
import type { AssertExtrinsicOptions } from "../../asserts";
import * as Assertions from "../../asserts";
import * as BspNetBlock from "../block";
import { sealBlock } from "../block";
import * as ShConsts from "../consts";
import * as DockerBspNet from "../docker";
import * as Files from "../fileHelpers";
import * as NodeBspNet from "../node";
import type { BspNetApi, SealBlockOptions } from "./types";
import * as Waits from "../waits";
import { addBsp } from "../helpers";

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export class ShTestApi implements AsyncDisposable {
  private _api: ApiPromise;
  private _endpoint: `ws://${string}` | `wss://${string}`;

  private constructor(api: ApiPromise, endpoint: `ws://${string}` | `wss://${string}`) {
    this._api = api;
    this._endpoint = endpoint;
  }

  /**
   * Creates a new instance of ShTestApi.
   *
   * @param endpoint - The WebSocket endpoint to connect to.
   * @returns A promise that resolves to an enriched BspNetApi.
   */
  public static async create(endpoint: `ws://${string}` | `wss://${string}`) {
    const api = await ShTestApi.connect(endpoint);
    await api.isReady;

    const ctx = new ShTestApi(api, endpoint);

    return ctx.enrichApi();
  }

  public async reconnect(): Promise<void> {
    if (!this._api.isConnected) {
      await this._api.disconnect();
      const newApi = await ApiPromise.create({
        provider: new WsProvider(this._endpoint),
        noInitWarn: true,
        throwOnConnect: false,
        throwOnUnknown: false,
        typesBundle: BundledTypes
      });
      await newApi.isReady;
      this._api = newApi;
      this.enrichApi();
    }
  }

  /**
   * Establishes a connection to the specified endpoint.
   * Note: This method shouldn't be called directly in tests. Use `create` instead.
   *
   * @param endpoint - The WebSocket endpoint to connect to.
   * @returns A promise that resolves to an ApiPromise with async disposal.
   */
  public static async connect(endpoint: `ws://${string}` | `wss://${string}`) {
    const api = await ApiPromise.create({
      provider: new WsProvider(endpoint),
      noInitWarn: true,
      throwOnConnect: false,
      throwOnUnknown: false,
      typesBundle: BundledTypes
    });
    return Object.assign(api, {
      [Symbol.asyncDispose]: async () => {
        await api.disconnect();
      }
    });
  }

  private async disconnect() {
    await this._api.disconnect();
  }

  /**
   * Seals a block with optional extrinsics and finalizes it.
   *
   * @param calls - Optional extrinsic(s) to include in the block.
   * @param signer - Optional signer for the extrinsics.
   * @param finaliseBlock - Whether to finalize the block. Defaults to true.
   * @returns A Promise resolving to a SealedBlock object.
   */
  private async sealBlock(
    calls?:
      | SubmittableExtrinsic<"promise", ISubmittableResult>
      | SubmittableExtrinsic<"promise", ISubmittableResult>[],
    signer?: KeyringPair,
    finaliseBlock = true
  ) {
    return sealBlock(this._api, calls, signer, finaliseBlock);
  }

  private async sendNewStorageRequest(source: string, location: string, bucketName: string) {
    return Files.sendNewStorageRequest(this._api, source, location, bucketName);
  }

  private async createBucket(bucketName: string) {
    return Files.createBucket(this._api, bucketName);
  }

  private assertEvent(module: string, method: string, events?: EventRecord[]) {
    return Assertions.assertEventPresent(this._api, module, method, events);
  }

  /**
   * Advances the blockchain to a specified block number.
   *
   * This function seals blocks until the specified block number is reached. It can optionally
   * wait between blocks and watch for BSP proofs.
   *
   * @param api - The ApiPromise instance to interact with the blockchain.
   * @param blockNumber - The target block number to advance to.
   * @param waitBetweenBlocks - Optional. If specified:
   *                            - If a number, waits for that many milliseconds between blocks.
   *                            - If true, waits for 500ms between blocks.
   *                            - If false or undefined, doesn't wait between blocks.
   * @param watchForBspProofs - Optional. An array of BSP IDs to watch for proofs.
   *                            If specified, the function will wait for BSP proofs at appropriate intervals.
   *
   * @returns A Promise that resolves to a SealedBlock object representing the last sealed block.
   *
   * @throws Will throw an error if the target block number is lower than the current block number.
   *
   * @example
   * // Advance to block 100 with no waiting
   * const result = await advanceToBlock(api, 100);
   *
   * @example
   * // Advance to block 200, waiting 1000ms between blocks
   * const result = await advanceToBlock(api, 200, 1000);
   *
   * @example
   * // Advance to block 300, watching for proofs from two BSPs
   * const result = await advanceToBlock(api, 300, true, ['bsp1', 'bsp2']);
   */
  private advanceToBlock(
    blockNumber: number,
    options?: {
      waitBetweenBlocks?: number | boolean;
      waitForBspProofs?: string[];
    }
  ) {
    return BspNetBlock.advanceToBlock(
      this._api,
      blockNumber,
      options?.waitBetweenBlocks,
      options?.waitForBspProofs
    );
  }

  private enrichApi() {
    const remappedAssertNs = {
      fetchEventData: Assertions.fetchEventData,

      /**
       * Asserts that a specific event is present in the given events or the latest block.
       * @param module - The module name of the event.
       * @param method - The method name of the event.
       * @param events - Optional. The events to search through. If not provided, it will fetch the latest block's events.
       * @returns The matching event and its data.
       */
      eventPresent: async (module: string, method: string, events?: EventRecord[]) => {
        const evts = events ?? ((await this._api.query.system.events()) as EventRecord[]);
        return Assertions.assertEventPresent(this._api, module, method, evts);
      },
      /**
       * Asserts that multiple instances of a specific event are present.
       * @param module - The module name of the event.
       * @param method - The method name of the event.
       * @param events - Optional. The events to search through. If not provided, it will fetch the latest block's events.
       * @returns An array of matching events and their data.
       */
      eventMany: async (module: string, method: string, events?: EventRecord[]) => {
        const evts = events ?? ((await this._api.query.system.events()) as EventRecord[]);
        return Assertions.assertEventMany(this._api, module, method, evts);
      },
      /**
       * Asserts that a specific extrinsic is present in the transaction pool or recent blocks.
       * @param options - Options specifying the extrinsic to search for.
       * @returns An array of matching extrinsics.
       */
      extrinsicPresent: (options: AssertExtrinsicOptions) =>
        Assertions.assertExtrinsicPresent(this._api, options),
      /**
       * Asserts that a specific provider has been slashed.
       * @param providerId - The ID of the provider to check.
       * @returns A boolean indicating whether the provider was slashed.
       */
      providerSlashed: (providerId: string) =>
        Assertions.checkProviderWasSlashed(this._api, providerId),

      /**
       * Asserts that a specific log message appears in a Docker container's output.
       * @param options - The options for the log assertion.
       * @param options.searchString - The string to search for in the container's logs.
       * @param options.containerName - The name of the Docker container to search logs in.
       * @param options.timeout - Optional. The maximum time (in milliseconds) to wait for the log message to appear. Default 10s.
       * @returns A promise that resolves to the matching log message if found, or rejects if the timeout is reached.
       */
      log: async (options: {
        searchString: string;
        containerName: string;
        timeout?: number;
      }) => {
        return Assertions.assertDockerLog(
          options.containerName,
          options.searchString,
          options.timeout
        );
      }
    };

    /**
     * Waits namespace
     * Contains methods for waiting on specific events or conditions in the BSP network.
     */
    const remappedWaitsNs = {
      /**
       * Waits for a BSP to volunteer for a storage request.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a BSP has volunteered.
       */
      bspVolunteer: (expectedExts?: number) => Waits.waitForBspVolunteer(this._api, expectedExts),

      /**
       * Waits for a BSP to confirm storing a file.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a BSP has confirmed storing a file.
       */
      bspStored: (expectedExts?: number) => Waits.waitForBspStored(this._api, expectedExts),

      /**
       * Waits for a MSP to respond to storage requests.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a MSP has responded to storage requests.
       */
      mspResponse: (expectedExts?: number) => Waits.waitForMspResponse(this._api, expectedExts)
    };

    /**
     * File operations namespace
     * Contains methods for interacting with StorageHub file system.
     */
    const remappedFileNs = {
      /**
       * Creates a new bucket.
       *
       * @param bucketName - The name of the bucket to be created.
       * @param mspId - <TODO> Optional MSP ID to use for the new storage request. Defaults to DUMMY_MSP_ID.
       * @param owner - Optional signer with which to issue the newStorageRequest Defaults to SH_USER.
       * @returns A promise that resolves to a new bucket event.
       */
      newBucket: (bucketName: string, owner?: KeyringPair) =>
        Files.createBucket(this._api, bucketName, undefined, owner),

      /**
       * Creates a new bucket and submits a new storage request.
       *
       * @param source - The local path to the file to be uploaded.
       * @param location - The StorageHub "location" field of the file to be uploaded.
       * @param bucketName - The name of the bucket to be created.
       * @param mspId - <TODO> Optional MSP ID to use for the new storage request. Defaults to DUMMY_MSP_ID.
       * @param owner - Optional signer with which to issue the newStorageRequest Defaults to SH_USER.
       * @returns A promise that resolves to file metadata.
       */
      newStorageRequest: (
        source: string,
        location: string,
        bucketName: string,
        msp_id?: HexString,
        owner?: KeyringPair
      ) => Files.sendNewStorageRequest(this._api, source, location, bucketName, msp_id, owner)
    };

    /**
     * Block operations namespace
     * Contains methods for manipulating and interacting with blocks in the BSP network.
     */
    const remappedBlockNs = {
      /**
       * Seals a block with optional extrinsics.
       * @param options - Options for sealing the block, including calls, signer, and whether to finalize.
       * @returns A promise that resolves to a SealedBlock object.
       */
      seal: (options?: SealBlockOptions) =>
        BspNetBlock.sealBlock(this._api, options?.calls, options?.signer, options?.finaliseBlock),
      /**
       * Seal blocks until the next challenge period block.
       * It will verify that the SlashableProvider event is emitted and check if the provider is slashable with an additional failed challenge deadline.
       * @param nextChallengeTick - The block number of the next challenge.
       * @param provider - The provider to check for slashing.
       * @returns A promise that resolves when the challenge period block is reached.
       */
      skipToChallengePeriod: (nextChallengeTick: number, provider: string) =>
        BspNetBlock.runToNextChallengePeriodBlock(this._api, nextChallengeTick, provider),
      /**
       * Skips a specified number of blocks.
       * Note: This skips too quickly for nodes to BSPs to react. Use skipTo where reaction extrinsics are required.
       * @param blocksToAdvance - The number of blocks to skip.
       * @returns A promise that resolves when the specified number of blocks have been skipped.
       */
      skip: (blocksToAdvance: number) => BspNetBlock.skipBlocks(this._api, blocksToAdvance),
      /**
       * Advances the chain to a specific block number.
       * @param blockNumber - The target block number to advance to.
       * @param options - Optional parameters for waiting between blocks and watching for BSP proofs.
       * @returns A promise that resolves when the specified block number is reached.
       */
      skipTo: (
        blockNumber: number,
        options?: {
          waitBetweenBlocks?: number | boolean;
          waitForBspProofs?: string[];
          spam?: boolean;
          verbose?: boolean;
        }
      ) =>
        BspNetBlock.advanceToBlock(
          this._api,
          blockNumber,
          options?.waitBetweenBlocks,
          options?.waitForBspProofs,
          options?.spam,
          options?.verbose
        ),
      /**
       * Skips blocks until the minimum time for capacity changes is reached.
       * @returns A promise that resolves when the minimum change time is reached.
       */
      skipToMinChangeTime: () => BspNetBlock.skipBlocksToMinChangeTime(this._api),
      /**
       * Causes a chain re-org by creating a finalized block on top of the parent block.
       * Note: This requires the head block to be unfinalized, otherwise it will throw!
       * @returns A promise that resolves when the chain re-org is complete.
       */
      reOrg: () => BspNetBlock.reOrgBlocks(this._api)
    };

    const remappedNodeNs = {
      /**
       * Drops transaction(s) from the node's transaction pool.
       *
       * @param extrinsic - Optional. Specifies which transaction(s) to drop:
       *                    - If omitted, all transactions in the pool will be cleared.
       *                    - If an object with module and method, it will drop matching transactions.
       *                    - If a hex string, it will drop the transaction with the matching hash.
       * @param sealAfter - Whether to seal a block after dropping the transaction(s). Defaults to false.
       */
      dropTxn: (extrinsic?: { module: string; method: string } | HexString, sealAfter = false) =>
        NodeBspNet.dropTransaction(this._api, extrinsic, sealAfter)
    };

    const remappedDockerNs = {
      ...DockerBspNet,
      onboardBsp: (options: {
        bspSigner: KeyringPair;
        name?: string;
        rocksdb?: boolean;
        bspKeySeed?: string;
        bspId?: string;
        bspStartingWeight?: bigint;
        maxStorageCapacity?: number;
        additionalArgs?: string[];
      }) => addBsp(this._api, options.bspSigner, options)
    };

    return Object.assign(this._api, {
      /**
       * Soon Deprecated. Use api.block.seal() instead.
       * @see {@link sealBlock}
       */
      sealBlock: this.sealBlock.bind(this),
      /**
       * Soon Deprecated. Use api.file.newStorageRequest() instead.
       * @see {@link sendNewStorageRequest}
       */
      sendNewStorageRequest: this.sendNewStorageRequest.bind(this),
      /**
       * Soon Deprecated. Use api.file.newBucket() instead.
       * @see {@link createBucket}
       */
      createBucket: this.createBucket.bind(this),
      /**
       * Soon Deprecated. Use api.assert.eventPresent() instead.
       * @see {@link assertEvent}
       */
      assertEvent: this.assertEvent.bind(this),
      /**
       * Soon Deprecated. Use api.assert.eventPresent() instead.
       * @see {@link advanceToBlock}
       */
      advanceToBlock: this.advanceToBlock.bind(this),
      /**
       * Assertions namespace
       * Provides methods for asserting various conditions in the BSP network tests.
       */
      assert: remappedAssertNs,
      /**
       * Waits namespace
       * Contains methods for waiting on specific events or conditions in the BSP network.
       */
      wait: remappedWaitsNs,
      /**
       * File operations namespace
       * Offers methods for file-related operations in the BSP network, such as creating buckets and storage requests.
       */
      file: remappedFileNs,
      /**
       * Node operations namespace
       * Provides methods for interacting with and manipulating nodes in the BSP network.
       */
      node: remappedNodeNs,
      /**
       * Block operations namespace
       * Contains methods for manipulating and interacting with blocks in the BSP network.
       */
      block: remappedBlockNs,
      /**
       * StorageHub Constants  namespace
       * Contains static data useful for testing the BSP network.
       */
      shConsts: ShConsts,
      /**
       * Docker operations namespace
       * Offers methods for interacting with Docker containers in the BSP network test environment.
       */
      docker: remappedDockerNs,
      [Symbol.asyncDispose]: this.disconnect.bind(this)
    }) satisfies BspNetApi;
  }

  async [Symbol.asyncDispose]() {
    await this._api.disconnect();
  }
}

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 * This type extends the standard Polkadot API with additional methods and namespaces
 * specifically designed for testing and interacting with a StorageHub BSP network.
 *
 * It includes:
 * - Extended assertion capabilities (@see {@link Assertions})
 * - Waiting utilities for BSP-specific events (@see {@link Waits})
 * - File and bucket operations (@see {@link Files})
 * - Block manipulation and advancement utilities (@see {@link BspNetBlock})
 * - Node interaction methods (@see {@link NodeBspNet})
 * - Docker container management for BSP testing (@see {@link DockerBspNet})
 * - StorageHub constants (@see {@link ShConsts})
 *
 * This API is created using the ShTestApi.create() static method and provides
 * a comprehensive toolkit for testing and developing BSP network functionality.
 */
export type EnrichedBspApi = Awaited<ReturnType<typeof ShTestApi.create>>;
