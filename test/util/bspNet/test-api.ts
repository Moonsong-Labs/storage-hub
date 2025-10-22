import "@storagehub/api-augment"; // must be first import

import { ApiPromise, WsProvider } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord, H256 } from "@polkadot/types/interfaces";
import type { HexString } from "@polkadot/util/types";
import { types as BundledTypes } from "@storagehub/types-bundle";
import type { AssertExtrinsicOptions } from "../asserts";
import * as Assertions from "../asserts";
import {
  alith,
  ethBspDownKey,
  ethBspKey,
  ethBspThreeKey,
  ethBspTwoKey,
  ethMspDownKey,
  ethMspKey,
  ethMspThreeKey,
  ethMspTwoKey,
  ethShUser
} from "../evmNet/keyring";
import {
  alice,
  bspDownKey,
  bspKey,
  bspThreeKey,
  bspTwoKey,
  mspDownKey,
  mspKey,
  mspThreeKey,
  mspTwoKey,
  shUser
} from "../pjsKeyring";
import * as BspNetBlock from "./block";
import * as ShConsts from "./consts";
import * as DockerBspNet from "./docker";
import * as Files from "./fileHelpers";
import { addBsp } from "./helpers";
import * as NodeBspNet from "./node";
import type { BspNetApi, BspStoredOptions, SealBlockOptions } from "./types";
import * as Waits from "./waits";

/**
 * Options for the waitForTxInPool method.
 * @param module - The module name of the event.
 * @param method - The method name of the event.
 * @param checkQuantity - Optional. The number of expected extrinsics.
 * @param strictQuantity - Optional. Whether to strictly check the quantity of extrinsics.
 * @param shouldSeal - Optional. Whether to seal a block after waiting for the transaction.
 * @param finalizeBlock - Optional. Whether to finalize a block after waiting for the transaction.
 * @param expectedEvent - Optional. The expected event to wait for.
 * @param iterations - Optional. The number of iterations to wait for the transaction.
 * @param delay - Optional. The delay between iterations.
 * @param timeout - Optional. The timeout for the wait.
 */
