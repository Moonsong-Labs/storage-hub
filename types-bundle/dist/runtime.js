const FILE_SYSTEM_V1 = {
    is_storage_request_open_to_volunteers: {
        description: "Check if a storage request is open to volunteers.",
        params: [
            {
                name: "fileKey",
                type: "H256"
            }
        ],
        type: "Result<bool, IsStorageRequestOpenToVolunteersError>"
    },
    query_earliest_file_volunteer_tick: {
        description: "Query the earliest tick number that a BSP can volunteer for a file.",
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
    },
    query_msp_confirm_chunks_to_prove_for_file: {
        description: "Query the chunks that a MSP needs to prove to confirm that it is storing a file.",
        params: [
            {
                name: "mspId",
                type: "MainStorageProviderId"
            },
            {
                name: "fileKey",
                type: "H256"
            }
        ],
        type: "Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>"
    },
    query_bsps_volunteered_for_file: {
        description: "Query the BSPs that volunteered for a file.",
        params: [
            {
                name: "fileKey",
                type: "H256"
            }
        ],
        type: "Result<Vec<BackupStorageProviderId>, QueryBspsVolunteeredForFileError>"
    },
    decode_generic_apply_delta_event_info: {
        description: "Decodes the BucketId expected to be found in the event info of a generic apply delta.",
        params: [
            {
                name: "encodedEventInfo",
                type: "Vec<u8>"
            }
        ],
        type: "Result<BucketId, GenericApplyDeltaEventInfoError>"
    },
    pending_storage_requests_by_msp: {
        description: "Get pending storage requests for a Main Storage Provider.",
        params: [
            {
                name: "mspId",
                type: "MainStorageProviderId"
            }
        ],
        type: "BTreeMap<H256, StorageRequestMetadata>"
    },
    storage_requests_by_msp: {
        description: "Get the storage requests for a given MSP.",
        params: [
            {
                name: "mspId",
                type: "MainStorageProviderId"
            }
        ],
        type: "BTreeMap<H256, StorageRequestMetadata>"
    },
    query_incomplete_storage_request_metadata: {
        description: "Query incomplete storage request metadata for a file key.",
        params: [
            {
                name: "fileKey",
                type: "H256"
            }
        ],
        type: "Result<IncompleteStorageRequestMetadataResponse, QueryIncompleteStorageRequestMetadataError>"
    },
    list_incomplete_storage_request_keys: {
        description: "List incomplete storage request keys with pagination.",
        params: [
            {
                name: "startAfter",
                type: "Option<H256>"
            },
            {
                name: "limit",
                type: "u32"
            }
        ],
        type: "Vec<H256>"
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
        type: "Result<BlockNumber, GetProofSubmissionRecordError>"
    },
    get_next_tick_to_submit_proof_for: {
        description: "Get the next tick for which the submitter should submit a proof.",
        params: [
            {
                name: "providerId",
                type: "ProviderId"
            }
        ],
        type: "Result<BlockNumber, GetProofSubmissionRecordError>"
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
    get_worst_case_scenario_slashable_amount: {
        description: "Get the worst case scenario slashable amount for a provider.",
        params: [
            {
                name: "providerId",
                type: "ProviderId"
            }
        ],
        type: "Option<Balance>"
    },
    get_slash_amount_per_max_file_size: {
        description: "Get the slashable amount corresponding to the configured max file size.",
        params: [],
        type: "Balance"
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
    },
    query_msp_id_of_bucket_id: {
        description: "Query the MSP ID of a bucket ID.",
        params: [
            {
                name: "bucketId",
                type: "H256"
            }
        ],
        type: "Result<ProviderId, QueryMspIdOfBucketIdError>"
    },
    query_provider_multiaddresses: {
        description: "Query the provider's multiaddresses.",
        params: [
            {
                name: "providerId",
                type: "ProviderId"
            }
        ],
        type: "Result<Multiaddresses, QueryProviderMultiaddressesError>"
    },
    query_value_propositions_for_msp: {
        description: "Query the value propositions for a MSP.",
        params: [
            {
                name: "mspId",
                type: "MainStorageProviderId"
            }
        ],
        type: "Vec<ValuePropositionWithId>"
    },
    get_bsp_stake: {
        description: "Get the stake of a BSP.",
        params: [
            {
                name: "bspId",
                type: "BackupStorageProviderId"
            }
        ],
        type: "Result<Balance, GetStakeError>"
    },
    can_delete_provider: {
        description: "Check if a provider can be deleted.",
        params: [
            {
                name: "providerId",
                type: "ProviderId"
            }
        ],
        type: "bool"
    },
    query_buckets_for_msp: {
        description: "Get the Buckets that an MSP is storing.",
        params: [
            {
                name: "mspId",
                type: "MainStorageProviderId"
            }
        ],
        type: "Result<Vec<BucketId>, QueryBucketsForMspError>"
    },
    query_buckets_of_user_stored_by_msp: {
        description: "Query the buckets stored by an MSP that belong to a specific user.",
        params: [
            {
                name: "mspId",
                type: "ProviderId"
            },
            {
                name: "user",
                type: "AccountId"
            }
        ],
        type: "Result<Vec<H256>, QueryBucketsOfUserStoredByMspError>"
    }
};
const PAYMENT_STREAMS_V1 = {
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
    },
    get_providers_with_payment_streams_with_user: {
        description: "Get the Providers that have at least one payment stream with a specific user.",
        params: [
            {
                name: "userAccount",
                type: "AccountId"
            }
        ],
        type: "Vec<ProviderId>"
    },
    get_current_price_per_giga_unit_per_tick: {
        description: "Get the current price per giga unit per tick",
        params: [],
        type: "Balance"
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
    ],
    PaymentStreamsApi: [
        {
            methods: PAYMENT_STREAMS_V1,
            version: 1
        }
    ]
};
//# sourceMappingURL=runtime.js.map