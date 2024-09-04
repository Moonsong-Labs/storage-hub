import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { HexString } from "@polkadot/util/types";
import "@storagehub/api-augment";
import { types as BundledTypes } from "@storagehub/types-bundle";
import { Assertions, type AssertExtrinsicOptions } from "../asserts";
import { BspNetBlock, sealBlock } from "./block";
import { ShConsts } from "./consts";
import { DockerBspNet } from "./docker";
import { Files } from "./fileHelpers";
import { NodeBspNet } from "./node";
import type { BspNetApi, SealBlockOptions } from "./types";
import { Waits } from "./waits";

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export class BspNetTestApi implements AsyncDisposable {
  private _api: ApiPromise;
  private _endpoint: `ws://${string}` | `wss://${string}`;

  private constructor(api: ApiPromise, endpoint: `ws://${string}` | `wss://${string}`) {
    this._api = api;
    this._endpoint = endpoint;
  }

  /**
   * Creates a new instance of BspNetTestApi.
   *
   * @param endpoint - The WebSocket endpoint to connect to.
   * @returns A promise that resolves to an enriched BspNetApi.
   */
  public static async create(endpoint: `ws://${string}` | `wss://${string}`) {
    const api = await BspNetTestApi.connect(endpoint);
    await api.isReady;

    const ctx = new BspNetTestApi(api, endpoint);

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
    return Files.newStorageRequest(this._api, source, location, bucketName);
  }

  private async createBucket(bucketName: string) {
    return Files.newBucket(this._api, bucketName);
  }

  private assertEvent(module: string, method: string, events?: EventRecord[]) {
    return Assertions.eventPresent(this._api, module, method, events);
  }

  private advanceToBlock(
    blockNumber: number,
    options?: {
      waitBetweenBlocks?: number | boolean;
      waitForBspProofs?: string[];
    }
  ) {
    return BspNetBlock.skipTo(
      this._api,
      blockNumber,
      options?.waitBetweenBlocks,
      options?.waitForBspProofs
    );
  }

  private enrichApi() {
    const remappedAssertNs = {
      ...Assertions,
      eventPresent: (module: string, method: string, events?: EventRecord[]) =>
        Assertions.eventPresent(this._api, module, method, events),
      eventMany: (module: string, method: string, events?: EventRecord[]) =>
        Assertions.eventMany(this._api, module, method, events),
      extrinsicPresent: (options: AssertExtrinsicOptions) =>
        Assertions.extrinsicPresent(this._api, options),
      providerSlashed: (providerId: string) => Assertions.providerSlashed(this._api, providerId)
    };

    const remappedWaitsNs = {
      ...Waits,
      bspVolunteer: () => Waits.bspVolunteer(this._api),
      bspStored: () => Waits.bspStored(this._api)
    };

    const remappedFileNs = {
      ...Files,
      /**
       * Creates a new bucket.
       *
       * @param bucketName - The name of the bucket to be created.
       * @returns A promise that resolves to a new bucket event.
       */
      newBucket: (bucketName: string) => Files.newBucket(this._api, bucketName),
      /**
       * Creates a new bucket and submits a new storage request.
       *
       * @param source - The local path to the file to be uploaded.
       * @param location - The StorageHub "location" field of the file to be uploaded.
       * @param bucketName - The name of the bucket to be created.
       * @returns A promise that resolves to file metadata.
       */
      newStorageRequest: (source: string, location: string, bucketName: string) =>
        Files.newStorageRequest(this._api, source, location, bucketName)
    };

    const remappedBlockNs = {
      ...BspNetBlock,
      /**
       * Creates a new block, with options to include calls, sign with a specific keypair, and finalise the block.
       * @param options - Options for creating the block.
       */
      seal: (options?: SealBlockOptions) =>
        BspNetBlock.seal(this._api, options?.calls, options?.signer, options?.finaliseBlock),
      /**
       * Seal blocks until the next challenge period block.
       *
       * It will verify that the SlashableProvider event is emitted and check if the provider is slashable with an additional failed challenge deadline.
       */
      skipToChallengePeriod: (nextChallengeTick: number, provider: string) =>
        BspNetBlock.skipToChallengePeriod(this._api, nextChallengeTick, provider),
      skip: (blocksToAdvance: number) => BspNetBlock.skip(this._api, blocksToAdvance),
      skipTo: (
        blockNumber: number,
        options?: { waitBetweenBlocks?: number | boolean; waitForBspProofs?: string[] }
      ) =>
        BspNetBlock.skipTo(
          this._api,
          blockNumber,
          options?.waitBetweenBlocks,
          options?.waitForBspProofs
        ),

      skipToMinChangeTime: () => BspNetBlock.skipToMinChangeTime(this._api),
      /**
       * This will cause a chain re-org by creating a finalized block on top of parent block.
       * Note: This requires head block to be unfinalized, otherwise will throw!
       */
      reOrg: () => BspNetBlock.reOrg(this._api)
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
      dropTxn: (extrinsic?: { module: string; method: string } | HexString, sealAfter = true) =>
        NodeBspNet.dropTxn(this._api, extrinsic, sealAfter)
    };

    const remappedDockerNs = {
      ...DockerBspNet
    };

    return Object.assign(this._api, {
      sealBlock: this.sealBlock.bind(this),
      sendNewStorageRequest: this.sendNewStorageRequest.bind(this),
      createBucket: this.createBucket.bind(this),
      assertEvent: this.assertEvent.bind(this),
      advanceToBlock: this.advanceToBlock.bind(this),
      assert: remappedAssertNs,
      wait: remappedWaitsNs,
      file: remappedFileNs,
      node: remappedNodeNs,
      block: remappedBlockNs,
      shConsts: ShConsts,
      docker: remappedDockerNs,
      [Symbol.asyncDispose]: this.disconnect.bind(this)
    }) satisfies BspNetApi;
  }

  async [Symbol.asyncDispose]() {
    await this._api.disconnect();
  }
}

export type EnrichedBspApi = Awaited<ReturnType<typeof BspNetTestApi.create>>;
