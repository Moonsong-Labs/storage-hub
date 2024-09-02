import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { EventRecord } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import "@storagehub/api-augment";
import { types as BundledTypes } from "@storagehub/types-bundle";
import {
  assertEventMany,
  assertEventPresent,
  assertExtrinsicPresent,
  Assertions,
  type AssertExtrinsicOptions
} from "../asserts";
import { createBucket, Files, sendNewStorageRequest } from "./fileHelpers";
import { sealBlock } from "./helpers";
import type { BspNetApi } from "./types";
import { waitForBspStored, waitForBspVolunteer, Waits } from "./waits";

export class BspNetTestApi implements AsyncDisposable {
  private _api: ApiPromise;

  private constructor(api: ApiPromise) {
    this._api = api;
  }

  public static async create(endpoint: `ws://${string}` | `wss://${string}`) {
    const api = await ApiPromise.create({
      provider: new WsProvider(endpoint),
      noInitWarn: true,
      throwOnConnect: false,
      throwOnUnknown: false,
      typesBundle: BundledTypes
    });

    const ctx = new BspNetTestApi(api);

    return ctx.enrichApi();
  }

  private async sealBlock(
    calls?:
      | SubmittableExtrinsic<"promise", ISubmittableResult>
      | SubmittableExtrinsic<"promise", ISubmittableResult>[],
    signer?: KeyringPair
  ) {
    return sealBlock(this._api, calls, signer);
  }

  private async sendNewStorageRequest(source: string, location: string, bucketName: string) {
    return sendNewStorageRequest(this._api, source, location, bucketName);
  }
  private async createBucket(bucketName: string) {
    return createBucket(this._api, bucketName);
  }

  private assertEvent(module: string, method: string, events?: EventRecord[]) {
    return assertEventPresent(this._api, module, method, events);
  }

  private enrichApi() {
    const remappedAssertNs = {
      ...Assertions,
      eventPresent: (module: string, method: string, events?: EventRecord[]) =>
        assertEventPresent(this._api, module, method, events),
      eventMany: (module: string, method: string, events?: EventRecord[]) =>
        assertEventMany(this._api, module, method, events),
      extrinsicPresent: (options: AssertExtrinsicOptions) =>
        assertExtrinsicPresent(this._api, options)
    };

    const remappedWaitsNs = {
      ...Waits,
      bspVolunteer: () => waitForBspVolunteer(this._api),
      bspStored: () => waitForBspStored(this._api)
    };

    const remappedFileNs = {
      ...Files,
      /**
       * Creates a new bucket.
       *
       * @param bucketName - The name of the bucket to be created.
       * @returns A promise that resolves to a new bucket event.
       */
      newBucket: (bucketName: string) => createBucket(this._api, bucketName),
      /**
       * Creates a new bucket and submits a new storage request.
       *
       * @param source - The local path to the file to be uploaded.
       * @param location - The StorageHub "location" field of the file to be uploaded.
       * @param bucketName - The name of the bucket to be created.
       * @returns A promise that resolves to file metadata.
       */
      newStorageRequest: (source: string, location: string, bucketName: string) =>
        sendNewStorageRequest(this._api, source, location, bucketName)
    };

    return Object.assign(this._api, {
      sealBlock: this.sealBlock.bind(this),
      sendNewStorageRequest: this.sendNewStorageRequest.bind(this),
      createBucket: this.createBucket.bind(this),
      assertEvent: this.assertEvent.bind(this),
      assert: remappedAssertNs,
      wait: remappedWaitsNs,
      file: remappedFileNs
      // Add docker
    }) satisfies BspNetApi;
  }

  async [Symbol.asyncDispose]() {
    await this._api.disconnect();
  }
}

export type EnrichedBspApi = Awaited<ReturnType<typeof BspNetTestApi.create>>;
