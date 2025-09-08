import { shParachainDefinitions } from "@storagehub/types-bundle";
export default {
    types: shParachainDefinitions.types?.[0].types,
    runtime: shParachainDefinitions.runtime,
    rpc: shParachainDefinitions.rpc?.storagehubclient
};
//# sourceMappingURL=definitions.js.map