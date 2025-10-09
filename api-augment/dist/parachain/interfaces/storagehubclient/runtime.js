const FILE_SYSTEM_V1 = {
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
const PROOFS_DEALER_V1 = {
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
const STORAGE_PROVIDERS_V1 = {
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
export const runtime = {
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
    ]
};
//# sourceMappingURL=runtime.js.map