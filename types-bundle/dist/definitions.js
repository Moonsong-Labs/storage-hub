import { rpcDefinitions } from "./rpc.js";
import { runtime } from "./runtime.js";
import { ALL_TYPES } from "./types.js";
export const storageHubDefinitions = {
  rpc: rpcDefinitions,
  runtime,
  types: [
    {
      minmax: [0, undefined],
      types: ALL_TYPES
    }
  ]
};
export const types = {
  spec: {
    "sh-parachain-runtime": storageHubDefinitions
  }
};
//# sourceMappingURL=definitions.js.map
