import "@storagehub/api-augment";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import { alice } from "./pjsKeyring";
import type { EventRecord } from "@polkadot/types/interfaces";
import { strictEqual } from "node:assert";

export const sendTransaction = async (
  call: SubmittableExtrinsic<"promise", ISubmittableResult>,
  options?: {
    nonce?: number;
    signer?: KeyringPair;
    waitFor?: "Finalized" | "InBlock";
  },
) => {
  return new Promise(async (resolve, reject) => {
    const trigger = options?.waitFor || "InBlock";

    const unsub = await call.signAndSend(
      options?.signer || alice,
      { nonce: options?.nonce || -1 },
      (result) => {
        switch (result.status.type) {
          case trigger: {
            unsub();
            resolve(result);
            break;
          }

          case "Dropped": {
            unsub();
            reject("Transaction dropped");
            break;
          }

          // case "Invalid": {
          //   unsub()
          //   reject("Invalid transaction")
          //   break
          // }

          case "Usurped": {
            unsub();
            reject("Transaction was usurped");
            break;
          }
        }
      },
    );
  });
};

