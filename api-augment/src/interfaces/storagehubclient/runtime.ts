import type { DefinitionCall, DefinitionsCall } from "@polkadot/types/types";

const STORAGE_PROVIDERS_V1: Record<string, DefinitionCall> = {
  get_bsp_info: {
    description: "Get the BSP info for a given BSP ID.",
    params: [
      {
        name: "bspId",
        type: "BackupStorageProviderId"
      }
    ],
    type: "Result<BackupStorageProvider, GetBspInfoError>"
  }
};

export const runtime: DefinitionsCall = {
  StorageProvidersApi: [
    {
      methods: STORAGE_PROVIDERS_V1,
      version: 1
    }
  ]
};
