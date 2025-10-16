import { rpcDefinitions } from "./rpc.js";
import { runtime } from "./runtime.js";
import { PARACHAIN_TYPES, SOLOCHAIN_EVM_TYPES } from "./types.js";
export const shParachainDefinitions = {
    rpc: rpcDefinitions,
    runtime,
    types: [
        {
            minmax: [0, undefined],
            types: PARACHAIN_TYPES
        }
    ]
};
export const shSolochainEvmDefinitions = {
    rpc: rpcDefinitions,
    runtime,
    types: [
        {
            minmax: [0, undefined],
            types: SOLOCHAIN_EVM_TYPES
        }
    ]
};
export const types = {
    spec: {
        "sh-parachain-runtime": shParachainDefinitions,
        shParachainDefinitions,
        "sh-solochain-evm": shSolochainEvmDefinitions
    }
};
//# sourceMappingURL=definitions.js.map