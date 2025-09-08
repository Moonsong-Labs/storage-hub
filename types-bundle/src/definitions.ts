import type { OverrideBundleDefinition, OverrideBundleType } from "@polkadot/types/types";
import { rpcDefinitions } from "./rpc.js";
import { runtime } from "./runtime.js";
import { PARACHAIN_TYPES, SOLOCHAIN_EVM_TYPES } from "./types.js";

export const shParachainDefinitions: OverrideBundleDefinition = {
  rpc: rpcDefinitions,
  runtime,
  types: [
    {
      minmax: [0, undefined],
      types: PARACHAIN_TYPES
    }
  ]
};

export const shSolochainEvmDefinitions: OverrideBundleDefinition = {
  rpc: rpcDefinitions,
  runtime,
  types: [
    {
      minmax: [0, undefined],
      types: SOLOCHAIN_EVM_TYPES
    }
  ]
};

export const types: OverrideBundleType = {
  spec: {
    "sh-parachain-runtime": shParachainDefinitions,
    "datahaven-stagenet": shSolochainEvmDefinitions
  }
};
