import { storageHubDefinitions } from "@storagehub/types-bundle";
export default {
  types: storageHubDefinitions.types?.[0].types,
  runtime: storageHubDefinitions.runtime,
  rpc: storageHubDefinitions.rpc?.storagehubclient
};
