import type { DefinitionCall, DefinitionsCall } from "@polkadot/types/types";

const FILE_SYSTEM_V1: Record<string, DefinitionCall> = {
  query_earliest_file_volunteer_block: {
    description: "Query the earliest block number that a BSP can volunteer for a file.",
    params: [
      {
        name: "bspId",
        type: "BackupStorageProviderId"
      },
      {
        name: "fileKey",
        type: "H256"
      }
    ],
    type: "Result<BlockNumber, QueryFileEarliestVolunteerBlockError>"
  },
  query_bsp_confirm_chunks_to_prove_for_file: {
    description: "Query the chunks that a BSP needs to prove to confirm that it is storing a file.",
    params: [
      {
        name: "bspId",
        type: "BackupStorageProviderId"
      },
      {
        name: "fileKey",
        type: "H256"
      }
    ],
    type: "Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>"
  }
};

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
  ],
  FileSystemApi: [
    {
      methods: FILE_SYSTEM_V1,
      version: 1
    }
  ]
};
