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
  ethFishermanKey,
  ethMspDownKey,
  ethMspKey,
  ethMspThreeKey,
  ethMspTwoKey,
  ethShUser
} from "../evmNet/keyring";
import { createPendingSqlClient } from "../helpers";
import {
  alice,
  bspDownKey,
  bspKey,
  bspThreeKey,
  bspTwoKey,
  fishermanKey,
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
import * as BspNetFisherman from "./fisherman";
import { addBsp } from "./helpers";
import * as BspNetIndexer from "./indexer";
import * as NodeBspNet from "./node";
import * as PendingDb from "./pending";
import type { BspNetApi, BspStoredOptions, SealBlockOptions, SqlClient } from "./types";
import * as Waits from "./waits";
import * as Prometheus from "../prometheus";

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
   * @param runtimeType - The type of runtime ("parachain" or "solochain").
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
      shUser: runtimeType === "solochain" ? ethShUser : shUser,
      fishermanKey: runtimeType === "solochain" ? ethFishermanKey : fishermanKey
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
       * Waits for an MSP to complete deleting a file from a bucket in its forest.
       * @param fileKey - Param to specify the file key to wait for deletion.
       * @param bucketId - Param to specify the bucket ID to wait for deletion.
       * @returns A promise that resolves when an MSP has correctly deleted the file from its bucket forest storage.
       */
      mspBucketFileDeletionCompleted: (fileKey: H256 | string, bucketId: H256 | string) =>
        Waits.waitForMspBucketFileDeletionComplete(this._api, fileKey, bucketId),

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
        ),

      /**
       * Batches multiple storage requests together for efficient processing.
       *
       * This function handles the complete flow where both BSP and MSP respond:
       * 1. Creates buckets if bucket names are provided (deduplicates unique bucket names)
       * 2. Prepares all storage request transactions for the provided files
       * 3. Pauses MSP1 container to deterministically control storage request flow
       * 4. Seals all storage requests in a single block (finalized or unfinalized based on `finaliseBlock`)
       * 5. Waits for all BSP volunteers to appear in tx pool
       * 6. Processes BSP confirmations in batches (handles batched extrinsics)
       * 7. Verifies all files are confirmed by BSP
       * 8. Waits for BSP to store all files locally
       * 9. Unpauses MSP1 container
       * 10. Waits for MSP to catch up to chain tip
       * 11. Processes MSP acceptances in batches (handles batched extrinsics)
       * 12. Verifies all files are accepted by MSP
       * 13. Waits for MSP to store all files locally
       * 14. Returns all file metadata (fileKeys, bucketIds, locations, fingerprints, fileSizes)
       *
       * **Purpose:**
       * This helper simplifies the common case of batch creating storage requests where both BSP and MSP
       * respond. For tests that need more granular control (e.g., BSP-only or MSP-only scenarios), write
       * custom logic instead of using this helper.
       *
       * **Parameter Requirements:**
       * - `bspApi` is required for verifying BSP file storage
       * - `mspApi` is required for MSP catchup and verifying MSP file storage
       * - `owner` is always required (defaults to `shUser` or `ethShUser` based on runtime type)
       *
       * @param options - Batch storage request options
       * @returns Promise resolving to batch storage request result with all file metadata
       */
      batchStorageRequests: (
        options: Omit<Files.BatchStorageRequestsOptions, "owner"> & {
          owner?: KeyringPair;
        }
      ) =>
        Files.batchStorageRequests(this._api as EnrichedBspApi, {
          ...options,
          owner: options.owner ?? (this._runtimeType === "solochain" ? ethShUser : shUser)
        })
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
        BspNetBlock.reOrgWithLongerChain(this._api, startingBlockHash),

      /**
       * Calculates the next challenge tick for a given provider.
       * @param options - Options object
       * @param options.api - The enriched BSP API instance.
       * @param options.providerId - The provider ID to calculate the next challenge tick for.
       * @returns The next challenge tick block number.
       * @throws Error if the API call fails or returns an error.
       */
      calculateNextChallengeTick: (options: { api: any; providerId: string }) =>
        BspNetBlock.calculateNextChallengeTick(options),

      /**
       * Triggers a complete provider charging cycle by advancing to the next challenge tick,
       * waiting for proof submission, and processing the charge transaction.
       * @param options - Options object
       * @param options.api - The enriched BSP API instance.
       * @param options.providerId - The provider ID to trigger charging for.
       * @param options.userAddress - Optional user address for balance tracking.
       * @returns Object containing charging event details including if user became insolvent.
       */
      triggerProviderChargingCycle: (options: {
        api: any;
        providerId: string;
        userAddress?: string;
      }) => BspNetBlock.triggerProviderChargingCycle(options),

      /**
       * Keeps charging a user until they become insolvent (UserWithoutFunds event is emitted).
       * This function will repeatedly call triggerProviderChargingCycle until the user runs out of funds.
       * @param options - Options object
       * @param options.api - The enriched BSP API instance.
       * @param options.providerId - The provider ID to charge the user.
       * @param options.maxAttempts - Maximum number of charging attempts to prevent infinite loops (default: 10).
       * @param options.userAddress - Optional user address for balance logging and debugging.
       * @returns Object containing details about all charging cycles and final result.
       */
      chargeUserUntilInsolvent: (options: {
        api: any;
        providerId: string;
        maxAttempts?: number;
        userAddress?: string;
      }) => BspNetBlock.chargeUserUntilInsolvent(options)
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

    /**
     * Indexer operations namespace
     * Contains methods for interacting with the indexer and verifying indexed data.
     */
    const remappedIndexerNs = {
      /**
       * Waits for the indexer to process blocks from a producer node.
       *
       * This method should be called on the indexer API instance (e.g., `indexerApi.indexer.waitForIndexing(...)`).
       * For embedded indexers, you can now omit the producer API parameter (e.g., `userApi.indexer.waitForIndexing()`).
       *
       * @param options - Options object or producer API (for backward compatibility)
       * @param options.producerApi - Optional. The producer API to get block number and finalization status from.
       * @param options.sealBlock - Whether to seal a new block on the producer (default: `true`)
       * @param options.finalizeOnIndexer - Whether to finalize blocks on this indexer node (default: `true`)
       * @returns A Promise that resolves when the indexer has processed the block
       *
       * @example
       * // Standalone indexer
       * await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
       *
       * @example
       * // Embedded indexer (simplified)
       * await userApi.indexer.waitForIndexing({});
       */
      waitForIndexing: (options: {
        producerApi?: any;
        sealBlock?: boolean;
        finalizeOnIndexer?: boolean;
        sql: SqlClient;
      }) =>
        BspNetIndexer.waitForIndexing({
          indexerApi: this._api as any,
          ...options
        }),

      /**
       * Verifies that a file has been indexed in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.bucketName - The name of the bucket containing the file.
       * @param options.fileKey - The file key to verify.
       * @returns The indexed file record from the database.
       */
      verifyFileIndexed: (options: { sql: any; bucketName: string; fileKey: string }) =>
        BspNetIndexer.verifyFileIndexed(options),

      /**
       * Verifies that a provider association exists in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check association for.
       * @param options.providerId - The provider ID to verify association with.
       * @param options.providerType - The type of provider ("msp" or "bsp").
       * @returns The provider association record from the database.
       */
      verifyProviderAssociation: (options: {
        sql: any;
        fileKey: string;
        providerId: string;
        providerType: "msp" | "bsp";
      }) => BspNetIndexer.verifyProviderAssociation(options),

      /**
       * Verifies that deletion signatures are stored in the database for all specified file keys.
       *
       * This function waits for the first file to have a deletion signature stored, then verifies
       * that all files have non-empty SCALE-encoded deletion signatures in the database.
       *
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKeys - Array of file keys to verify have deletion signatures.
       * @throws Error if any file doesn't have a deletion signature stored or if the signature is empty.
       */
      verifyDeletionSignaturesStored: (options: { sql: any; fileKeys: string[] }) =>
        BspNetIndexer.verifyDeletionSignaturesStored(options),

      /**
       * Waits for a specific block to be indexed by checking docker logs.
       * @param options - Options object
       * @param options.api - The indexer API
       * @param options.blockNumber - Optional block number to wait for. Defaults to current block.
       * @returns A Promise that resolves when the block has been indexed.
       */
      waitForBlockIndexed: (options: { api: any; blockNumber?: number }) =>
        BspNetIndexer.waitForBlockIndexed(options),

      /**
       * Waits for a file to be indexed in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to wait for.
       * @returns A Promise that resolves when the file is indexed.
       */
      waitForFileIndexed: (options: { sql: any; fileKey: string }) =>
        BspNetIndexer.waitForFileIndexed(options),

      /**
       * Waits for a bucket to be indexed in the database by name.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.bucketName - The bucket name to wait for.
       * @returns A Promise that resolves when the bucket is indexed.
       */
      waitForBucketIndexed: (options: { sql: any; bucketName: string }) =>
        BspNetIndexer.waitForBucketIndexed(options),

      /**
       * Waits for a bucket to be indexed in the database by onchain bucket ID.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.bucketId - The onchain bucket ID to wait for.
       * @param options.mspId - Optional MSP ID filter.
       * @returns A Promise that resolves when the bucket is indexed.
       */
      waitForBucketByIdIndexed: (options: { sql: any; bucketId: string; mspId?: string }) =>
        BspNetIndexer.waitForBucketByIdIndexed(options),

      /**
       * Waits for a bucket to be marked as deleted in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.bucketId - The onchain bucket ID to wait for deletion.
       * @returns A Promise that resolves when the bucket is marked as deleted.
       */
      waitForBucketDeleted: (options: { sql: any; bucketId: string }) =>
        BspNetIndexer.waitForBucketDeleted(options),

      /**
       * Waits for an MSP file association to be created in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check association for.
       * @param options.mspId - Optional MSP ID filter.
       * @returns A Promise that resolves when the association exists.
       */
      waitForMspFileAssociation: (options: { sql: any; fileKey: string; mspId?: string }) =>
        BspNetIndexer.waitForMspFileAssociation(options),

      /**
       * Waits for a BSP file association to be created in the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check association for.
       * @param options.bspId - Optional BSP ID filter.
       * @returns A Promise that resolves when the association exists.
       */
      waitForBspFileAssociation: (options: { sql: any; fileKey: string; bspId?: string }) =>
        BspNetIndexer.waitForBspFileAssociation(options),

      /**
       * Waits for a file to be deleted from the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to wait for deletion.
       * @returns A Promise that resolves when the file is deleted.
       */
      waitForFileDeleted: (options: { sql: any; fileKey: string }) =>
        BspNetIndexer.waitForFileDeleted(options),

      /**
       * Waits for a BSP file association to be removed from the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check association for.
       * @param options.bspId - Optional BSP ID filter.
       * @returns A Promise that resolves when the association is removed.
       */
      waitForBspFileAssociationRemoved: (options: { sql: any; fileKey: string; bspId?: string }) =>
        BspNetIndexer.waitForBspFileAssociationRemoved(options),

      /**
       * Waits for an MSP file association to be removed from the database.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check association for.
       * @param options.mspId - Optional MSP ID filter.
       * @returns A Promise that resolves when the association is removed.
       */
      waitForMspFileAssociationRemoved: (options: { sql: any; fileKey: string; mspId?: string }) =>
        BspNetIndexer.waitForMspFileAssociationRemoved(options),

      /**
       * Verifies that no BSP file associations exist for a given file.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check.
       * @throws Error if associations are found.
       */
      verifyNoBspFileAssociation: (options: { sql: any; fileKey: string }) =>
        BspNetIndexer.verifyNoBspFileAssociation(options),

      /**
       * Verifies that no MSP file associations exist for a given file.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.fileKey - The file key to check.
       * @throws Error if associations are found.
       */
      verifyNoMspFileAssociation: (options: { sql: any; fileKey: string }) =>
        BspNetIndexer.verifyNoMspFileAssociation(options),

      /**
       * Verifies that no orphaned BSP associations exist (associations without corresponding files).
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.bspId - The BSP ID to check.
       * @throws Error if orphaned associations are found.
       */
      verifyNoOrphanedBspAssociations: (options: { sql: any; bspId: string }) =>
        BspNetIndexer.verifyNoOrphanedBspAssociations(options),

      /**
       * Verifies that no orphaned MSP associations exist (associations without corresponding files).
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @param options.mspId - The MSP ID to check.
       * @throws Error if orphaned associations are found.
       */
      verifyNoOrphanedMspAssociations: (options: { sql: any; mspId: string }) =>
        BspNetIndexer.verifyNoOrphanedMspAssociations(options),

      /**
       * Get the last indexed block number from the service_state table.
       * @param options - Options object
       * @param options.sql - The SQL client instance.
       * @returns The last indexed finalized block number.
       */
      getLastIndexedBlock: (options: { sql: any }) => BspNetIndexer.getLastIndexedBlock(options)
    };

    /**
     * Pending transactions DB namespace
     * Helpers to interact with the pending transactions Postgres database.
     */
    const remappedPendingDbNs = {
      /**
       * Creates a client connected to the pending transactions DB.
       * Default connection maps to docker compose service sh-pending-postgres -> localhost:5433.
       */
      createClient: () => createPendingSqlClient(),
      /**
       * Utility to convert an ss58 address into AccountId bytes for DB queries.
       */
      accountIdFromAddress: (address: string) => PendingDb.accountIdFromAddress(address),
      /**
       * Returns row (transaction) for (account, nonce) if present.
       */
      getByNonce: (options: { sql: SqlClient; accountId: Buffer; nonce: bigint }) =>
        PendingDb.getByNonce(options),
      /**
       * Returns all rows (transactions) for an account ordered by nonce.
       */
      getAllByAccount: (options: { sql: SqlClient; accountId: Buffer }) =>
        PendingDb.getAllByAccount(options),
      /**
       * Counts active-state rows for an account.
       *
       * Active-state rows are transactions which are not in terminal states.
       * Terminal states are: "finalized", "dropped", "invalid", "usurped", "finality_timeout".
       */
      countActive: (options: { sql: SqlClient; accountId: Buffer }) =>
        PendingDb.countActive(options),
      /**
       * Waits until a given nonce reaches target state, or times out.
       */
      waitForState: (options: {
        sql: SqlClient;
        accountId: Buffer;
        nonce: bigint;
        state: string;
        timeoutMs?: number;
        pollMs?: number;
      }) => PendingDb.waitForState(options),
      /**
       * Asserts there are no active rows with nonce < onChainNonce.
       */
      expectClearedBelow: (options: { sql: SqlClient; accountId: Buffer; onChainNonce: bigint }) =>
        PendingDb.expectClearedBelow(options)
    };

    /**
     * Fisherman operations namespace
     * Contains methods for interacting with and testing fisherman node functionality.
     */
    const remappedFishermanNs = {
      /**
       * Waits for fisherman to process batch deletions by sealing blocks until
       * the fisherman submits extrinsics for the specified deletion type.
       *
       * This handles the alternating User/Incomplete deletion cycle timing issue
       * where fisherman might be on the wrong cycle when deletions are created.
       *
       * If `expectExt` is provided, this function will verify that the expected
       * number of extrinsics (BSP + bucket) are present in the transaction pool before returning,
       * preventing a race condition where blocks are sealed before verification.
       *
       * If `sealBlock` is true, a block will be sealed after verifying extrinsics.
       * Defaults to false to allow manual block sealing in tests.
       *
       * @param options - Options object
       * @param options.deletionType - Either "User" or "Incomplete" to determine which deletion cycle to wait for
       * @param options.expectExt - Optional. Total expected extrinsics (BSP + bucket) to verify in the transaction pool
       * @param options.sealBlock - Optional. Whether to seal a block after verifying extrinsics. Defaults to false.
       */
      waitForBatchDeletions: (options: {
        deletionType: "User" | "Incomplete";
        expectExt?: number;
        sealBlock?: boolean;
      }) =>
        BspNetFisherman.waitForFishermanBatchDeletions({
          blockProducerApi: this._api as EnrichedBspApi,
          ...options
        }),

      /**
       * Verifies BSP deletion results from a batch deletion operation.
       *
       * This function verifies:
       * 1. The expected number of BSP deletion events are present
       * 2. The BSP forest root has changed (oldRoot !== newRoot)
       * 3. The current BSP forest root matches the newRoot from the deletion event
       *
       * @param options - Verification options
       * @param options.userApi - The enriched BSP API for assertions and event fetching
       * @param options.bspApi - The BSP API instance for forest root verification
       * @param options.events - Events array from the sealed block
       * @param options.expectedCount - Expected number of BSP deletion events. Defaults to 1.
       */
      verifyBspDeletionResults: (options: {
        userApi: any;
        bspApi: any;
        events: any[];
        expectedCount?: number;
      }) => BspNetFisherman.verifyBspDeletionResults(options),

      /**
       * Verifies bucket deletion results from a batch deletion operation.
       *
       * This function verifies:
       * 1. The expected number of bucket deletion events are present
       * 2. For each bucket, the forest root has changed (oldRoot !== newRoot)
       * 3. For each bucket, the current forest root matches the newRoot from the deletion event
       *
       * @param options - Verification options
       * @param options.userApi - The enriched BSP API for assertions and event fetching
       * @param options.mspApi - The MSP API instance for bucket forest root verification
       * @param options.events - Events array from the sealed block
       * @param options.expectedCount - Expected number of bucket deletion events
       */
      verifyBucketDeletionResults: (options: {
        userApi: any;
        mspApi: any;
        events: any[];
        expectedCount: number;
      }) => BspNetFisherman.verifyBucketDeletionResults(options),

      /**
       * Waits for fisherman batch deletions and verifies BSP deletion results with retry logic.
       *
       * This function will retry both `waitForBatchDeletions` and `verifyBspDeletionResults`
       * if `ForestProofVerificationFailed` errors are detected in the events, up to a maximum
       * number of attempts (defaults to 3).
       *
       * @param options - Options for the retryable batch deletions
       * @param options.blockProducerApi - The block producer API (normally userApi)
       * @param options.deletionType - Either "User" or "Incomplete" to determine which deletion cycle to wait for
       * @param options.expectExt - Optional. Total expected extrinsics (BSP + bucket) to verify in the transaction pool
       * @param options.userApi - The enriched BSP API for assertions and event fetching
       * @param options.bspApi - The BSP API instance for forest root verification
       * @param options.expectedBspCount - Expected number of BSP deletion events. Defaults to 1.
       * @param options.mspApi - Optional MSP API instance for bucket forest root verification
       * @param options.expectedBucketCount - Expected number of bucket deletion events. If provided, bucket deletions will be verified.
       * @param options.maxRetries - Maximum number of retry attempts. Defaults to 3.
       * @param options.skipBucketIds - Optional. Array of bucket IDs to skip forest root verification for (e.g., when MSP stopped storing the bucket)
       */
      retryableWaitAndVerifyBatchDeletions: (options: {
        blockProducerApi: any;
        deletionType: "User" | "Incomplete";
        expectExt?: number;
        userApi: any;
        bspApi: any;
        expectedBspCount?: number;
        mspApi?: any;
        expectedBucketCount?: number;
        maxRetries?: number;
        skipBucketIds?: string[];
      }) => BspNetFisherman.retryableWaitAndVerifyBatchDeletions(options)
    };

    /**
     * Prometheus operations namespace
     * Contains methods for querying and asserting Prometheus metrics.
     */
    const remappedPrometheusNs = {
      /**
       * Query the Prometheus API with a PromQL query.
       * @param query - PromQL query string
       * @returns Prometheus query result
       */
      query: (query: string) => Prometheus.queryPrometheus(query),

      /**
       * Get the current value of a metric from Prometheus.
       * @param query - PromQL query string
       * @returns Numeric value of the metric, or 0 if not found
       */
      getMetricValue: (query: string) => Prometheus.getMetricValue(query),

      /**
       * Get the targets that Prometheus is currently scraping.
       * @returns Prometheus targets result with active scrape targets
       */
      getTargets: () => Prometheus.getPrometheusTargets(),

      /**
       * Wait for Prometheus to scrape and reflect updated metrics (7s).
       */
      waitForScrape: () => Prometheus.waitForMetricsScrape(),

      /**
       * Wait for the Prometheus server to become ready (up to 60s).
       */
      waitForReady: () => Prometheus.waitForPrometheusReady(),

      /**
       * Assert that a metric has incremented from an initial value.
       * Waits for Prometheus to scrape before checking.
       * @param options - Query string, initial value, and optional message
       */
      assertMetricIncremented: (options: Prometheus.AssertMetricIncrementedOptions) =>
        Prometheus.assertMetricIncremented(options),

      /**
       * Assert that a metric is above a threshold.
       * Waits for Prometheus to scrape before checking.
       * @param options - Query string, threshold, and optional message
       */
      assertMetricAbove: (options: Prometheus.AssertMetricAboveOptions) =>
        Prometheus.assertMetricAbove(options),

      /**
       * Assert that a metric equals an expected value.
       * Waits for Prometheus to scrape before checking.
       * @param options - Query string, expected value, and optional message
       */
      assertMetricEquals: (options: Prometheus.AssertMetricEqualsOptions) =>
        Prometheus.assertMetricEquals(options),

      /**
       * All StorageHub metrics definitions as defined in client/src/metrics.rs.
       */
      metrics: Prometheus.ALL_STORAGEHUB_METRICS,

      /**
       * Default Prometheus URL for tests.
       */
      url: Prometheus.PROMETHEUS_URL
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
       * Indexer operations namespace
       * Contains methods for interacting with the indexer and verifying indexed data.
       */
      indexer: remappedIndexerNs,
      /**
       * Pending transactions DB namespace
       */
      pendingDb: remappedPendingDbNs,
      /**
       * Fisherman operations namespace
       * Contains methods for interacting with and testing fisherman node functionality.
       */
      fisherman: remappedFishermanNs,
      /**
       * Prometheus operations namespace
       * Provides methods for querying and asserting Prometheus metrics.
       */
      prometheus: remappedPrometheusNs,
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
