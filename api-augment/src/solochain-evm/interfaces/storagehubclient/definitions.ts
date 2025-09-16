import { shSolochainEvmDefinitions } from "@storagehub/types-bundle";

export default {
  types: shSolochainEvmDefinitions.types?.[0].types,
  runtime: shSolochainEvmDefinitions.runtime,
  rpc: shSolochainEvmDefinitions.rpc?.storagehubclient
};
