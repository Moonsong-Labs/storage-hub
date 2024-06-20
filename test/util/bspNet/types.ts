import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { Codec, IEventData, ISubmittableResult } from "@polkadot/types/types";
import type { FileSendResponse, SealedBlock } from "./helpers";
import type { EventRecord,Event } from "@polkadot/types/interfaces";

export type BspNetApi = ApiPromise & {
  sealBlock: (
    call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
    signer?: KeyringPair,
  ) => Promise<SealedBlock>;

  sendFile: (
    localPath: string,
    remotePath: string,
    addressId: string,
  ) => Promise<FileSendResponse>;

  assertEvent: (
    module: string,
    method: string,
    events?: EventRecord[],
  ) => {event: Event, data: Codec[] & IEventData};
  
  // TODO: add a fetchEvent function which infers type from the module and method
  // fetchEvent: () => void;
};
