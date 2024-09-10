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

const PROOFS_DEALER_V1: Record<string, DefinitionCall> = {
  get_last_tick_provider_submitted_proof: {
    description: "Get the last tick for which the submitter submitted a proof.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Result<BlockNumber, GetLastTickProviderSubmittedProofError>"
  },
  get_last_checkpoint_challenge_tick: {
    description: "Get the last checkpoint challenge tick.",
    params: [],
    type: "BlockNumber"
  },
  get_checkpoint_challenges: {
    description: "Get checkpoint challenges for a given block.",
    params: [
      {
        name: "tick",
        type: "BlockNumber"
      }
    ],
    type: "Result<Vec<(Key, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError>"
  },
  get_challenge_seed: {
    description: "Get the seed for a given challenge tick.",
    params: [
      {
        name: "tick",
        type: "BlockNumber"
      }
    ],
    type: "Result<RandomnessOutput, GetChallengeSeedError>"
  },
  get_challenge_period: {
    description: "Get the challenge period for a given Provider.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Result<BlockNumber, GetChallengePeriodError>"
  },
  get_checkpoint_challenge_period: {
    description: "Get the checkpoint challenge period.",
    params: [],
    type: "BlockNumber"
  },
  get_challenges_from_seed: {
    description: "Get challenges from a seed.",
    params: [
      {
        name: "seed",
        type: "RandomnessOutput"
      },
      {
        name: "providerId",
        type: "ProviderId"
      },
      {
        name: "count",
        type: "u32"
      }
    ],
    type: "Vec<Key>"
  },
  get_forest_challenges_from_seed: {
    description: "Get forest challenges from a seed.",
    params: [
      {
        name: "seed",
        type: "RandomnessOutput"
      },
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Vec<Key>"
  },
  get_current_tick: {
    description: "Get the current tick.",
    params: [],
    type: "BlockNumber"
  },
  get_next_deadline_tick: {
    description: "Get the next deadline tick.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Result<BlockNumber, GetNextDeadlineTickError>"
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
  },
  get_storage_provider_id: {
    description: "Get the Storage Provider ID for a given Account ID.",
    params: [
      {
        name: "who",
        type: "AccountId"
      }
    ],
    type: "Option<StorageProviderId>"
  },
  query_storage_provider_capacity: {
    description: "Query the storage provider capacity.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Result<StorageDataUnit, QueryStorageProviderCapacityError>"
  },
  query_available_storage_capacity: {
    description: "Query the available storage capacity.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Result<StorageDataUnit, QueryAvailableStorageCapacityError>"
  },
  query_earliest_change_capacity_block: {
    description: "Query the earliest block number that a BSP can change its capacity.",
    params: [
      {
        name: "providerId",
        type: "BackupStorageProviderId"
      }
    ],
    type: "Result<BlockNumber, QueryEarliestChangeCapacityBlockError>"
  }
};

const PAYMENT_STREAMS_V1: Record<string, DefinitionCall> = {
  get_users_with_debt_over_threshold: {
    description: "Get the users that have a debt to the provider greater than the threshold.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      },
      {
        name: "threshold",
        type: "Balance"
      }
    ],
    type: "Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError>"
  },
  get_users_of_payment_streams_of_provider: {
    description: "Get the payment streams of a provider.",
    params: [
      {
        name: "providerId",
        type: "ProviderId"
      }
    ],
    type: "Vec<AccountId>"
  }
};

export const runtime: DefinitionsCall = {
  FileSystemApi: [
    {
      methods: FILE_SYSTEM_V1,
      version: 1
    }
  ],
  ProofsDealerApi: [
    {
      methods: PROOFS_DEALER_V1,
      version: 1
    }
  ],
  StorageProvidersApi: [
    {
      methods: STORAGE_PROVIDERS_V1,
      version: 1
    }
  ],
  PaymentStreamsApi: [
    {
      methods: PAYMENT_STREAMS_V1,
      version: 1
    }
  ]
};
