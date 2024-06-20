import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { CreatedBlock } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";

export type BspNetApi = ApiPromise & {
  sealBlock: (
    call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
    signer?: KeyringPair,
  ) => Promise<CreatedBlock>;
};
