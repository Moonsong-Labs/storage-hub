import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { Codec, IEventData, ISubmittableResult } from "@polkadot/types/types";
import type { SealedBlock } from "./helpers";
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
    calls?:
      | SubmittableExtrinsic<"promise", ISubmittableResult>
      | SubmittableExtrinsic<"promise", ISubmittableResult>[],
    signer?: KeyringPair
  ) => Promise<SealedBlock>;

  /**
   * @description Advances the block number to the given block number.
   *
   * @param blockNumber - The block number to advance to.
   * @param waitBetweenBlocks - Whether to wait between blocks. Defaults to false. Can also be set to a number to wait that many milliseconds between blocks.
   * @returns A promise that resolves when the block number is advanced.
   */
  advanceToBlock: (
    blockNumber: number,
    waitBetweenBlocks?: number | boolean
  ) => Promise<SealedBlock>;

  /**
   * @description Creates a new bucket and submits a new storage request.
   *
   * @param source - The local path to the file to be uploaded.
   * @param location - The StorageHub "location" field of the file to be uploaded.
   * @param bucketName - The name of the bucket to be created.
   * @returns
   */
  sendNewStorageRequest: (
    source: string,
    location: string,
    bucketName: string
  ) => Promise<FileMetadata>;

  /**
   * @description Creates a new bucket.
   *
   * @param bucketName - The name of the bucket to be created.
   * @returns A promise that resolves to a new bucket event.
   */
  createBucket: (bucketName: string) => Promise<Event>;

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
 * @property {string} fileKey - The StorageHub file key of the file.
 * @property {string} bucketId - The StorageHub bucket ID of the file.
 * @property {string} location - The StorageHub location of the file.
 * @property {string} owner - The StorageHub owner of the file.
 * @property {string} fingerprint - The StorageHub fingerprint of the file.
 * @property {number} fileSize - The size of the file in bytes.
 */
export interface FileMetadata {
  fileKey: string;
  bucketId: string;
  location: string;
  owner: string;
  fingerprint: string;
  fileSize: number;
}
