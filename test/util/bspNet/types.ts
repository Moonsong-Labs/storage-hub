import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { Codec, IEventData, ISubmittableResult } from "@polkadot/types/types";
import type { FileSendResponse, SealedBlock } from "./helpers";
import type { EventRecord, Event } from "@polkadot/types/interfaces";

/**
 * Represents an enhanced API for interacting with StorageHub BSPNet.
 */
export type BspNetApi = ApiPromise & {
  /**
   * Seals a block optionally with a given extrinsic and signer.
   *
   * @param call - The extrinsic to be included in the block.
   * @param signer - The keyring pair used to sign the block.
   * @returns A promise that resolves to a sealed block.
   */
  sealBlock: (
    call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
    signer?: KeyringPair
  ) => Promise<SealedBlock>;

  /**
   * @description Issues a sendFile RPC call to UserNode.
   * N.B. Local file must exist in mounted volume '/res'
   *
   * @param localPath - The local file path.
   * @param remotePath - The destination path on the blockchain.
   * @param addressId - The address ID associated with the file transfer.
   * @returns A promise that resolves to a file send response.
   */
  sendFile: (localPath: string, remotePath: string, addressId: string, bucketId: string) => Promise<FileSendResponse>;

  /**
   * @description Asserts that a specific event occurred in a list of events.
   *
   * @param module - The module where the event originated.
   * @param method - The method that triggered the event.
   * @param events - The list of event records to search through.
   * @returns An object containing the event and its data.
   */
  assertEvent: (
    module: string,
    method: string,
    events?: EventRecord[]
  ) => { event: Event; data: Codec[] & IEventData };

  /**
   * @description Fetches an event, inferring its type from the module and method.
   *
   * @remarks
   * This function needs to be implemented.
   */
  // fetchEvent: () => void;
};
