export const ALL_TYPES = {
  FileMetadata: {
    owner: "Vec<u8>",
    bucket_id: "Vec<u8>",
    location: "Vec<u8>",
    file_size: "u64",
    fingerprint: "[u8; 32]"
  },
  LoadFileInStorageResult: {
    file_key: "H256",
    file_metadata: "FileMetadata"
  },
  IncompleteFileStatus: {
    file_metadata: "FileMetadata",
    stored_chunks: "u64",
    total_chunks: "u64"
  },
  SaveFileToDisk: {
    _enum: {
      FileNotFound: null,
      Success: "FileMetadata",
      IncompleteFile: "IncompleteFileStatus"
    }
  },
  GetFileFromFileStorageResult: {
    _enum: {
      FileNotFound: null,
      FileFound: "FileMetadata",
      IncompleteFile: "IncompleteFileStatus",
      FileFoundWithInconsistency: "FileMetadata"
    }
  },
  ProviderId: "H256",
  Key: "H256",
  RandomnessOutput: "H256",
  TrieRemoveMutation: "Null",
  CheckpointChallenge: {
    file_key: "H256",
    should_remove_file: "bool"
  },
  ShouldRemoveFile: "bool",
  BackupStorageProviderId: "H256",
  MainStorageProviderId: "H256",
  BucketId: "H256",
  StorageData: "u64",
  MerklePatriciaRoot: "H256",
  ChunkId: "u64",
  StorageDataUnit: "u64",
  Multiaddresses: "Vec<Bytes>",
  ReputationWeightType: "u32",
  ValuePropId: "H256",
  ValueProposition: {
    price_per_giga_unit_of_data_per_block: "u64",
    bucket_data_limit: "StorageDataUnit"
  },
  ValuePropositionWithId: {
    id: "ValuePropId",
    value_prop: "ValueProposition"
  },
  BackupStorageProvider: {
    capacity: "StorageDataUnit",
    capacity_used: "StorageDataUnit",
    multiaddresses: "Multiaddresses",
    root: "MerklePatriciaRoot",
    last_capacity_change: "BlockNumber",
    owner_account: "AccountId",
    payment_account: "AccountId",
    reputation_weight: "ReputationWeightType",
    sign_up_block: "BlockNumber"
  },
  StorageProviderId: {
    _enum: {
      BackupStorageProvider: "BackupStorageProviderId",
      MainStorageProvider: "MainStorageProviderId"
    }
  },
  GetProofSubmissionRecordError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderNeverSubmittedProof: null,
      InternalApiError: null
    }
  },
  GetCheckpointChallengesError: {
    _enum: {
      TickGreaterThanLastCheckpointTick: null,
      NoCheckpointChallengesInTick: null,
      InternalApiError: null
    }
  },
  GetChallengeSeedError: {
    _enum: {
      TickBeyondLastSeedStored: null,
      TickIsInTheFuture: null,
      InternalApiError: null
    }
  },
  GetChallengePeriodError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  GetBspInfoError: {
    _enum: {
      BspNotRegistered: null,
      InternalApiError: null
    }
  },
  GetNextDeadlineTickError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderNotInitialised: null,
      ArithmeticOverflow: null,
      InternalApiError: null
    }
  },
  IsStorageRequestOpenToVolunteersError: {
    _enum: {
      StorageRequestNotFound: null,
      InternalApiError: null
    }
  },
  QueryFileEarliestVolunteerBlockError: {
    _enum: {
      FailedToEncodeFingerprint: null,
      FailedToEncodeBsp: null,
      ThresholdArithmeticError: null,
      StorageRequestNotFound: null,
      InternalError: null
    }
  },
  QueryBspConfirmChunksToProveForFileError: {
    _enum: {
      StorageRequestNotFound: null,
      ConfirmChunks: "QueryConfirmChunksToProveForFileError",
      InternalError: null
    }
  },
  QueryMspConfirmChunksToProveForFileError: {
    _enum: {
      StorageRequestNotFound: null,
      ConfirmChunks: "QueryConfirmChunksToProveForFileError",
      InternalError: null
    }
  },
  GenericApplyDeltaEventInfoError: {
    _enum: {
      DecodeError: null
    }
  },
  QueryProviderMultiaddressesError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  GetStakeError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalError: null
    }
  },
  QueryBucketsForMspError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalError: null
    }
  },
  QueryConfirmChunksToProveForFileError: {
    _enum: {
      ChallengedChunkToChunkIdError: null
    }
  },
  GetUsersWithDebtOverThresholdError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderWithoutPaymentStreams: null,
      AmountToChargeOverflow: null,
      AmountToChargeUnderflow: null,
      DebtOverflow: null,
      InternalApiError: null
    }
  },
  QueryStorageProviderCapacityError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  QueryAvailableStorageCapacityError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  QueryEarliestChangeCapacityBlockError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  QueryMspIdOfBucketIdError: {
    _enum: {
      BucketNotFound: null,
      InternalApiError: null
    }
  }
};
//# sourceMappingURL=types.js.map
