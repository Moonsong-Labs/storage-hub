import "@storagehub/api-augment";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { types as BundledTypes } from "@storagehub/types-bundle";
import { sealBlock, sendNewStorageRequest, createBucket } from "./helpers";
import type { BspNetApi } from "./types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import { assertEventPresent, Assertions } from "../asserts";
import type { EventRecord } from "@polkadot/types/interfaces";

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
    return Object.assign(this._api, {
      sealBlock: this.sealBlock.bind(this),
      sendNewStorageRequest: this.sendNewStorageRequest.bind(this),
      createBucket: this.createBucket.bind(this),
      assertEvent: this.assertEvent.bind(this),
      assert: Assertions
    }) satisfies BspNetApi;
  }

  async [Symbol.asyncDispose]() {
    await this._api.disconnect();
  }

  // TODO: Add namespaces
  //      - assert
  //      - files
  //      - wait
}