export interface WaitForTxOptions {
  module: string;
  method: string;
  checkQuantity?: number;
  strictQuantity?: boolean;
  shouldSeal?: boolean;
  finalizeBlock?: boolean;
  expectedEvent?: string;
  timeout?: number;
  verbose?: boolean;
}

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export class BspNetTestApi implements AsyncDisposable {
  private _api: ApiPromise;
  private _endpoint: `ws://${string}` | `wss://${string}`;
  private _runtimeType: "parachain" | "solochain";

  private constructor(
    api: ApiPromise,
    endpoint: `ws://${string}` | `wss://${string}`,
    runtimeType: "parachain" | "solochain"
  ) {
    this._api = api;
    this._endpoint = endpoint;
    this._runtimeType = runtimeType;
  }

  /**
   * Creates a new instance of BspNetTestApi.
   *
   * @param endpoint - The WebSocket endpoint to connect to.
   * @returns A promise that resolves to an enriched BspNetApi.
   */
  public static async create(
    endpoint: `ws://${string}` | `wss://${string}`,
    runtimeType?: "parachain" | "solochain"
  ) {
    const api = await BspNetTestApi.connect(endpoint);
    await api.isReady;

    const ctx = new BspNetTestApi(api, endpoint, runtimeType ?? "parachain");

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
      isPedantic: false,
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

  private async createBucketAndSendNewStorageRequest(
    source: string,
    location: string,
    bucketName: string,
    valuePropId: HexString
  ) {
    return Files.createBucketAndSendNewStorageRequest(
      this._api,
      source,
      location,
      bucketName,
      this._runtimeType === "solochain" ? ethShUser : shUser,
      valuePropId
    );
  }

  private async createBucket(bucketName: string, valuePropId?: HexString | null) {
    return Files.createBucket(
      this._api,
      bucketName,
      this._runtimeType === "solochain" ? ethShUser : shUser,
      valuePropId
    );
  }

  private assertEvent(module: string, method: string, events?: EventRecord[]) {
    return Assertions.assertEventPresent(this._api, module, method, events);
  }

  private enrichApi() {
    const runtimeType = this._runtimeType;
    const remappedAccountsNs = {
      sudo: runtimeType === "solochain" ? alith : alice,
      bspKey: runtimeType === "solochain" ? ethBspKey : bspKey,
      bspDownKey: runtimeType === "solochain" ? ethBspDownKey : bspDownKey,
      bspTwoKey: runtimeType === "solochain" ? ethBspTwoKey : bspTwoKey,
      bspThreeKey: runtimeType === "solochain" ? ethBspThreeKey : bspThreeKey,
      mspKey: runtimeType === "solochain" ? ethMspKey : mspKey,
      mspDownKey: runtimeType === "solochain" ? ethMspDownKey : mspDownKey,
      mspTwoKey: runtimeType === "solochain" ? ethMspTwoKey : mspTwoKey,
      mspThreeKey: runtimeType === "solochain" ? ethMspThreeKey : mspThreeKey,
      shUser: runtimeType === "solochain" ? ethShUser : shUser
    } as const;

    const remappedAssertNs = {
      fetchEvent: Assertions.fetchEvent,

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
      log: async (options: { searchString: string; containerName: string; timeout?: number }) => {
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
       * @param finalizeBlock - Optional param to specify whether to finalize the block after volunteering.
       * @returns A promise that resolves when a BSP has volunteered.
       */
      bspVolunteer: (expectedExts?: number) => Waits.waitForBspVolunteer(this._api, expectedExts),

      /**
       * Waits for a BSP to submit to the tx pool the extrinsic to volunteer for a storage request.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a BSP has volunteered.
       */
      bspVolunteerInTxPool: (expectedExts?: number) =>
        Waits.waitForBspVolunteerWithoutSealing(this._api, expectedExts),

      /**
       * Waits for a BSP to confirm storing a file.
       *
       * Checks that `expectedExts` extrinsics have been submitted to the tx pool.
       * Then seals a block and checks for the `BspConfirmedStoring` events.
       * @param options - Options for the BSP Stored waiting utility function.
       * @returns A promise that resolves when a BSP has confirmed storing a file.
       */
      bspStored: (
        options: BspStoredOptions = {
          expectedExts: undefined,
          bspAccount: undefined,
          timeoutMs: undefined,
          sealBlock: true,
          finalizeBlock: true
        }
      ) =>
        Waits.waitForBspStored(
          this._api,
          options.expectedExts,
          options.bspAccount,
          options.timeoutMs,
          options.sealBlock,
          options.finalizeBlock
        ),

      /**
       * A generic utility to wait for a transaction to be in the tx pool.
       * @param options - Options for the wait.
       * @returns A promise that resolves when the transaction is in the tx pool.
       */
      waitForTxInPool: (options: WaitForTxOptions) => Waits.waitForTxInPool(this._api, options),

      /**
       * Waits for a Storage Provider to complete storing a file key.
       * @param fileKey - Param to specify the file key to wait for.
       * @returns A promise that resolves when a BSP has completed to store a file.
       */
      fileStorageComplete: (fileKey: H256 | string) =>
        Waits.waitForFileStorageComplete(this._api, fileKey),

      /**
       * Waits for a Storage Provider to complete deleting a file key from the file storage.
       * @param fileKey - Param to specify the file key to wait for.
       * @returns A promise that resolves when the Provider has completed to delete the file.
       */
      fileDeletionFromFileStorage: (fileKey: H256 | string) =>
        Waits.waitForFileDeletionFromFileStorageComplete(this._api, fileKey),

      /**
       * Waits for a BSP to complete deleting a file from its forest.
       * @param fileKey - Param to specify the file key to wait for deletion.
       * @returns A promise that resolves when a BSP has correctly deleted the file from its forest storage.
       */
      bspFileDeletionCompleted: (fileKey: H256 | string) =>
        Waits.waitForBspFileDeletionComplete(this._api, fileKey),

      /**
       * Waits for a MSP to complete deleting a bucket from its forest.
       * @param fileKey - Param to specify the bucket ID of the bucket to wait for deletion.
       * @returns A promise that resolves when the MSP has correctly deleted the bucket from its forest storage.
       */
      mspBucketDeletionCompleted: (bucketId: H256 | string) =>
        Waits.waitForMspBucketDeletionComplete(this._api, bucketId),

      /**
       * Waits for a node to catch up to the tip of the chain
       * @param nodeBehindApi - The Api object of the node that is behind
       * @returns A promise that resolves when a node has caught up to the tip of the chain
       */
      nodeCatchUpToChainTip: (nodeBehindApi: ApiPromise) =>
        Waits.waitForNodeToCatchUpToChainTip(this._api, nodeBehindApi),

      /**
       * Waits for a node to have imported a block.
       * @param blockHash - The hash of the block to wait for.
       * @returns A promise that resolves when the block is imported.
       */
      blockImported: (blockHash: string) => Waits.waitForBlockImported(this._api, blockHash),

      // TODO: Maybe we should refactor these to a different file under `mspNet` or something along those lines
      /**
       * Waits for a MSP to submit a proof for a pending file deletion request.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a MSP has submitted a proof for a pending file deletion request.
       */
      mspPendingFileDeletionRequestSubmitProof: (expectedExts?: number) =>
        Waits.waitForMspPendingFileDeletionRequestSubmitProof(this._api, expectedExts),

      /**
       * Waits for a MSP to submit to the tx pool the extrinsic to respond to storage requests.
       * @param expectedExts - Optional param to specify the number of expected extrinsics.
       * @returns A promise that resolves when a MSP has submitted to the tx pool the extrinsic to respond to storage requests.
       */
      mspResponseInTxPool: (expectedExts?: number, timeoutMs?: number) =>
        Waits.waitForMspResponseWithoutSealing(this._api, expectedExts, timeoutMs),

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
       * @param address - The address of the account to wait for.
       * @returns A promise that resolves when the address has no pending extrinsics.
       */
      waitForAvailabilityToSendTx: (address: string) =>
        Waits.waitForAvailabilityToSendTx(this._api, address),

      /**
       * Waits for a storage request to be removed from on-chain storage.
       * This could happen because the storage request was fulfilled, it has expired or
       * it has been rejected by the MSP. Either way, we only care that it's not on-chain anymore.
       * @param fileKey - File key of the storage request to wait for.
       * @returns A promise that resolves when the storage request is not on-chain.
       */
      storageRequestNotOnChain: (fileKey: H256 | string) =>
        Waits.waitForStorageRequestNotOnChain(this._api, fileKey),

      /**
       * Waits for a storage request to be fulfilled by waiting and sealing blocks until
       * the StorageRequestFulfilled event is detected.
       * @param fileKey - File key of the storage request to wait for.
       * @returns A promise that resolves when the storage request has been fulfilled.
       */
      storageRequestFulfilled: (fileKey: H256 | string) =>
        Waits.waitForStorageRequestFulfilled(this._api, fileKey)
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
      newBucket: (
        bucketName: string,
        owner?: KeyringPair,
        valuePropId?: HexString | null,
        mspId?: HexString | null
      ) =>
        Files.createBucket(
          this._api,
          bucketName,
          owner ?? (this._runtimeType === "solochain" ? ethShUser : shUser),
          valuePropId,
          mspId
        ),

      /**
       * Issue a new storage request.
       *
       * @param source - The local path to the file to be uploaded.
       * @param location - The StorageHub "location" field of the file to be uploaded.
       * @param bucketID - The ID of the bucket to use for the new storage request.
       * @param owner - Signer with which to issue the newStorageRequest Defaults to SH_USER.
       * @param mspId - <TODO> Optional MSP ID to use for the new storage request. Defaults to DUMMY_MSP_ID.
       * @returns A promise that resolves to file metadata.
       */
      newStorageRequest: (
        source: string,
        location: string,
        bucketId: H256,
        owner?: KeyringPair,
        msp_id?: HexString,
        replicationTarget?: number
      ) =>
        Files.sendNewStorageRequest(
          this._api,
          source,
          location,
          bucketId,
          owner ?? (this._runtimeType === "solochain" ? ethShUser : shUser),
          msp_id,
          replicationTarget
        ),

      /**
       * Creates a new bucket and submits a new storage request.
       *
       * @param source - The local path to the file to be uploaded.
       * @param location - The StorageHub "location" field of the file to be uploaded.
       * @param bucketName - The name of the bucket to be created.
       * @param mspId - <TODO> Optional MSP ID to use for the new storage request. Defaults to DUMMY_MSP_ID.
       * @param owner - Optional signer with which to issue the newStorageRequest Defaults to SH_USER.
       * @param replicationTarget - Optional number of replicas to store the file. Defaults to the BasicReplicationTarget of the runtime.
       * @param finalizeBlock - Optional boolean to finalize the blocks created when sending the new storage request. Defaults to true.
       * @returns A promise that resolves to file metadata.
       */
      createBucketAndSendNewStorageRequest: (
        source: string,
        location: string,
        bucketName: string,
        valuePropId?: HexString | null,
        msp_id?: HexString | null,
        owner?: KeyringPair | null,
        replicationTarget?: number | null,
        finalizeBlock?: boolean
      ) =>
        Files.createBucketAndSendNewStorageRequest(
          this._api,
          source,
          location,
          bucketName,
          owner ?? (this._runtimeType === "solochain" ? ethShUser : shUser),
          valuePropId,
          msp_id,
          replicationTarget,
          finalizeBlock
        )
    };

    /**
     * Block operations namespace
     * Contains methods for manipulating and interacting with blocks in the BSP network.
     */
    const remappedBlockNs = {
      /**
       * Extends a fork in the blockchain by creating new blocks on top of a specified parent block.
       *
       * This function is used for testing chain fork scenarios. It creates a specified number
       * of new blocks, each building on top of the previous one, starting from a given parent
       * block hash.
       *
       * @param options - Configuration options for extending the fork:
       *   @param options.parentBlockHash - The hash of the parent block to build upon.
       *   @param options.amountToExtend - The number of blocks to add to the fork.
       *   @param options.verbose - If true, logs detailed information about the fork extension process.
       *
       * @returns A Promise that resolves when all blocks have been created.
       */
      extendFork: (options: {
        /**
         * The hash of the parent block to build upon.
         *  e.g. "0x827392aa...."
         */
        parentBlockHash: string;
        /**
         * The number of blocks to add to the fork.
         *  e.g. 5
         */
        amountToExtend: number;
        /**
         * If true, logs detailed information about the fork extension process.
         *  e.g. true
         */
        verbose?: boolean;
      }) =>
        BspNetBlock.extendFork(this._api, {
          ...options,
          verbose: options.verbose ?? false
        }),
      /**
       * Seals a block with optional extrinsics.
       * @param options - Options for sealing the block, including calls, signer, and whether to finalize.
       * @returns A promise that resolves to a SealedBlock object.
       */
      seal: (options?: SealBlockOptions) =>
        BspNetBlock.sealBlock(
          this._api,
          options?.calls,
          options?.signer ?? (this._runtimeType === "solochain" ? alith : alice),
          options?.nonce,
          options?.parentHash,
          options?.finaliseBlock,
          options?.failOnExtrinsicNonInclusion
        ),
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
       * Skips a specified number of blocks quickly.
       * Use this when you just need to advance the chain and don't care about BSP reactions.
       *
       * @param input - Either:
       *   - A number specifying how many blocks to advance, or
       *   - An options object with:
       *     @param blocksToAdvance - The number of blocks to skip
       *     @param paddingMs - Time in milliseconds to wait between blocks
       * @returns A promise that resolves when the specified number of blocks have been skipped.
       */
      skip: (input: number | { blocksToAdvance: number; paddingMs?: number }) => {
        if (typeof input === "number") {
          return BspNetBlock.skipBlocks(this._api, input);
        }
        return BspNetBlock.skipBlocks(this._api, input.blocksToAdvance, input.paddingMs);
      },
      /**
       * Advances the chain to a specific block number, allowing time for BSPs to react.
       * Use this when you need BSPs to have time to submit proofs or other reactions.
       *
       * @param blockNumber - The target block number to advance to
       * @param options - Optional configuration:
       *   @param options.waitBetweenBlocks - Time to wait between blocks (ms), or true for default wait
       *   @param options.watchForBspProofs - Array of BSP addresses to watch for proofs from
       *   @param options.finalised - Whether to finalize the blocks
       *   @param options.spam - Whether to include spam transactions
       *   @param options.verbose - Whether to log detailed progress
       * @returns A promise that resolves when the specified block number is reached
       */
      skipTo: (
        blockNumber: number,
        options?: {
          waitBetweenBlocks?: number | boolean;
          watchForBspProofs?: string[];
          finalised?: boolean;
          spam?: boolean;
          verbose?: boolean;
        }
      ) => BspNetBlock.advanceToBlock(this._api, { ...options, blockNumber }),
      /**
       * Skips blocks until the minimum time for capacity changes is reached.
       * It will stop at the block before the minimum change time is reached since the capacity
       * change extrinsic will be sent and included in the next block.
       *
       * @param bspId - The ID of the BSP that the capacity change is for.
       * @returns A promise that resolves when the minimum change time is reached.
       */
      skipUntilBspCanChangeCapacity: (bspId?: `0x${string}` | H256 | Uint8Array) =>
        BspNetBlock.skipBlocksUntilBspCanChangeCapacity(this._api, bspId),
      /**
       * Finalises a block (and therefore all of its predecessors) in the blockchain.
       *
       * @param api - The ApiPromise instance.
       * @param hashToFinalise - The hash of the block to finalise.
       * @returns A Promise that resolves when the chain reorganization is complete.
       */
      finaliseBlock: (hashToFinalise: string) =>
        BspNetBlock.finaliseBlock(this._api, hashToFinalise),

      /**
       * Performs a chain reorganisation by creating a finalised block on top of the parent block.
       *
       * This function is used to simulate network forks and test the system's ability to handle
       * chain reorganizations. It's a critical tool for ensuring the robustness of the BSP network
       * in face of potential consensus issues.
       *
       * @throws Will throw an error if the head block is already finalised.
       * @returns A Promise that resolves when the chain reorganization is complete.
       */
      reOrgWithFinality: () => BspNetBlock.reOrgWithFinality(this._api),
      /**
       * Performs a chain reorganisation by creating a longer forked chain.
       * If no parent starting block is provided, the chain will start the fork from the last
       * finalised block.
       *
       * !!! WARNING !!!
       *
       * The number of blocks this function can create for the alternative fork is limited by the
       * "unincluded segment capacity" parameter, set in the `ConsensusHook` config type of the
       * `cumulus-pallet-parachain-system`. If you try to build more blocks than this limit to
       * achieve the reorg, the node will panic when building the block.
       *
       * This function is used to simulate network forks and test the system's ability to handle
       * chain reorganizations. It's a critical tool for ensuring the robustness of the BSP network
       * in face of potential consensus issues.
       *
       * @param startingBlockHash - Optional. The hash of the block to start the fork from.
       * @throws Will throw an error if the last finalised block is greater than the starting block
       *         or if the starting block is the same or higher than the current block.
       * @returns A promise that resolves when the chain re-org is complete.
       */
      reOrgWithLongerChain: (startingBlockHash?: string) =>
        BspNetBlock.reOrgWithLongerChain(this._api, startingBlockHash)
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
        bspId?: string;
        bspStartingWeight?: bigint;
        maxStorageCapacity?: number;
        additionalArgs?: string[];
        waitForIdle?: boolean;
      }) =>
        addBsp(
          this._api,
          options.bspSigner,
          this._runtimeType === "solochain" ? alith : alice,
          options
        )
    };

    return Object.assign(this._api, {
      /**
       * Soon Deprecated. Use api.file.newStorageRequest() instead.
       * @see {@link createBucketAndSendNewStorageRequest}
       */
      createBucketAndSendNewStorageRequest: this.createBucketAndSendNewStorageRequest.bind(this),
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
      /**
       * Accounts namespace
       * Provides runtime-dependent test accounts for convenience.
       * Access as: api.accounts.sudo, api.accounts.bspKey, etc.
       */
      accounts: remappedAccountsNs,
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
 * This API is created using the BspNetTestApi.create() static method and provides
 * a comprehensive toolkit for testing and developing BSP network functionality.
 */
export type EnrichedBspApi = Awaited<ReturnType<typeof BspNetTestApi.create>>;
