import type { OverrideBundleDefinition, OverrideBundleType } from "@polkadot/types/types";
import { rpcDefinitions } from "./rpc.js";
import { runtime } from "./runtime.js";
import { ALL_TYPES } from "./types.js";

export const storageHubDefinitions: OverrideBundleDefinition = {
  rpc: rpcDefinitions,
  runtime,
  types: [
    {
      minmax: [0, undefined],
      types: ALL_TYPES
    }
  ]
};

export const types: OverrideBundleType = {
  spec: {
    "sh-parachain-runtime": storageHubDefinitions
  }
};
