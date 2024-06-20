import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { FileSendResponse, SealedBlock } from "./helpers";
import type { EventRecord } from "@polkadot/types/interfaces";
import type { Event } from "@polkadot/types/interfaces";

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
  ) => Event;
};
